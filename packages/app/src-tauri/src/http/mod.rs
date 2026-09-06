//! The localhost capture listener: the axum router, and starting it on
//! `127.0.0.1:<port>` (design D5).

pub mod captures;
pub mod origin;
pub mod status;

use axum::Json;
use axum::extract::DefaultBodyLimit;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};

use crate::error::{AppError, Result};
use crate::library::{Library, SharedLibrary};
use crate::model::ListenerStatus;

/// The largest request body the listener will read. A capture is one image off
/// a page; anything past this is a mistake or a local process trying to grow the
/// app's heap, and the body layer refuses it instead of buffering it.
pub const MAX_BODY_BYTES: usize = 64 * 1024 * 1024;

/// What the handlers need. The library is shared with the Tauri commands, so a
/// capture and a UI action never see different databases.
#[derive(Clone)]
pub struct HttpState {
    pub library: SharedLibrary,
    pub version: String,
}

pub fn router(state: HttpState) -> axum::Router {
    axum::Router::new()
        .route("/captures", post(captures::post_capture))
        .route("/status", get(status::get_status))
        .layer(DefaultBodyLimit::max(MAX_BODY_BYTES))
        // Applied last, so it wraps everything above: a request the origin check
        // refuses is answered before any handler or body layer reads its body.
        .layer(axum::middleware::from_fn(origin::require_extension_origin))
        .with_state(state)
}

/// Bind and serve. A failure to bind is returned, not raised: the app still
/// opens and settings show the reason (spec `capture-ingest`).
pub async fn start(state: HttpState, port: u16) -> ListenerStatus {
    // `LOCALHOST`, never `UNSPECIFIED`: the origin header is the only guard this
    // listener has, and it is worth nothing against a caller on the LAN.
    let bound = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port)).await;
    let listener = match bound {
        Ok(listener) => listener,
        Err(error) => {
            return ListenerStatus {
                running: false,
                port,
                error: Some(error.to_string()),
            };
        }
    };

    tokio::spawn(async move {
        // The accept loop ends with the process. Nothing here can act on a
        // failure, and the app has to stay up when the listener goes down.
        let _ = axum::serve(listener, router(state)).await;
    });

    ListenerStatus {
        running: true,
        port,
        error: None,
    }
}

/// The one place an `AppError` becomes a status code — the mapping `error.rs`
/// documents. Split it and the two drift.
pub fn error_response(error: &AppError) -> Response {
    let status = match error {
        AppError::BadRequest(_) => StatusCode::BAD_REQUEST,
        AppError::Decode(_) => StatusCode::UNPROCESSABLE_ENTITY,
        AppError::NoLibrary => StatusCode::SERVICE_UNAVAILABLE,
        AppError::NotFound(_) => StatusCode::NOT_FOUND,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    };
    reason(status, error.to_string())
}

/// Every refusal answers in one shape, so a client reads one field to learn why.
pub fn reason(status: StatusCode, message: impl Into<String>) -> Response {
    (status, Json(serde_json::json!({ "error": message.into() }))).into_response()
}

/// Borrow the open library, or fail with `NoLibrary`. `work` is a plain closure
/// on purpose: `SharedLibrary` is a `std::sync::Mutex`, so nothing holding this
/// lock may `.await` (see `captures::store`).
pub fn with_library<T>(
    library: &SharedLibrary,
    work: impl FnOnce(&Library) -> Result<T>,
) -> Result<T> {
    // A poisoned mutex means some earlier request panicked while holding the
    // library. The connection survives that (an open transaction rolls back when
    // it drops), and refusing every later capture would be the larger outage.
    let guard = library
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    work(guard.as_ref().ok_or(AppError::NoLibrary)?)
}

/// Fixtures shared by the `http` test modules: a router-driving `send`, a temp
/// library, and the multipart bodies the spec's scenarios describe.
#[cfg(test)]
pub mod test_support {
    use axum::body::Body;
    use axum::http::header::{CONTENT_TYPE, ORIGIN};
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    use super::{HttpState, router, with_library};
    use crate::library::{Library, SharedLibrary};

    pub const EXTENSION_ORIGIN: &str = "chrome-extension://abcdefghijklmnopabcdefghijklmnop";
    const BOUNDARY: &str = "boorubox-test-boundary";

    pub fn empty_state() -> HttpState {
        HttpState {
            library: SharedLibrary::default(),
            version: "1.2.3".to_string(),
        }
    }

    /// A state with a library open in a temp folder. The `TempDir` comes back
    /// with it: dropping it deletes the library.
    pub fn open_state() -> (tempfile::TempDir, HttpState) {
        let dir = tempfile::tempdir().unwrap();
        let library = Library::open_or_create(dir.path()).unwrap();
        let state = empty_state();
        *state.library.lock().unwrap() = Some(library);
        (dir, state)
    }

    pub fn image_count(state: &HttpState) -> i64 {
        with_library(&state.library, |library| library.image_count()).unwrap()
    }

    pub fn stored_file_exists(state: &HttpState, id: &str, ext: &str) -> bool {
        with_library(&state.library, |library| {
            Ok(library.image_path(id, ext).is_file())
        })
        .unwrap()
    }

    pub fn png_bytes(width: u32, height: u32) -> Vec<u8> {
        let image = image::DynamicImage::ImageRgba8(image::RgbaImage::new(width, height));
        let mut out = std::io::Cursor::new(Vec::new());
        image.write_to(&mut out, image::ImageFormat::Png).unwrap();
        out.into_inner()
    }

    pub struct Part<'a> {
        pub name: &'a str,
        pub filename: Option<&'a str>,
        pub body: &'a [u8],
    }

    pub fn file_part(bytes: &[u8]) -> Part<'_> {
        Part {
            name: "file",
            filename: Some("capture.png"),
            body: bytes,
        }
    }

    pub fn meta_part(json: &str) -> Part<'_> {
        Part {
            name: "meta",
            filename: None,
            body: json.as_bytes(),
        }
    }

    /// A `CaptureMeta` as the bridge extension sends it, adapter record included.
    pub fn meta_json(id: &str) -> String {
        serde_json::json!({
            "id": id,
            "imageUrl": "https://example.test/i.png",
            "pageUrl": "https://example.test/p",
            "pageTitle": "a page",
            "capturedAt": 1_700_000_000_000i64,
            "adapter": { "site": "danbooru", "fields": { "tags": ["1girl"] } },
        })
        .to_string()
    }

    pub fn capture_request(origin: Option<&str>, parts: &[Part<'_>]) -> Request<Body> {
        let mut body = Vec::new();
        for part in parts {
            body.extend_from_slice(
                format!(
                    "--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"{}\"",
                    part.name
                )
                .as_bytes(),
            );
            if let Some(filename) = part.filename {
                body.extend_from_slice(format!("; filename=\"{filename}\"").as_bytes());
            }
            body.extend_from_slice(b"\r\n\r\n");
            body.extend_from_slice(part.body);
            body.extend_from_slice(b"\r\n");
        }
        body.extend_from_slice(format!("--{BOUNDARY}--\r\n").as_bytes());

        let mut request = Request::post("/captures").header(
            CONTENT_TYPE,
            format!("multipart/form-data; boundary={BOUNDARY}"),
        );
        if let Some(origin) = origin {
            request = request.header(ORIGIN, origin);
        }
        request.body(Body::from(body)).unwrap()
    }

    pub fn get_request(uri: &str, origin: Option<&str>) -> Request<Body> {
        let mut request = Request::get(uri);
        if let Some(origin) = origin {
            request = request.header(ORIGIN, origin);
        }
        request.body(Body::empty()).unwrap()
    }

    /// Drive one request through a fresh router over `state`, and read the JSON
    /// body back. Every answer this listener gives is JSON.
    pub async fn send(
        state: &HttpState,
        request: Request<Body>,
    ) -> (StatusCode, serde_json::Value) {
        let response = router(state.clone()).oneshot(request).await.unwrap();
        let status = response.status();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let body = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
        (status, body)
    }
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
    use std::time::Duration;

    use super::test_support::*;
    use super::*;

    fn free_port() -> u16 {
        TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
            .unwrap()
            .local_addr()
            .unwrap()
            .port()
    }

    fn ask_for_status(port: u16) -> String {
        let mut stream = TcpStream::connect((Ipv4Addr::LOCALHOST, port)).unwrap();
        stream
            .write_all(b"GET /status HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n")
            .unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        response
    }

    /// The machine's own address on the network, when it has one. Connecting a
    /// UDP socket sends no packet; it only asks the routing table which local
    /// address would be used, so this works offline and answers `None` when
    /// there is no route to ask about.
    fn routable_address() -> Option<Ipv4Addr> {
        let socket = std::net::UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)).ok()?;
        socket.connect(("192.0.2.1", 80)).ok()?;
        match socket.local_addr().ok()?.ip() {
            std::net::IpAddr::V4(address) if !address.is_loopback() => Some(address),
            _ => None,
        }
    }

    /// Multi-threaded on purpose: the blocking socket reads below would starve a
    /// current-thread runtime that also owes the spawned server a turn.
    #[tokio::test(flavor = "multi_thread")]
    async fn a_started_listener_answers_on_loopback_and_nowhere_else() {
        let port = free_port();

        let status = start(empty_state(), port).await;

        assert!(status.running, "start failed: {:?}", status.error);
        assert_eq!(status.port, port);
        assert_eq!(status.error, None);
        assert!(
            ask_for_status(port).starts_with("HTTP/1.1 503"),
            "the loopback address must accept connections"
        );

        if let Some(routable) = routable_address() {
            let elsewhere = SocketAddr::from((routable, port));
            assert!(
                TcpStream::connect_timeout(&elsewhere, Duration::from_secs(2)).is_err(),
                "the listener answered on {routable}; it must bind loopback only"
            );
        }
    }

    #[tokio::test]
    async fn a_taken_port_is_reported_with_its_reason() {
        let taken = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let port = taken.local_addr().unwrap().port();

        let status = start(empty_state(), port).await;

        assert!(!status.running);
        assert_eq!(status.port, port);
        assert!(
            status.error.is_some(),
            "settings show the reason, so it must be carried"
        );
    }
}
