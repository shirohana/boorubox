//! `GET /status`: app version, library path and image count; 503 when no
//! library is open.

use axum::Json;
use axum::extract::State;
use axum::response::{IntoResponse, Response};

use crate::error::Result;
use crate::http::{HttpState, error_response};
use crate::library::with_library;
use crate::model::StatusResponse;

pub async fn get_status(State(state): State<HttpState>) -> Response {
    match read_status(&state) {
        Ok(status) => Json(status).into_response(),
        // `NoLibrary` is the 503 the extension shows as "app open, no library".
        Err(error) => error_response(&error),
    }
}

fn read_status(state: &HttpState) -> Result<StatusResponse> {
    with_library(&state.library, |library| {
        Ok(StatusResponse {
            version: state.version.clone(),
            library_path: library.paths.root.display().to_string(),
            image_count: library.image_count()?,
        })
    })
}

#[cfg(test)]
mod tests {
    use axum::http::StatusCode;

    use crate::http::test_support::*;

    #[tokio::test]
    async fn status_reports_the_version_path_and_count() {
        let (dir, state) = open_state();
        let png = png_bytes(4, 7);
        let meta = meta_json("id-1");
        send(
            &state,
            capture_request(Some(EXTENSION_ORIGIN), &[file_part(&png), meta_part(&meta)]),
        )
        .await;

        let (status, body) = send(&state, get_request("/status", Some(EXTENSION_ORIGIN))).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["version"], "1.2.3");
        assert_eq!(body["libraryPath"], dir.path().display().to_string());
        assert_eq!(body["imageCount"], 1);
    }

    #[tokio::test]
    async fn status_without_a_library_is_unavailable() {
        let state = empty_state();

        let (status, body) = send(&state, get_request("/status", None)).await;

        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(body["error"], "no library is open");
    }
}
