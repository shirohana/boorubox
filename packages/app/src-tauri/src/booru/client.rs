//! The Danbooru sequence, plus the connection test, over a plain `reqwest`
//! client (`booru-upload` design D3, D4, D8, D12).
//!
//! Every request and response shape here is read off Danbooru's own source
//! (`UploadPolicy`, `Upload#files=`, `PostsController#create`,
//! `ArtistCommentariesController`, `ApplicationController#render_error_page`)
//! as of the multi-file upload change of 2022-02-19, which is the oldest
//! instance the part name below can reach; an instance older than that took a
//! single `upload[file]` and is out of scope. The owner's own instance
//! (a 2023-10 build) is what task 6.2 ran against.

use std::time::{Duration, Instant};

use reqwest::StatusCode;
use serde_json::Value;

use crate::model::{BooruConnectionTest, UploadStep, UploadStepError};

/// Time to establish the TCP/TLS connection, client-wide (design D4).
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
/// Every call but the one that carries the file (design D4).
const METADATA_TIMEOUT: Duration = Duration::from_secs(30);
/// `create_upload` carries the file's bytes; a slow uplink is not a hung
/// server (design D4).
const UPLOAD_TIMEOUT: Duration = Duration::from_secs(120);
/// Kept from the legacy client verbatim (design D4): there is no better
/// evidence than "what the previous implementation used", and changing these
/// without a real instance to test against would be a guess on top of a guess.
pub const POLL_ATTEMPTS: u32 = 20;
pub const POLL_INTERVAL: Duration = Duration::from_secs(2);
/// The bound on the whole processing phase, wall clock (design D4). The
/// attempt count bounds nothing on its own: every poll also carries a request
/// timeout, so a booru that accepts the file and then answers nothing would
/// hold the upload for attempts x (interval + timeout). This is what stops it.
pub const POLL_DEADLINE: Duration = Duration::from_secs(60);

/// How long the processing phase may take in total, and how long to wait
/// between polls. A parameter rather than the constants directly so tests can
/// drive the whole phase in milliseconds (design D14).
#[derive(Clone, Copy, Debug)]
pub struct PollSchedule {
    pub interval: Duration,
    pub deadline: Duration,
}

impl Default for PollSchedule {
    fn default() -> Self {
        PollSchedule {
            interval: POLL_INTERVAL,
            deadline: POLL_DEADLINE,
        }
    }
}

/// One booru account: a base address, a username, and the API key already
/// read out of the credential store (design D4). One of these is built per
/// upload and per connection test, and the `reqwest::Client` it holds is
/// built with it: the connection pool is reused across the calls of a single
/// sequence and dropped with it. Nothing longer-lived holds one, because
/// nothing talks to a booru outside one sequence — the day something does,
/// the pool belongs in the app state and this struct borrows it.
pub struct BooruClient {
    http: reqwest::Client,
    base_url: String,
    username: String,
    api_key: String,
}

impl BooruClient {
    pub fn new(base_url: &str, username: &str, api_key: &str) -> Self {
        // No redirects: Danbooru answers a duplicate on `POST /posts.json`
        // with a redirect to the original post (design D12). Followed, that
        // fetches an HTML page under the credential and the failure would
        // read as an unreadable response instead of the 3xx it was.
        let http = reqwest::Client::builder()
            .connect_timeout(CONNECT_TIMEOUT)
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .expect("a reqwest client with a connect timeout and no redirects must build");
        BooruClient {
            http,
            base_url: base_url.trim_end_matches('/').to_string(),
            username: username.to_string(),
            api_key: api_key.to_string(),
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}{path}", self.base_url)
    }

    /// `GET /posts.json?tags=md5:<md5>`: the id of the post already holding
    /// a file with this checksum, if any (design D12). Asked before the file
    /// is sent: Danbooru accepts a duplicate file without complaint and only
    /// `POST /posts.json` notices, and it answers that by merging the sent
    /// rating and tags into the original post — a change to a post the user
    /// never meant to touch. Same `Authenticate` classification as every
    /// other call; any other refusal is the upload step's.
    pub async fn find_post_by_md5(
        &self,
        md5: &str,
    ) -> std::result::Result<Option<String>, UploadStepError> {
        let step = UploadStep::CreateUpload;
        let response = self
            .http
            .get(self.url("/posts.json"))
            .query(&[("tags", format!("md5:{md5}")), ("limit", "1".to_string())])
            .basic_auth(&self.username, Some(&self.api_key))
            .timeout(METADATA_TIMEOUT)
            .send()
            .await
            .map_err(|error| transport_error(step, &error))?;

        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(classify(status, step, booru_message(&body)));
        }
        let posts: Value = serde_json::from_str(&body).map_err(|error| UploadStepError {
            step,
            message: format!("the booru's response could not be read: {error}"),
            remote_ref: None,
        })?;
        Ok(posts
            .as_array()
            .and_then(|posts| posts.first())
            .and_then(|post| post.get("id"))
            .map(id_as_string))
    }

    /// `POST /uploads.json`, the file's bytes as the whole multipart body
    /// (design D3 — no `upload[source]`; the booru is never asked to fetch
    /// anything). Answers with the upload id `await_processing` polls.
    pub async fn create_upload(
        &self,
        filename: &str,
        mime: &str,
        bytes: Vec<u8>,
    ) -> std::result::Result<String, UploadStepError> {
        let step = UploadStep::CreateUpload;
        // `UploadPolicy` permits `files` as an index-keyed hash (`Upload#files=`
        // reads `files.map { |_index, file| … }`), so one file is
        // `upload[files][0]`. The singular `upload[file]` is refused as an
        // unpermitted parameter — with a 403, which reads as a credential
        // failure — on every instance since multi-file upload (2022-02-19).
        let part = reqwest::multipart::Part::bytes(bytes)
            .file_name(filename.to_string())
            .mime_str(mime)
            .map_err(|error| transport_error(step, &error))?;
        let form = reqwest::multipart::Form::new().part("upload[files][0]", part);

        let response = self
            .http
            .post(self.url("/uploads.json"))
            .basic_auth(&self.username, Some(&self.api_key))
            .timeout(UPLOAD_TIMEOUT)
            .multipart(form)
            .send()
            .await
            .map_err(|error| transport_error(step, &error))?;

        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(classify(status, step, booru_message(&body)));
        }
        read_id_field(&body, "id", step)
    }

    /// Poll `GET /uploads/{id}.json` until `upload_media_assets[0]` carries
    /// a `media_asset_id`, the booru reports `status: "error"`, or the
    /// schedule is spent — whichever of `POLL_ATTEMPTS` polls and the deadline
    /// runs out first (design D4). The `upload_media_assets` row exists from
    /// the moment the upload does, with `media_asset_id` null until the file
    /// is processed: its presence is what "finished" means, and its `id` is
    /// what `create_post` needs. A file upload is processed inside the create
    /// call itself, so the first poll usually answers. `schedule` is a
    /// parameter, not the constants directly, so tests can drive this without
    /// sleeping for real (task 2.3).
    ///
    /// Every failure here carries the upload id (design D6): the upload
    /// exists on the booru whatever went wrong afterwards, so the message can
    /// always point at `<base>/uploads/<id>`.
    pub async fn await_processing(
        &self,
        upload_id: &str,
        schedule: PollSchedule,
    ) -> std::result::Result<String, UploadStepError> {
        let step = UploadStep::AwaitProcessing;
        let with_upload_ref = |mut error: UploadStepError| {
            error.remote_ref = Some(upload_id.to_string());
            error
        };
        let deadline = Instant::now() + schedule.deadline;
        for attempt in 0..POLL_ATTEMPTS {
            if attempt > 0 {
                tokio::time::sleep(schedule.interval).await;
            }
            // The deadline caps each poll's own timeout, not just the number
            // of polls: a booru that accepts the connection and then answers
            // nothing would otherwise hold `METADATA_TIMEOUT` per attempt.
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                break;
            }

            let response = match self
                .http
                .get(self.url(&format!("/uploads/{upload_id}.json")))
                .basic_auth(&self.username, Some(&self.api_key))
                .timeout(remaining.min(METADATA_TIMEOUT))
                .send()
                .await
            {
                Ok(response) => response,
                // A poll cut short by the deadline is the phase running out,
                // not a transport fault: fall through to the message that
                // says so.
                Err(_) if Instant::now() >= deadline => break,
                Err(error) => return Err(with_upload_ref(transport_error(step, &error))),
            };

            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            if !status.is_success() {
                return Err(with_upload_ref(classify(
                    status,
                    step,
                    booru_message(&body),
                )));
            }

            let json: Value = serde_json::from_str(&body).unwrap_or(Value::Null);
            if json.get("status").and_then(Value::as_str) == Some("error") {
                return Err(UploadStepError {
                    step,
                    message: booru_message(&body),
                    remote_ref: Some(upload_id.to_string()),
                });
            }
            let processed_asset = json
                .get("upload_media_assets")
                .and_then(Value::as_array)
                .and_then(|assets| assets.first())
                .filter(|asset| asset.get("media_asset_id").is_some_and(|id| !id.is_null()));
            if let Some(upload_media_asset_id) = processed_asset
                .and_then(|asset| asset.get("id"))
                .map(id_as_string)
            {
                return Ok(upload_media_asset_id);
            }
        }

        Err(UploadStepError {
            step,
            message: "the booru had not finished processing the file within the waiting period"
                .to_string(),
            remote_ref: Some(upload_id.to_string()),
        })
    }

    /// `POST /posts.json` (design D3): `upload_media_asset_id` top-level, the
    /// post's own attributes nested under `post[...]` the way Danbooru's
    /// Rails parameters take them. Answers with the post id.
    pub async fn create_post(
        &self,
        media_asset_id: &str,
        tag_string: &str,
        rating: &str,
        source: &str,
    ) -> std::result::Result<String, UploadStepError> {
        let step = UploadStep::CreatePost;
        let response = self
            .http
            .post(self.url("/posts.json"))
            .basic_auth(&self.username, Some(&self.api_key))
            .timeout(METADATA_TIMEOUT)
            .form(&[
                ("upload_media_asset_id", media_asset_id),
                ("post[tag_string]", tag_string),
                ("post[rating]", rating),
                ("post[source]", source),
            ])
            .send()
            .await
            .map_err(|error| transport_error(step, &error))?;

        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(classify(status, step, booru_message(&body)));
        }
        read_id_field(&body, "id", step)
    }

    /// `PUT /posts/{id}/artist_commentary/create_or_update.json` (design D3):
    /// the post is named by the path alone, and the commentary's fields nest
    /// under `artist_commentary[...]`. Called only when a title or a body was
    /// given — the caller decides that, this always sends.
    pub async fn set_commentary(
        &self,
        post_id: &str,
        title: &str,
        body_text: &str,
    ) -> std::result::Result<(), UploadStepError> {
        let step = UploadStep::Commentary;
        let response = self
            .http
            .put(self.url(&format!(
                "/posts/{post_id}/artist_commentary/create_or_update.json"
            )))
            .basic_auth(&self.username, Some(&self.api_key))
            .timeout(METADATA_TIMEOUT)
            .form(&[
                ("artist_commentary[original_title]", title),
                ("artist_commentary[original_description]", body_text),
            ])
            .send()
            .await
            .map_err(|error| transport_error(step, &error))?;

        let status = response.status();
        if status.is_success() {
            return Ok(());
        }
        let body = response.text().await.unwrap_or_default();
        Err(classify(status, step, booru_message(&body)))
    }

    /// `GET /profile.json`, judged by status code alone (design D8): the
    /// cheapest authenticated read, with no side effect on the booru.
    pub async fn test_connection(&self) -> BooruConnectionTest {
        let result = self
            .http
            .get(self.url("/profile.json"))
            .basic_auth(&self.username, Some(&self.api_key))
            .timeout(METADATA_TIMEOUT)
            .send()
            .await;

        match result {
            Ok(response) if response.status().is_success() => BooruConnectionTest::Connected,
            Ok(response)
                if matches!(
                    response.status(),
                    StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN
                ) =>
            {
                BooruConnectionTest::CredentialRejected
            }
            // "Answered but not usefully" (design D8) has no scenario of its
            // own in the `booru-sites` spec — folded into `Unreachable` with
            // the status named, which is what the spec's "includes the reason
            // reported by the transport" already promises for this case.
            Ok(response) => BooruConnectionTest::Unreachable {
                reason: format!("the site answered with status {}", response.status()),
            },
            Err(error) => BooruConnectionTest::Unreachable {
                reason: error.to_string(),
            },
        }
    }
}

fn transport_error(step: UploadStep, error: &reqwest::Error) -> UploadStepError {
    UploadStepError {
        step,
        message: error.to_string(),
        remote_ref: None,
    }
}

/// 401/403 always means the credential, whichever of the four calls returned
/// it (design D6's "Credential rejected" scenario is phrased over the whole
/// sequence, not one call) — every other status keeps the step it happened on.
fn classify(status: StatusCode, step: UploadStep, message: String) -> UploadStepError {
    let step = if matches!(status, StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN) {
        UploadStep::Authenticate
    } else {
        step
    };
    UploadStepError {
        step,
        message,
        remote_ref: None,
    }
}

/// The booru's own message out of an error body, never a paraphrase (design
/// D6). Danbooru has three shapes: an exception page is `{ success, error:
/// <class>, message }`; a validation refusal is `{ errors: { field: [..] } }`;
/// an upload that failed processing is the upload record itself, with the
/// reason in its `error` column. `error` is tried last because on the first
/// shape it holds the exception class, not the words.
fn booru_message(body: &str) -> String {
    if let Ok(json) = serde_json::from_str::<Value>(body) {
        if let Some(message) = json.get("message").and_then(Value::as_str) {
            return message.to_string();
        }
        if let Some(errors) = json.get("errors") {
            return errors.to_string();
        }
        if let Some(error) = json.get("error").and_then(Value::as_str) {
            return error.to_string();
        }
    }
    if body.trim().is_empty() {
        "the booru gave no further detail".to_string()
    } else {
        body.trim().to_string()
    }
}

/// A JSON id field as a string regardless of whether the booru sent it as a
/// number or a string — Danbooru's own APIs are inconsistent about this
/// across endpoints and versions.
fn id_as_string(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        other => other.to_string(),
    }
}

fn read_id_field(
    body: &str,
    field: &str,
    step: UploadStep,
) -> std::result::Result<String, UploadStepError> {
    let json: Value = serde_json::from_str(body).map_err(|error| UploadStepError {
        step,
        message: format!("the booru's response could not be read: {error}"),
        remote_ref: None,
    })?;
    json.get(field)
        .map(id_as_string)
        .ok_or_else(|| UploadStepError {
            step,
            message: format!("the booru's response had no {field:?} field"),
            remote_ref: None,
        })
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use wiremock::matchers::{basic_auth, body_string_contains, method, path, query_param};
    use wiremock::{Match, Mock, MockServer, Request, ResponseTemplate};

    use super::*;

    fn client(server: &MockServer) -> BooruClient {
        BooruClient::new(&server.uri(), "alice", "secret-key")
    }

    /// A processing phase that finishes in milliseconds: the suite must not
    /// sleep for the real interval, nor hang for the real deadline on the
    /// polls that are deliberately never satisfied.
    fn fast_polling() -> PollSchedule {
        PollSchedule {
            interval: Duration::from_millis(1),
            deadline: Duration::from_secs(30),
        }
    }

    /// The whole form-encoded body, decoded back into pairs and compared as a
    /// set. Every request this client sends carries names no instance has yet
    /// confirmed (task 6.2), so the one thing the suite can hold is that the
    /// names in the design are the names on the wire: a renamed, dropped or
    /// extra field fails the mock here rather than a real booru later.
    struct FormFields(Vec<(String, String)>);

    fn form_fields(pairs: &[(&str, &str)]) -> FormFields {
        FormFields(
            pairs
                .iter()
                .map(|(name, value)| ((*name).to_string(), (*value).to_string()))
                .collect(),
        )
    }

    impl Match for FormFields {
        fn matches(&self, request: &Request) -> bool {
            let body = String::from_utf8_lossy(&request.body);
            let Ok(parsed) = reqwest::Url::parse(&format!("http://form.invalid/?{body}")) else {
                return false;
            };
            let mut actual: Vec<(String, String)> = parsed
                .query_pairs()
                .map(|(name, value)| (name.into_owned(), value.into_owned()))
                .collect();
            actual.sort();
            let mut expected = self.0.clone();
            expected.sort();
            actual == expected
        }
    }

    #[tokio::test]
    async fn create_upload_sends_basic_auth_and_the_file() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/uploads.json"))
            .and(basic_auth("alice", "secret-key"))
            // The part name is the one thing about this request a real
            // instance rejects outright: `upload[file]` was a 403.
            .and(body_string_contains("name=\"upload[files][0]\""))
            .and(body_string_contains("filename=\"a.png\""))
            .respond_with(ResponseTemplate::new(201).set_body_json(json!({ "id": 9 })))
            .expect(1)
            .mount(&server)
            .await;

        let id = client(&server)
            .create_upload("a.png", "image/png", vec![1, 2, 3])
            .await
            .unwrap();

        assert_eq!(id, "9");
    }

    #[tokio::test]
    async fn create_upload_reports_the_booru_message_verbatim_on_refusal() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/uploads.json"))
            .respond_with(
                ResponseTemplate::new(422)
                    .set_body_json(json!({ "message": "duplicate of post #123" })),
            )
            .mount(&server)
            .await;

        let error = client(&server)
            .create_upload("a.png", "image/png", vec![1])
            .await
            .unwrap_err();

        assert_eq!(error.step, UploadStep::CreateUpload);
        assert_eq!(error.message, "duplicate of post #123");
    }

    #[tokio::test]
    async fn create_upload_names_authentication_when_the_booru_answers_unauthorized() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/uploads.json"))
            .respond_with(ResponseTemplate::new(401).set_body_json(json!({ "message": "bad key" })))
            .mount(&server)
            .await;

        let error = client(&server)
            .create_upload("a.png", "image/png", vec![1])
            .await
            .unwrap_err();

        assert_eq!(error.step, UploadStep::Authenticate);
    }

    #[tokio::test]
    async fn find_post_by_md5_asks_for_the_checksum_and_reads_the_post_id() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/posts.json"))
            .and(query_param("tags", "md5:5289df737df57326fcdd22597afb1fac"))
            .and(basic_auth("alice", "secret-key"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(
                    json!([{ "id": 1047, "md5": "5289df737df57326fcdd22597afb1fac" }]),
                ),
            )
            .mount(&server)
            .await;

        let found = client(&server)
            .find_post_by_md5("5289df737df57326fcdd22597afb1fac")
            .await
            .unwrap();

        assert_eq!(found, Some("1047".to_string()));
    }

    #[tokio::test]
    async fn find_post_by_md5_answers_none_for_an_unknown_checksum() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/posts.json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
            .mount(&server)
            .await;

        let found = client(&server)
            .find_post_by_md5("5289df737df57326fcdd22597afb1fac")
            .await
            .unwrap();

        assert_eq!(found, None);
    }

    #[tokio::test]
    async fn await_processing_completes_on_the_third_poll() {
        let server = MockServer::start().await;
        // The row is there from the first poll; only its `media_asset_id`
        // says the file has been processed.
        Mock::given(method("GET"))
            .and(path("/uploads/9.json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "status": "processing",
                "upload_media_assets": [{ "id": 77, "media_asset_id": null }],
            })))
            .up_to_n_times(2)
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

        let media_asset_id = client(&server)
            .await_processing("9", fast_polling())
            .await
            .unwrap();

        assert_eq!(media_asset_id, "77");
    }

    /// The failed upload is answered as the upload record itself, its reason
    /// in the `error` column — not the `message` of an exception page.
    #[tokio::test]
    async fn await_processing_reports_an_error_status_with_the_upload_id() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/uploads/9.json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "status": "error",
                "error": "unsupported file",
                "upload_media_assets": [{ "id": 77, "media_asset_id": null }],
            })))
            .mount(&server)
            .await;

        let error = client(&server)
            .await_processing("9", fast_polling())
            .await
            .unwrap_err();

        assert_eq!(error.step, UploadStep::AwaitProcessing);
        assert_eq!(error.message, "unsupported file");
        assert_eq!(error.remote_ref, Some("9".to_string()));
    }

    #[tokio::test]
    async fn await_processing_times_out_naming_the_upload_id() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/uploads/9.json"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({ "status": "processing" })),
            )
            .mount(&server)
            .await;

        let error = client(&server)
            .await_processing("9", fast_polling())
            .await
            .unwrap_err();

        assert_eq!(error.step, UploadStep::AwaitProcessing);
        assert_eq!(error.remote_ref, Some("9".to_string()));
    }

    /// Design D4: the phase is bounded by wall clock, not by attempts alone.
    /// A booru that accepts the connection and then never answers used to
    /// cost `POLL_ATTEMPTS` x `METADATA_TIMEOUT`; the deadline is what makes
    /// the stated bound true.
    #[tokio::test]
    async fn await_processing_gives_up_on_the_deadline_when_a_poll_is_never_answered() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/uploads/9.json"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_delay(Duration::from_secs(30))
                    .set_body_json(json!({ "status": "processing" })),
            )
            .mount(&server)
            .await;

        let started = Instant::now();
        let error = client(&server)
            .await_processing(
                "9",
                PollSchedule {
                    interval: Duration::from_millis(1),
                    deadline: Duration::from_millis(200),
                },
            )
            .await
            .unwrap_err();

        assert_eq!(error.step, UploadStep::AwaitProcessing);
        assert_eq!(error.remote_ref, Some("9".to_string()));
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "the deadline must end the phase, not the per-poll timeout"
        );
    }

    #[tokio::test]
    async fn create_post_sends_the_form_fields_and_reads_the_post_id() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/posts.json"))
            .and(form_fields(&[
                ("upload_media_asset_id", "77"),
                ("post[tag_string]", "1girl blue_sky"),
                ("post[rating]", "s"),
                ("post[source]", "https://example.test/p"),
            ]))
            .respond_with(ResponseTemplate::new(201).set_body_json(json!({ "id": 555 })))
            .mount(&server)
            .await;

        let id = client(&server)
            .create_post("77", "1girl blue_sky", "s", "https://example.test/p")
            .await
            .unwrap();

        assert_eq!(id, "555");
    }

    /// Danbooru's answer to a duplicate at this step is a redirect to the
    /// original post. Unfollowed, it is a refusal of the post step like any
    /// other, not a page fetched under the credential.
    #[tokio::test]
    async fn create_post_does_not_follow_a_redirect() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/posts.json"))
            .respond_with(ResponseTemplate::new(302).insert_header("Location", "/posts/1047"))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/posts/1047"))
            .respond_with(ResponseTemplate::new(200).set_body_string("<html>a post page</html>"))
            .expect(0)
            .mount(&server)
            .await;

        let error = client(&server)
            .create_post("77", "1girl", "s", "")
            .await
            .unwrap_err();

        assert_eq!(error.step, UploadStep::CreatePost);
    }

    #[tokio::test]
    async fn create_post_reports_its_own_step_on_refusal() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/posts.json"))
            .respond_with(
                ResponseTemplate::new(422).set_body_json(json!({ "message": "invalid rating" })),
            )
            .mount(&server)
            .await;

        let error = client(&server)
            .create_post("77", "1girl", "s", "")
            .await
            .unwrap_err();

        assert_eq!(error.step, UploadStep::CreatePost);
        assert_eq!(error.message, "invalid rating");
    }

    #[tokio::test]
    async fn commentary_succeeds_on_a_2xx() {
        let server = MockServer::start().await;
        Mock::given(method("PUT"))
            .and(path("/posts/555/artist_commentary/create_or_update.json"))
            // The post is named by the path alone: no `post_id` field rides
            // along in the body (design D3).
            .and(form_fields(&[
                ("artist_commentary[original_title]", "a title"),
                ("artist_commentary[original_description]", "a body"),
            ]))
            .respond_with(ResponseTemplate::new(200))
            .mount(&server)
            .await;

        client(&server)
            .set_commentary("555", "a title", "a body")
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn commentary_reports_its_own_step_on_refusal() {
        let server = MockServer::start().await;
        Mock::given(method("PUT"))
            .and(path("/posts/555/artist_commentary/create_or_update.json"))
            .respond_with(
                ResponseTemplate::new(422).set_body_json(json!({ "message": "title too long" })),
            )
            .mount(&server)
            .await;

        let error = client(&server)
            .set_commentary("555", "a title", "a body")
            .await
            .unwrap_err();

        assert_eq!(error.step, UploadStep::Commentary);
        assert_eq!(error.message, "title too long");
    }

    #[tokio::test]
    async fn test_connection_reports_connected_on_a_2xx() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/profile.json"))
            .and(basic_auth("alice", "secret-key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "id": 1 })))
            .mount(&server)
            .await;

        assert_eq!(
            client(&server).test_connection().await,
            BooruConnectionTest::Connected
        );
    }

    #[tokio::test]
    async fn test_connection_reports_credential_rejected_on_401() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/profile.json"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&server)
            .await;

        assert_eq!(
            client(&server).test_connection().await,
            BooruConnectionTest::CredentialRejected
        );
    }

    #[tokio::test]
    async fn test_connection_reports_unreachable_when_nothing_answers() {
        // A dropped server: nothing is listening on this address any more.
        let server = MockServer::start().await;
        let uri = server.uri();
        drop(server);

        let outcome = BooruClient::new(&uri, "alice", "secret-key")
            .test_connection()
            .await;

        assert!(matches!(outcome, BooruConnectionTest::Unreachable { .. }));
    }
}
