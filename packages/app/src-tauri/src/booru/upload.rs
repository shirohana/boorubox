//! The upload sequence's orchestration (`booru-upload` design D5, D6): turns
//! four `BooruClient` calls into one `BooruUploadOutcome`, with no database
//! access at all. `commands::booru_upload` is the only caller — it reads the
//! image and the site before this runs and records the post after, each a
//! brief, separate hold of the library mutex, never across an `.await`.

use md5::{Digest, Md5};

use crate::booru::client::{BooruClient, PollSchedule};
use crate::model::{
    BooruUploadForm, BooruUploadOutcome, CommentaryOutcome, PostRef, UploadStep, UploadStepError,
};

/// One image, decoded down to what the client needs: its bytes, the filename
/// and mime type to send them under. Built by the caller from the
/// `ImageRecord` and the library's own file, so this module never touches a
/// path.
pub struct UploadPayload {
    pub filename: String,
    pub mime: String,
    pub bytes: Vec<u8>,
}

/// Run the whole sequence: refuse a file the booru already holds, create the
/// upload, wait for it to process, create the post, then — only if a
/// commentary was given — set it (design: "Each step of the upload is
/// reported separately"). `site_id` becomes `PostRef.site` on success;
/// `posted_at` is left at `0`, the caller's signal to stamp it at the moment
/// it actually commits the record (design D5 — the time of posting is the
/// time of the durable write, not of the network call that preceded it).
pub async fn run(
    client: &BooruClient,
    payload: UploadPayload,
    site_id: &str,
    form: &BooruUploadForm,
    poll: PollSchedule,
) -> BooruUploadOutcome {
    // Danbooru keys posts by the file's MD5 and does not refuse a duplicate
    // until the post step, where it merges the sent tags and rating into the
    // original post instead (design D12). Asking first is what keeps a
    // re-upload from another machine's post a refusal and not an edit.
    let md5 = format!("{:x}", Md5::digest(&payload.bytes));
    match client.find_post_by_md5(&md5).await {
        Ok(None) => {}
        Ok(Some(post_id)) => {
            return BooruUploadOutcome::Failed {
                error: UploadStepError {
                    step: UploadStep::CreateUpload,
                    message: format!("the booru already has this file as post #{post_id}"),
                    remote_ref: Some(post_id),
                },
            };
        }
        Err(error) => return BooruUploadOutcome::Failed { error },
    }

    let upload_id = match client
        .create_upload(&payload.filename, &payload.mime, payload.bytes)
        .await
    {
        Ok(id) => id,
        Err(error) => return BooruUploadOutcome::Failed { error },
    };

    let media_asset_id = match client.await_processing(&upload_id, poll).await {
        Ok(id) => id,
        Err(error) => return BooruUploadOutcome::Failed { error },
    };

    // The artist candidate has no field of its own on `POST /posts.json`
    // (design D3, D10) — Danbooru has none — so it joins `tag_string` as an
    // ordinary tag, the same way any other tag does.
    let tag_string = tag_string_with_artist(form);
    let post_id = match client
        .create_post(&media_asset_id, &tag_string, &form.rating, &form.source)
        .await
    {
        Ok(id) => id,
        Err(error) => return BooruUploadOutcome::Failed { error },
    };

    let commentary = if form.commentary_title.is_empty() && form.commentary_body.is_empty() {
        CommentaryOutcome::Skipped
    } else {
        match client
            .set_commentary(&post_id, &form.commentary_title, &form.commentary_body)
            .await
        {
            Ok(()) => CommentaryOutcome::Applied,
            // A commentary failure never turns the upload itself into a
            // failure (design D6): the post already exists.
            Err(error) => CommentaryOutcome::Failed {
                message: error.message,
            },
        }
    };

    BooruUploadOutcome::Posted {
        post: PostRef {
            site: site_id.to_string(),
            remote_id: post_id,
            posted_at: 0,
        },
        commentary,
    }
}

fn tag_string_with_artist(form: &BooruUploadForm) -> String {
    let artist = form.artist.trim();
    if artist.is_empty() || form.tags.iter().any(|tag| tag == artist) {
        return form.tags.join(" ");
    }
    let mut tags = form.tags.clone();
    tags.push(artist.to_string());
    tags.join(" ")
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use serde_json::json;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;
    use crate::model::UploadStep;

    /// Milliseconds, not the shipped two seconds: the suite drives the whole
    /// processing phase without sleeping for it (design D14).
    fn fast_polling() -> PollSchedule {
        PollSchedule {
            interval: Duration::from_millis(1),
            deadline: Duration::from_secs(30),
        }
    }

    fn form(
        tags: &[&str],
        artist: &str,
        commentary_title: &str,
        commentary_body: &str,
    ) -> BooruUploadForm {
        BooruUploadForm {
            tags: tags.iter().map(|tag| (*tag).to_string()).collect(),
            rating: "s".to_string(),
            source: "https://example.test/p".to_string(),
            artist: artist.to_string(),
            commentary_title: commentary_title.to_string(),
            commentary_body: commentary_body.to_string(),
        }
    }

    fn payload() -> UploadPayload {
        UploadPayload {
            filename: "a.png".to_string(),
            mime: "image/png".to_string(),
            bytes: vec![1, 2, 3],
        }
    }

    /// The MD5 lookup that precedes every upload, answered "no such post".
    async fn no_post_holds_the_file(server: &MockServer) {
        Mock::given(method("GET"))
            .and(path("/posts.json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
            .mount(server)
            .await;
    }

    async fn happy_path_server() -> MockServer {
        let server = MockServer::start().await;
        no_post_holds_the_file(&server).await;
        Mock::given(method("POST"))
            .and(path("/uploads.json"))
            .respond_with(ResponseTemplate::new(201).set_body_json(json!({ "id": 9 })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/uploads/9.json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "status": "completed",
                "upload_media_assets": [{ "id": 77, "media_asset_id": 4001 }],
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/posts.json"))
            .respond_with(ResponseTemplate::new(201).set_body_json(json!({ "id": 555 })))
            .mount(&server)
            .await;
        server
    }

    #[tokio::test]
    async fn a_full_run_posts_and_folds_the_artist_into_the_tag_string() {
        let server = happy_path_server().await;
        Mock::given(method("PUT"))
            .and(path("/posts/555/artist_commentary/create_or_update.json"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&server)
            .await;
        let client = BooruClient::new(&server.uri(), "alice", "key");

        let outcome = run(
            &client,
            payload(),
            "danbooru",
            &form(&["1girl"], "pixiv_user_1", "a title", "a body"),
            fast_polling(),
        )
        .await;

        match outcome {
            BooruUploadOutcome::Posted { post, commentary } => {
                assert_eq!(post.site, "danbooru");
                assert_eq!(post.remote_id, "555");
                assert_eq!(commentary, CommentaryOutcome::Applied);
            }
            BooruUploadOutcome::Failed { error } => panic!("expected success, got {error:?}"),
        }
    }

    #[test]
    fn the_artist_is_appended_to_the_tags_when_absent() {
        assert_eq!(
            tag_string_with_artist(&form(&["1girl", "blue_sky"], "pixiv_user_1", "", "")),
            "1girl blue_sky pixiv_user_1"
        );
    }

    #[test]
    fn an_artist_already_among_the_tags_is_not_duplicated() {
        assert_eq!(
            tag_string_with_artist(&form(&["1girl", "pixiv_user_1"], "pixiv_user_1", "", "")),
            "1girl pixiv_user_1"
        );
    }

    #[test]
    fn an_empty_artist_changes_nothing() {
        assert_eq!(
            tag_string_with_artist(&form(&["1girl"], "", "", "")),
            "1girl"
        );
    }

    #[tokio::test]
    async fn no_commentary_given_is_skipped_not_attempted() {
        let server = happy_path_server().await;
        // No mock for the commentary endpoint: a call to it would fail the
        // test with "unexpected request", which is exactly what proves it was
        // never attempted.
        let client = BooruClient::new(&server.uri(), "alice", "key");

        let outcome = run(
            &client,
            payload(),
            "danbooru",
            &form(&["1girl"], "", "", ""),
            fast_polling(),
        )
        .await;

        assert!(matches!(
            outcome,
            BooruUploadOutcome::Posted {
                commentary: CommentaryOutcome::Skipped,
                ..
            }
        ));
    }

    #[tokio::test]
    async fn a_commentary_failure_still_counts_as_a_successful_post() {
        let server = happy_path_server().await;
        Mock::given(method("PUT"))
            .and(path("/posts/555/artist_commentary/create_or_update.json"))
            .respond_with(
                ResponseTemplate::new(422).set_body_json(json!({ "message": "title too long" })),
            )
            .mount(&server)
            .await;
        let client = BooruClient::new(&server.uri(), "alice", "key");

        let outcome = run(
            &client,
            payload(),
            "danbooru",
            &form(&["1girl"], "", "a title", ""),
            fast_polling(),
        )
        .await;

        match outcome {
            BooruUploadOutcome::Posted { post, commentary } => {
                assert_eq!(post.remote_id, "555");
                assert_eq!(
                    commentary,
                    CommentaryOutcome::Failed {
                        message: "title too long".to_string()
                    }
                );
            }
            BooruUploadOutcome::Failed { error } => panic!("expected success, got {error:?}"),
        }
    }

    /// Design D12: a file the booru already holds is refused before it is
    /// sent, naming the post — never handed to `POST /posts.json`, which
    /// would merge the form's tags and rating into that post.
    #[tokio::test]
    async fn a_file_the_booru_already_holds_is_refused_before_it_is_sent() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/posts.json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([{ "id": 1047 }])))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/uploads.json"))
            .respond_with(ResponseTemplate::new(201).set_body_json(json!({ "id": 9 })))
            .expect(0)
            .mount(&server)
            .await;
        let client = BooruClient::new(&server.uri(), "alice", "key");

        let outcome = run(
            &client,
            payload(),
            "danbooru",
            &form(&["1girl"], "", "", ""),
            fast_polling(),
        )
        .await;

        match outcome {
            BooruUploadOutcome::Failed { error } => {
                assert_eq!(error.step, UploadStep::CreateUpload);
                assert_eq!(
                    error.message,
                    "the booru already has this file as post #1047"
                );
                assert_eq!(error.remote_ref, Some("1047".to_string()));
            }
            BooruUploadOutcome::Posted { .. } => panic!("expected a refusal"),
        }
    }

    #[tokio::test]
    async fn a_refused_upload_records_nothing_and_names_its_step() {
        let server = MockServer::start().await;
        no_post_holds_the_file(&server).await;
        Mock::given(method("POST"))
            .and(path("/uploads.json"))
            .respond_with(
                ResponseTemplate::new(422)
                    .set_body_json(json!({ "message": "duplicate of post #1" })),
            )
            .mount(&server)
            .await;
        let client = BooruClient::new(&server.uri(), "alice", "key");

        let outcome = run(
            &client,
            payload(),
            "danbooru",
            &form(&["1girl"], "", "", ""),
            fast_polling(),
        )
        .await;

        match outcome {
            BooruUploadOutcome::Failed { error } => {
                assert_eq!(error.step, UploadStep::CreateUpload);
                assert_eq!(error.message, "duplicate of post #1");
            }
            BooruUploadOutcome::Posted { .. } => panic!("expected a failure"),
        }
    }

    #[tokio::test]
    async fn processing_that_never_completes_names_the_upload_id() {
        let server = MockServer::start().await;
        no_post_holds_the_file(&server).await;
        Mock::given(method("POST"))
            .and(path("/uploads.json"))
            .respond_with(ResponseTemplate::new(201).set_body_json(json!({ "id": 9 })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/uploads/9.json"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({ "status": "processing" })),
            )
            .mount(&server)
            .await;
        let client = BooruClient::new(&server.uri(), "alice", "key");

        let outcome = run(
            &client,
            payload(),
            "danbooru",
            &form(&["1girl"], "", "", ""),
            fast_polling(),
        )
        .await;

        match outcome {
            BooruUploadOutcome::Failed { error } => {
                assert_eq!(error.step, UploadStep::AwaitProcessing);
                assert_eq!(error.remote_ref, Some("9".to_string()));
            }
            BooruUploadOutcome::Posted { .. } => panic!("expected a failure"),
        }
    }

    #[tokio::test]
    async fn a_refused_post_creation_records_nothing() {
        let server = MockServer::start().await;
        no_post_holds_the_file(&server).await;
        Mock::given(method("POST"))
            .and(path("/uploads.json"))
            .respond_with(ResponseTemplate::new(201).set_body_json(json!({ "id": 9 })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/uploads/9.json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "status": "completed",
                "upload_media_assets": [{ "id": 77, "media_asset_id": 4001 }],
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/posts.json"))
            .respond_with(
                ResponseTemplate::new(422).set_body_json(json!({ "message": "invalid rating" })),
            )
            .mount(&server)
            .await;
        let client = BooruClient::new(&server.uri(), "alice", "key");

        let outcome = run(
            &client,
            payload(),
            "danbooru",
            &form(&["1girl"], "", "", ""),
            fast_polling(),
        )
        .await;

        match outcome {
            BooruUploadOutcome::Failed { error } => assert_eq!(error.step, UploadStep::CreatePost),
            BooruUploadOutcome::Posted { .. } => panic!("expected a failure"),
        }
    }
}
