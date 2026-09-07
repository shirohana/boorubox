//! `POST /captures/pending` and `DELETE /captures/pending/{id}`: the two relays
//! that let the app show a capture before its bytes exist (design D2). Neither
//! stores anything — a pending capture is webview state, and the database never
//! hears of it.

use axum::Json;
use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Deserialize;

use crate::http::{CaptureEvent, HttpState, error_response, reason};
use crate::library::with_library;
use crate::model::{CaptureMeta, CaptureWithdrawn};

/// The announcement's body is the `meta` the capture will be posted with, so a
/// body that is not one is refused the way `POST /captures` refuses the same
/// JSON. The extractor's own rejection would answer 422 for a missing field;
/// one shape of refusal for one shape of body is worth the `Result`.
pub async fn post_pending(
    State(state): State<HttpState>,
    meta: Result<Json<CaptureMeta>, JsonRejection>,
) -> Response {
    let meta = match meta {
        Ok(Json(meta)) => meta,
        Err(rejection) => {
            return reason(
                StatusCode::BAD_REQUEST,
                format!("the body is not a capture: {}", rejection.body_text()),
            );
        }
    };

    // The capture this announces would be refused for the same reason, and a
    // placeholder for it would promise an image that cannot arrive.
    if let Err(error) = with_library(&state.library, |_| Ok(())) {
        return error_response(&error);
    }

    (state.on_event)(CaptureEvent::Pending(meta));
    StatusCode::ACCEPTED.into_response()
}

/// Why the capture will not arrive, when the extension named one.
#[derive(Debug, Deserialize)]
pub struct Withdrawal {
    #[serde(default)]
    reason: Option<String>,
}

/// 204 for any id, announced or not: the app holds no set to check one against,
/// and a client retrying a lost response wants the same answer twice.
pub async fn delete_pending(
    State(state): State<HttpState>,
    Path(id): Path<String>,
    body: Option<Json<Withdrawal>>,
) -> Response {
    let withdrawn = CaptureWithdrawn {
        id,
        reason: body.and_then(|Json(body)| body.reason),
    };
    (state.on_event)(CaptureEvent::Withdrawn(withdrawn));
    StatusCode::NO_CONTENT.into_response()
}

#[cfg(test)]
mod tests {
    use axum::http::StatusCode;

    use crate::http::test_support::*;
    use crate::model::CaptureWithdrawn;

    #[tokio::test]
    async fn an_announcement_reaches_the_window_and_stores_nothing() {
        let (_dir, state, events) = open_state_recording();

        let (status, _) = send(
            &state,
            announce_request(Some(EXTENSION_ORIGIN), &meta_json("id-1")),
        )
        .await;

        assert_eq!(status, StatusCode::ACCEPTED);
        let [announced] = events.announced().try_into().unwrap();
        assert_eq!(announced.id, "id-1");
        assert_eq!(announced.page_title, "a page");
        assert_eq!(announced.image_url, "https://example.test/i.png");
        assert_eq!(announced.captured_at, 1_700_000_000_000);
        assert_eq!(announced.adapter.unwrap().site, "danbooru");
        assert_eq!(
            image_count(&state),
            0,
            "an announcement is not library content",
        );
    }

    #[tokio::test]
    async fn an_announcement_with_no_library_open_is_unavailable_and_silent() {
        let (state, events) = empty_state_recording();

        let (status, body) = send(
            &state,
            announce_request(Some(EXTENSION_ORIGIN), &meta_json("id-1")),
        )
        .await;

        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(body["error"], "no library is open");
        assert!(events.all().is_empty());
    }

    #[tokio::test]
    async fn an_announcement_without_an_id_is_refused_and_silent() {
        let (_dir, state, events) = open_state_recording();
        let body = serde_json::json!({
            "imageUrl": "https://example.test/i.png",
            "pageUrl": "https://example.test/p",
            "pageTitle": "a page",
            "capturedAt": 1_700_000_000_000i64,
        })
        .to_string();

        let (status, answer) = send(&state, announce_request(Some(EXTENSION_ORIGIN), &body)).await;

        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(answer["error"].as_str().unwrap().contains("id"));
        assert!(events.all().is_empty());
    }

    #[tokio::test]
    async fn an_announcement_that_is_not_json_is_refused_and_silent() {
        let (_dir, state, events) = open_state_recording();

        let (status, _) = send(
            &state,
            announce_request(Some(EXTENSION_ORIGIN), "not json at all"),
        )
        .await;

        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(events.all().is_empty());
    }

    #[tokio::test]
    async fn a_web_page_cannot_announce_or_withdraw() {
        let (_dir, state, events) = open_state_recording();
        let page = Some("https://example.com");

        let (announced, _) = send(&state, announce_request(page, &meta_json("id-1"))).await;
        let (withdrawn, _) = send(&state, withdraw_request(page, "id-1", Some("no"))).await;

        assert_eq!(announced, StatusCode::FORBIDDEN);
        assert_eq!(withdrawn, StatusCode::FORBIDDEN);
        assert!(events.all().is_empty());
    }

    #[tokio::test]
    async fn a_withdrawal_carries_its_reason_to_the_window() {
        let (_dir, state, events) = open_state_recording();
        send(
            &state,
            announce_request(Some(EXTENSION_ORIGIN), &meta_json("id-1")),
        )
        .await;

        let (status, _) = send(
            &state,
            withdraw_request(Some(EXTENSION_ORIGIN), "id-1", Some("HTTP 403: Forbidden")),
        )
        .await;

        assert_eq!(status, StatusCode::NO_CONTENT);
        assert_eq!(
            events.withdrawals(),
            [CaptureWithdrawn {
                id: "id-1".to_string(),
                reason: Some("HTTP 403: Forbidden".to_string()),
            }],
        );
    }

    #[tokio::test]
    async fn a_withdrawal_for_an_id_never_announced_is_answered_the_same_way() {
        let (_dir, state, events) = open_state_recording();

        let (status, _) = send(
            &state,
            withdraw_request(Some(EXTENSION_ORIGIN), "never-heard-of", None),
        )
        .await;

        assert_eq!(status, StatusCode::NO_CONTENT);
        assert_eq!(
            events.withdrawals(),
            [CaptureWithdrawn {
                id: "never-heard-of".to_string(),
                reason: None,
            }],
        );
    }
}
