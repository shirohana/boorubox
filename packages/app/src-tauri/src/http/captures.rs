//! `POST /captures`: the multipart handler that turns one file part and one
//! JSON part into a call to `ingest::store_image`.

use axum::Json;
use axum::extract::multipart::MultipartError;
use axum::extract::{Multipart, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

use crate::error::Result;
use crate::http::{HttpState, error_response, reason, with_library};
use crate::ingest::{self, IngestInput, Ingested};
use crate::library::SharedLibrary;
use crate::model::{CaptureMeta, ImageSource};

/// Design D15 pins these two names; the bridge extension sends exactly them, and
/// any other name is a 400 rather than a capture that silently never arrives.
const FILE_FIELD: &str = "file";
const META_FIELD: &str = "meta";

/// Why a body was refused, carried as data rather than a built `Response` so the
/// reading code stays one small `Result`.
type Refusal = (StatusCode, String);

pub async fn post_capture(State(state): State<HttpState>, multipart: Multipart) -> Response {
    let capture = match read_parts(multipart).await {
        Ok(capture) => capture,
        Err((status, message)) => return reason(status, message),
    };
    // Awaited, so nothing answers before the record exists (spec
    // `capture-ingest`: "A response earlier than the stored record SHALL NOT be
    // sent").
    store(state, capture).await
}

struct Capture {
    bytes: Vec<u8>,
    meta: CaptureMeta,
}

/// The whole body is read before storage begins: the store is blocking work on
/// another thread, and it cannot pull the rest of a stream from this one.
async fn read_parts(mut multipart: Multipart) -> std::result::Result<Capture, Refusal> {
    let mut bytes = None;
    let mut meta = None;

    while let Some(field) = multipart.next_field().await.map_err(malformed)? {
        // Owned: reading the field's body consumes it, and the name is needed
        // after that.
        let name = field.name().unwrap_or_default().to_string();
        match name.as_str() {
            FILE_FIELD => {
                let read = field.bytes().await.map_err(malformed)?;
                set_once(&mut bytes, FILE_FIELD, read.to_vec())?;
            }
            META_FIELD => {
                let read = field.text().await.map_err(malformed)?;
                let parsed = serde_json::from_str::<CaptureMeta>(&read).map_err(|error| {
                    (
                        StatusCode::BAD_REQUEST,
                        format!("the {META_FIELD} part is not a capture: {error}"),
                    )
                })?;
                set_once(&mut meta, META_FIELD, parsed)?;
            }
            unknown => {
                return Err((
                    StatusCode::BAD_REQUEST,
                    format!(
                        "unexpected multipart field {unknown:?}; \
                         a capture carries exactly {FILE_FIELD:?} and {META_FIELD:?}"
                    ),
                ));
            }
        }
    }

    Ok(Capture {
        bytes: bytes.ok_or_else(|| missing(FILE_FIELD))?,
        meta: meta.ok_or_else(|| missing(META_FIELD))?,
    })
}

fn set_once<T>(slot: &mut Option<T>, field: &str, value: T) -> std::result::Result<(), Refusal> {
    if slot.is_some() {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("more than one {field:?} part; a capture carries exactly one"),
        ));
    }
    *slot = Some(value);
    Ok(())
}

fn missing(field: &str) -> Refusal {
    (
        StatusCode::BAD_REQUEST,
        format!("the {field:?} part is missing"),
    )
}

/// axum has already decided what a malformed or oversized multipart body is
/// worth (413 past `MAX_BODY_BYTES`, 400 for the rest). Judging it again here
/// would only let the two answers disagree.
fn malformed(error: MultipartError) -> Refusal {
    (error.status(), error.body_text())
}

/// Store on a blocking thread, and hold the library lock only there.
///
/// `SharedLibrary` is a `std::sync::Mutex` and every step under it blocks
/// (decode, fsync, SQLite). Locking it in async code and awaiting while holding
/// it parks a runtime worker with the one connection in hand, and the next
/// request scheduled onto that worker waits on a lock that cannot be released:
/// the whole listener stops, with nothing at the call site to show why.
async fn store(state: HttpState, capture: Capture) -> Response {
    let library = state.library.clone();
    match tokio::task::spawn_blocking(move || store_now(&library, &capture)).await {
        Ok(Ok(Ingested::Created(record))) => {
            // Only a new row is announced. A re-post under a known id changed
            // nothing, and telling the webview otherwise would reload the grid
            // under the user for an image already in it.
            (state.on_stored)(&record);
            (StatusCode::CREATED, Json(record)).into_response()
        }
        Ok(Ok(Ingested::Existing(record))) => (StatusCode::OK, Json(record)).into_response(),
        Ok(Err(error)) => error_response(&error),
        Err(panicked) => reason(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("storing the capture failed: {panicked}"),
        ),
    }
}

fn store_now(library: &SharedLibrary, capture: &Capture) -> Result<Ingested> {
    let meta = &capture.meta;
    with_library(library, |library| {
        ingest::store_image(
            library,
            IngestInput {
                id: &meta.id,
                bytes: &capture.bytes,
                source: ImageSource::Extension,
                // The site stays on `source_ref` so per-source counts and that
                // column keep the meaning they have; the whole record goes to
                // `adapter` beside it (design D11). Nothing here reads a field
                // out of it — that is policy, and policy lives in the app's
                // rules, not in the door every image enters by.
                source_ref: meta.adapter.as_ref().map(|adapter| adapter.site.as_str()),
                image_url: Some(&meta.image_url),
                page_url: Some(&meta.page_url),
                page_title: Some(&meta.page_title),
                adapter: meta.adapter.as_ref(),
                rating: None,
                tags: &[],
                captured_at: meta.captured_at,
            },
        )
    })
}

#[cfg(test)]
mod tests {
    use axum::http::StatusCode;

    use crate::http::test_support::*;

    #[tokio::test]
    async fn a_new_capture_is_stored_and_returned() {
        let (_dir, state) = open_state();
        let png = png_bytes(4, 7);
        let meta = meta_json("id-1");

        let (status, body) = send(
            &state,
            capture_request(Some(EXTENSION_ORIGIN), &[file_part(&png), meta_part(&meta)]),
        )
        .await;

        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(body["id"], "id-1");
        assert_eq!(body["source"], "extension");
        assert_eq!(body["sourceRef"], "danbooru");
        assert_eq!(body["width"], 4);
        assert_eq!(body["height"], 7);
        assert_eq!(body["pageTitle"], "a page");
        assert!(stored_file_exists(&state, "id-1", "png"));
        assert_eq!(image_count(&state), 1);
    }

    fn meta_with_adapter(id: &str, adapter: serde_json::Value) -> String {
        serde_json::json!({
            "id": id,
            "imageUrl": "https://example.test/i.png",
            "pageUrl": "https://example.test/p",
            "pageTitle": "a page",
            "capturedAt": 1_700_000_000_000i64,
            "adapter": adapter,
        })
        .to_string()
    }

    #[tokio::test]
    async fn the_adapter_record_survives_the_round_trip() {
        let (_dir, state) = open_state();
        let png = png_bytes(4, 7);
        let meta = meta_with_adapter(
            "id-1",
            serde_json::json!({
                "site": "x",
                "fields": {
                    "handle": "alice",
                    "postUrl": "https://x.com/alice/status/1",
                },
            }),
        );

        let (status, body) = send(
            &state,
            capture_request(Some(EXTENSION_ORIGIN), &[file_part(&png), meta_part(&meta)]),
        )
        .await;

        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(body["sourceRef"], "x");
        assert_eq!(body["adapter"]["site"], "x");
        assert_eq!(body["adapter"]["fields"]["handle"], "alice");
        assert_eq!(
            body["adapter"]["fields"]["postUrl"],
            "https://x.com/alice/status/1"
        );
    }

    #[tokio::test]
    async fn adapter_fields_the_app_has_no_meaning_for_are_kept_as_sent() {
        let (_dir, state) = open_state();
        let png = png_bytes(4, 7);
        let meta = meta_with_adapter(
            "id-1",
            serde_json::json!({
                "site": "somewhere-new",
                "fields": { "whatIsThis": ["one", "two"], "andThis": "a value" },
            }),
        );

        let (status, body) = send(
            &state,
            capture_request(Some(EXTENSION_ORIGIN), &[file_part(&png), meta_part(&meta)]),
        )
        .await;

        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(
            body["adapter"]["fields"]["whatIsThis"],
            serde_json::json!(["one", "two"])
        );
        assert_eq!(body["adapter"]["fields"]["andThis"], "a value");
        assert_eq!(image_count(&state), 1);
    }

    #[tokio::test]
    async fn a_capture_without_an_adapter_record_is_stored_without_one() {
        let (_dir, state) = open_state();
        let png = png_bytes(4, 7);
        let meta = serde_json::json!({
            "id": "id-1",
            "imageUrl": "https://example.test/i.png",
            "pageUrl": "https://example.test/p",
            "pageTitle": "a page",
            "capturedAt": 1_700_000_000_000i64,
        })
        .to_string();

        let (status, body) = send(
            &state,
            capture_request(Some(EXTENSION_ORIGIN), &[file_part(&png), meta_part(&meta)]),
        )
        .await;

        assert_eq!(status, StatusCode::CREATED);
        assert!(body["adapter"].is_null());
        assert!(body["sourceRef"].is_null());
        assert_eq!(image_count(&state), 1);
    }

    #[tokio::test]
    async fn a_stored_capture_is_announced_once_and_a_re_post_is_not() {
        let (_dir, state, stored) = open_state_recording();
        let png = png_bytes(4, 7);
        let meta = meta_json("id-1");
        let request =
            || capture_request(Some(EXTENSION_ORIGIN), &[file_part(&png), meta_part(&meta)]);

        send(&state, request()).await;
        assert_eq!(stored.lock().unwrap().as_slice(), ["id-1"]);

        // The retry of a delivery the app already accepted. Nothing changed, so
        // nothing is announced and the grid stays where the user left it.
        send(&state, request()).await;
        assert_eq!(stored.lock().unwrap().as_slice(), ["id-1"]);
    }

    #[tokio::test]
    async fn a_refused_capture_is_never_announced() {
        let (_dir, state, stored) = open_state_recording();

        send(
            &state,
            capture_request(Some(EXTENSION_ORIGIN), &[meta_part(&meta_json("id-1"))]),
        )
        .await;

        assert!(stored.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn the_same_id_again_returns_the_stored_record() {
        let (_dir, state) = open_state();
        let first = png_bytes(4, 7);
        let second = png_bytes(9, 9);
        let meta = meta_json("id-1");

        send(
            &state,
            capture_request(
                Some(EXTENSION_ORIGIN),
                &[file_part(&first), meta_part(&meta)],
            ),
        )
        .await;
        let (status, body) = send(
            &state,
            capture_request(
                Some(EXTENSION_ORIGIN),
                &[file_part(&second), meta_part(&meta)],
            ),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            body["width"], 4,
            "the retry must not replace the stored image"
        );
        assert_eq!(image_count(&state), 1);
    }

    #[tokio::test]
    async fn a_missing_file_part_is_refused() {
        let (_dir, state) = open_state();
        let meta = meta_json("id-1");

        let (status, body) = send(
            &state,
            capture_request(Some(EXTENSION_ORIGIN), &[meta_part(&meta)]),
        )
        .await;

        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(body["error"].as_str().unwrap().contains("file"));
        assert_eq!(image_count(&state), 0);
    }

    #[tokio::test]
    async fn a_missing_meta_part_is_refused() {
        let (_dir, state) = open_state();
        let png = png_bytes(4, 7);

        let (status, body) = send(
            &state,
            capture_request(Some(EXTENSION_ORIGIN), &[file_part(&png)]),
        )
        .await;

        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(body["error"].as_str().unwrap().contains("meta"));
        assert_eq!(image_count(&state), 0);
    }

    #[tokio::test]
    async fn meta_without_an_id_is_refused() {
        let (_dir, state) = open_state();
        let png = png_bytes(4, 7);
        let meta = serde_json::json!({
            "imageUrl": "https://example.test/i.png",
            "pageUrl": "https://example.test/p",
            "pageTitle": "a page",
            "capturedAt": 1_700_000_000_000i64,
        })
        .to_string();

        let (status, body) = send(
            &state,
            capture_request(Some(EXTENSION_ORIGIN), &[file_part(&png), meta_part(&meta)]),
        )
        .await;

        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(body["error"].as_str().unwrap().contains("id"));
        assert_eq!(image_count(&state), 0);
    }

    #[tokio::test]
    async fn an_unknown_field_name_is_refused() {
        let (_dir, state) = open_state();
        let png = png_bytes(4, 7);
        let meta = meta_json("id-1");
        let mut parts = vec![file_part(&png), meta_part(&meta)];
        parts[1].name = "metadata";

        let (status, body) = send(&state, capture_request(Some(EXTENSION_ORIGIN), &parts)).await;

        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(body["error"].as_str().unwrap().contains("metadata"));
        assert_eq!(image_count(&state), 0);
    }

    #[tokio::test]
    async fn an_undecodable_image_is_refused() {
        let (_dir, state) = open_state();
        let meta = meta_json("id-1");

        let (status, body) = send(
            &state,
            capture_request(
                Some(EXTENSION_ORIGIN),
                &[file_part(b"not an image at all"), meta_part(&meta)],
            ),
        )
        .await;

        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert!(body["error"].is_string(), "body was {body}");
        assert_eq!(image_count(&state), 0);
    }
}
