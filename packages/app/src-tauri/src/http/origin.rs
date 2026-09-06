//! The layer that rejects any request whose `Origin` is not an extension, before
//! the body is read. `GET /status` is the one exemption (spec `capture-ingest`).

use axum::extract::Request;
use axum::http::header::ORIGIN;
use axum::http::{Method, StatusCode};
use axum::middleware::Next;
use axum::response::Response;

use crate::http::reason;
use crate::model::EXTENSION_ORIGIN_PREFIX;

/// Refusing here rather than in the handler is the point: `next.run` is what
/// starts reading the body, so a web page that fails this check never gets to
/// stream megabytes into the app.
pub async fn require_extension_origin(request: Request, next: Next) -> Response {
    if is_status_probe(&request) || has_extension_origin(&request) {
        return next.run(request).await;
    }
    reason(
        StatusCode::FORBIDDEN,
        format!("this listener accepts only requests from {EXTENSION_ORIGIN_PREFIX}… origins"),
    )
}

/// `GET /status` is what the extension's connected indicator and the migration
/// notice call, and neither sends an `Origin` (§5).
fn is_status_probe(request: &Request) -> bool {
    request.method() == Method::GET && request.uri().path() == "/status"
}

fn has_extension_origin(request: &Request) -> bool {
    request
        .headers()
        .get(ORIGIN)
        .and_then(|origin| origin.to_str().ok())
        .is_some_and(|origin| origin.starts_with(EXTENSION_ORIGIN_PREFIX))
}

#[cfg(test)]
mod tests {
    use axum::http::StatusCode;

    use crate::http::test_support::*;

    #[tokio::test]
    async fn a_web_page_origin_is_refused_and_stores_nothing() {
        let (_dir, state) = open_state();
        let png = png_bytes(4, 7);
        let meta = meta_json("id-1");

        let (status, body) = send(
            &state,
            capture_request(
                Some("https://example.com"),
                &[file_part(&png), meta_part(&meta)],
            ),
        )
        .await;

        assert_eq!(status, StatusCode::FORBIDDEN);
        assert!(body["error"].is_string(), "body was {body}");
        assert_eq!(image_count(&state), 0);
    }

    #[tokio::test]
    async fn a_capture_without_an_origin_is_refused() {
        let (_dir, state) = open_state();
        let png = png_bytes(4, 7);
        let meta = meta_json("id-1");

        let (status, _) = send(
            &state,
            capture_request(None, &[file_part(&png), meta_part(&meta)]),
        )
        .await;

        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(image_count(&state), 0);
    }

    #[tokio::test]
    async fn an_extension_origin_reaches_the_handler() {
        let (_dir, state) = open_state();

        // An empty body, so the answer can only come from the handler: 400 here
        // means the layer let the request through.
        let (status, _) = send(&state, capture_request(Some(EXTENSION_ORIGIN), &[])).await;

        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn status_is_answered_without_an_origin() {
        let (_dir, state) = open_state();

        let (status, _) = send(&state, get_request("/status", None)).await;

        assert_eq!(status, StatusCode::OK);
    }
}
