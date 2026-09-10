//! `POST /captures`: the multipart handler that turns one file part and one
//! JSON part into a call to `ingest::store_image`.

use axum::Json;
use axum::extract::multipart::MultipartError;
use axum::extract::{Multipart, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

use crate::error::Result;
use crate::http::{CaptureEvent, HttpState, error_response, reason};
use crate::ingest::{self, IngestInput, Ingested};
use crate::library::{SharedLibrary, with_library};
use crate::model::{CaptureMeta, CaptureWithdrawn, ImageSource};
use crate::thumbs;

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
    let id = capture.meta.id.clone();
    let library = state.library.clone();
    match tokio::task::spawn_blocking(move || store_now(&library, &capture)).await {
        Ok(Ok(Ingested::Created(record))) => {
            // Only a new row is announced as stored. A re-post under a known id changed
            // nothing, and telling the webview otherwise would reload the grid
            // under the user for an image already in it.
            (state.on_event)(CaptureEvent::Stored(Box::new(record.clone())));
            (StatusCode::CREATED, Json(record)).into_response()
        }
        Ok(Ok(Ingested::Existing(record))) => {
            withdraw(&state, id, None);
            (StatusCode::OK, Json(record)).into_response()
        }
        Ok(Err(error)) => {
            withdraw(&state, id, Some(error.to_string()));
            error_response(&error)
        }
        Err(panicked) => {
            let message = format!("storing the capture failed: {panicked}");
            withdraw(&state, id, Some(message.clone()));
            reason(StatusCode::INTERNAL_SERVER_ERROR, message)
        }
    }
}

/// Settle the announcement the extension made before it had the bytes, on the
/// same answer it is about to read (design D6). Without this the placeholder
/// would hang until its timeout on every answer that is not a new row, and the
/// extension would need a second request to clear it.
fn withdraw(state: &HttpState, id: String, reason: Option<String>) {
    (state.on_event)(CaptureEvent::Withdrawn(CaptureWithdrawn { id, reason }));
}

fn store_now(library: &SharedLibrary, capture: &Capture) -> Result<Ingested> {
    let meta = &capture.meta;
    let (paths, ingested) = with_library(library, |library| {
        let ingested = ingest::store_image(
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
                // The HTTP capture path is never a file on disk: the extension
                // sends bytes over the wire, so there is no modification time
                // to record (design D11, `browse-polish`).
                file_modified_at: None,
            },
        )?;
        Ok((library.paths.clone(), ingested))
    })?;

    // Outside the lock: the encode is the largest part of the hold, and the
    // window is scrolling a grid served by the same connection (design D13).
    if let Ingested::Created(record) = &ingested {
        thumbs::warm_thumbnail(&paths, record);
    }
    Ok(ingested)
}

#[cfg(test)]
mod tests {
    use axum::http::StatusCode;

    use crate::http::test_support::*;
    use crate::library::with_library;
    use crate::model::CaptureWithdrawn;

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

    /// The guarantee used to sit inside `ingest::store_image`; it moved out to
    /// keep the encode off the library lock (design D13), so it is pinned here,
    /// where the capture path now does it.
    #[tokio::test]
    async fn a_stored_capture_is_thumbnailed_before_the_answer() {
        let (_dir, state) = open_state();
        let png = png_bytes(400, 300);
        let meta = meta_json("id-1");

        let (status, _) = send(
            &state,
            capture_request(Some(EXTENSION_ORIGIN), &[file_part(&png), meta_part(&meta)]),
        )
        .await;

        assert_eq!(status, StatusCode::CREATED);
        assert!(thumbnail_exists(&state, "id-1"));
    }

    /// Design D11 (`browse-polish`): the HTTP capture path never has a file on
    /// disk to read a modification time from, so the column stays absent
    /// rather than 0 or the capture time.
    #[tokio::test]
    async fn a_captured_image_carries_no_file_modification_time() {
        let (_dir, state) = open_state();
        let png = png_bytes(4, 7);
        let meta = meta_json("id-1");

        let (status, body) = send(
            &state,
            capture_request(Some(EXTENSION_ORIGIN), &[file_part(&png), meta_part(&meta)]),
        )
        .await;

        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(body["fileModifiedAt"], serde_json::Value::Null);
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

    /// `auto-tag-rules` task 2.5: an end-to-end pass through the HTTP layer — a
    /// rule matching the adapter record's `handle` tags the capture on the way
    /// in, both in the response and in the stored row (spec `auto-tag-rules`,
    /// "A capture arrives tagged").
    #[tokio::test]
    async fn a_rule_matching_the_adapter_record_tags_the_capture() {
        let (_dir, state) = open_state();
        with_library(&state.library, |library| {
            crate::rules::upsert(
                library,
                &crate::model::RuleInput {
                    id: None,
                    name: "alice".to_string(),
                    pattern: "alice".to_string(),
                    is_regex: false,
                    tags: vec!["alice-fanart".to_string()],
                    enabled: true,
                },
            )
            .map(|_| ())
        })
        .unwrap();
        let png = png_bytes(4, 7);
        let meta = meta_with_adapter(
            "id-1",
            serde_json::json!({
                "site": "x",
                "fields": { "handle": "alice" },
            }),
        );

        let (status, body) = send(
            &state,
            capture_request(Some(EXTENSION_ORIGIN), &[file_part(&png), meta_part(&meta)]),
        )
        .await;

        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(body["tags"], serde_json::json!(["alice-fanart"]));
        with_library(&state.library, |library| {
            assert_eq!(
                crate::ingest::require_record(&library.conn, "id-1")
                    .unwrap()
                    .tags,
                vec!["alice-fanart".to_string()],
            );
            Ok(())
        })
        .unwrap();
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
        let (_dir, state, events) = open_state_recording();
        let png = png_bytes(4, 7);
        let meta = meta_json("id-1");
        let request =
            || capture_request(Some(EXTENSION_ORIGIN), &[file_part(&png), meta_part(&meta)]);

        send(&state, request()).await;
        assert_eq!(events.stored_ids(), ["id-1"]);
        assert!(events.withdrawals().is_empty());

        // The retry of a delivery the app already accepted. Nothing was stored,
        // so the grid stays where the user left it — and the announcement that
        // came with the retry is settled as withdrawn.
        send(&state, request()).await;
        assert_eq!(events.stored_ids(), ["id-1"]);
        assert_eq!(
            events.withdrawals(),
            [CaptureWithdrawn {
                id: "id-1".to_string(),
                reason: None,
            }],
        );
    }

    #[tokio::test]
    async fn a_refused_capture_is_never_announced() {
        let (_dir, state, events) = open_state_recording();

        send(
            &state,
            capture_request(Some(EXTENSION_ORIGIN), &[meta_part(&meta_json("id-1"))]),
        )
        .await;

        assert!(events.stored_ids().is_empty());
    }

    #[tokio::test]
    async fn a_refusal_the_id_could_be_read_from_withdraws_it_with_the_reason() {
        let (_dir, state, events) = open_state_recording();

        let (status, body) = send(
            &state,
            capture_request(
                Some(EXTENSION_ORIGIN),
                &[
                    file_part(b"not an image at all"),
                    meta_part(&meta_json("id-1")),
                ],
            ),
        )
        .await;

        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert!(events.stored_ids().is_empty());
        // The same words the extension is about to read, so the app and the
        // popup entry never name two different failures.
        assert_eq!(
            events.withdrawals(),
            [CaptureWithdrawn {
                id: "id-1".to_string(),
                reason: Some(body["error"].as_str().unwrap().to_string()),
            }],
        );
    }

    #[tokio::test]
    async fn a_request_refused_before_its_id_was_read_settles_nothing() {
        let (_dir, state, events) = open_state_recording();
        let png = png_bytes(4, 7);

        let (status, _) = send(
            &state,
            capture_request(Some(EXTENSION_ORIGIN), &[file_part(&png)]),
        )
        .await;

        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(events.all().is_empty(), "there is no id to settle");
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
