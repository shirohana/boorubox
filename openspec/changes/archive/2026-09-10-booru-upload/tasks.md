## 1. Sites, schema and credentials

- [x] 1.1 `packages/shared` + `packages/app/src-tauri`: add `BooruSite`, `PostRef`,
  `BooruUploadForm`, `BooruUploadOutcome`, `UploadStep`/`UploadStepError`, `CommentaryOutcome` to
  `src/index.ts`, add `posts: PostRef[]` to `ImageRecord`, and mirror all of it in `model.rs`
  (Phase 1 D11: hand-mirrored, change both). Verify: `pnpm -r typecheck` and `cargo test -p
  boorubox` build clean.
- [x] 1.2 `packages/app/src-tauri`: append the `booru_sites` migration (design D1) to `MIGRATIONS`
  in `db.rs`. Verify: a `db.rs` test asserts `user_version == MIGRATIONS.len()` after open and
  that `booru_sites` is present, and the existing `open_applies_schema_v1` style tests still pass.
- [x] 1.3 `packages/app/src-tauri`: new `booru/sites.rs` — list, save (insert or update), delete,
  with the host-slug id derivation and collision suffix of design D2 and the unique base address.
  Verify: unit tests for slug derivation (host with port, uppercase, punctuation), duplicate
  address refused, delete leaves `posts` rows intact.
- [x] 1.4 `packages/app/src-tauri`: new `booru/credentials.rs` — `trait Credentials`, the `keyring`
  implementation with service `"BooruBox"` and account `"<host>/<username>"` (design D7), an
  in-memory implementation for tests, and `AppError::Credential { reason }` in `error.rs`. Verify:
  tests over the in-memory implementation cover set/get/delete and a refusing store; no test
  touches the real keychain.
- [x] 1.5 `packages/app/src-tauri` + `packages/app/src`: commands `booru_site_list`,
  `booru_site_save`, `booru_site_delete`, registered in `lib.rs`, wrapped by
  `src/lib/api/booru.ts`. Verify: mock-runtime command tests in `commands.rs` (save then list
  round-trips; delete removes the site and its credential); `pnpm -r test` passes for the api
  wrapper.
- [x] 1.6 `packages/app/src-tauri`: fill `ImageRecord.posts` in `query.rs` alongside the existing
  tag fill, for search results and single-image reads. Verify: a `query.rs` test inserts a `posts`
  row and asserts the record carries it; a search of a page of images issues no per-image query.

## 2. Upload client and its mocked booru

- [x] 2.1 `packages/app/src-tauri`: add `reqwest` (`multipart`, `json`, `default-tls`), `keyring`,
  and dev-dependency `wiremock` (design D4, D14); create `booru/client.rs` with `BooruClient`,
  the timeout and poll constants, and Basic authentication. Verify: `cargo clippy --all-targets
  -- -D warnings` clean; a `wiremock` smoke test asserts the client sends the expected
  `Authorization` header.
- [x] 2.2 `packages/app/src-tauri`: `create_upload` — `POST /uploads.json` as multipart with
  the file part only, with the source on `post[source]` at post creation (design D3), carrying the `FIXME` that the part names are
  unverified against a real instance. Verify: `wiremock` tests for the accepted case (returns the
  upload id) and the refused case (`UploadStep::CreateUpload` with the booru's message verbatim).
- [x] 2.3 `packages/app/src-tauri`: `await_processing` — poll `GET /uploads/{id}.json` for
  `upload_media_assets[0].id`, with the poll interval injectable so tests do not sleep. Verify:
  `wiremock` tests for completes-on-third-poll, status `error`, and never-completes (returns
  `AwaitProcessing` with the upload id in `remote_ref`).
- [x] 2.4 `packages/app/src-tauri`: `create_post` (`POST /posts.json` with
  `upload_media_asset_id`, `tag_string`, `rating`, `source`) and `set_commentary`
  (`PUT /posts/{id}/artist_commentary/create_or_update.json`, skipped when both fields are empty).
  Verify: `wiremock` tests for post created (returns the post id), post refused
  (`UploadStep::CreatePost`), commentary refused (returns `CommentaryOutcome::Failed`, not an
  error).
- [x] 2.5 `packages/app/src-tauri` + `packages/app/src`: `test_connection` on `GET /profile.json`
  judged by status (design D8), command `booru_site_test`, api wrapper. Verify: `wiremock` tests
  for 200, 401 and a connection refused, each mapping to the three outcomes the `booru-sites` spec
  names.
- [x] 2.6 `packages/app/src-tauri` + `packages/app/src`: command `booru_upload(image_id, site_id,
  form)` that reads the image and its file, runs the four steps, and writes the `posts` row plus
  the `updated_at` bump in one transaction after success (design D5); api wrapper. Verify:
  command tests over a `wiremock` booru assert the `posts` row exists after the happy path, that
  no row exists after each of the four hard failures, and that a commentary failure still leaves
  the row; a missing credential fails before any request is made.

## 3. Upload action and form

- [x] 3.1 `packages/app`: port `extractArtistFromUrl` to `src/lib/domain/artist-from-url.ts`
  (design D11). Verify: `artist-from-url.test.ts` covers Pixiv user, Pixiv artwork, X/Twitter
  handle including the `i`/`home`/`search` exclusions, Fanbox, DeviantArt, ArtStation and a
  non-matching address.
  Built with one departure from verbatim: the fanbox capture class is `[^./]+`, not the
  legacy's `[^.]+`, which put the scheme in the tag (`https://someone_fanbox`). Design D11
  records why, and the legacy's own test — which the design believed did not exist — is what
  documented the quirk.
- [x] 3.2 `packages/app`: `src/lib/components/booru/upload-form.ts` — the prefill rules of design
  D10 as a pure function from `ImageRecord` to form values, and the send-time validation (tags
  present, rating chosen). Verify: `upload-form.test.ts` covers unrated (no rating preselected,
  validation refuses), no page address (source falls back to the image address), and tag sorting.
  `checkUploadForm` is one function, not a validator beside a builder: the refusal message and
  the `BooruUploadForm` come out of the same check, so the button and the message cannot
  disagree about what a sendable form is.
- [x] 3.3 `packages/app`: `src/lib/components/booru/UploadDialog.svelte` — the dialog with tags
  (reusing `src/lib/components/tags/TagInput.svelte` from `tags-and-ratings`), rating, source,
  artist, commentary title and body, and a preview. Verify: observable — opening it on a captured
  image shows every field prefilled per D10 and editing a field does not change the image.
  Hand check: on a captured, rated, tagged image, open `Upload to <site>` from the inspector —
  tags sorted, the rating shown as chosen, the page address as source, an artist for a Pixiv or
  X page, the page title as the commentary title. Change the tags and the rating, cancel, and
  see the inspector's own tags and rating unchanged.
  Probe (2026-09-10, agent-driven app on a scratch copy of the vault): the dialog opened with the image's tag, rating g, the X page as source, `IV70311741` as the artist and the page title as the commentary title. It opens with the tag field focused and its suggestion list already dropped over the form — yours to judge. Cancel-and-compare is yours.
- [x] 3.4 `packages/app`: the Inspector actions row (Slot: Inspector · actions) — `Upload to
  <site>`, a site chooser when more than one is configured, absent when none is configured, with
  a pointer to Settings · Booru. Verify: observable — with zero, one and two sites configured the
  row matches the three `booru-upload` spec scenarios.
  Hand check: with no site configured the row says so and links to Settings · Booru; with one it
  reads `Upload to <name>`; with two it is an `Upload to…` menu listing both. The row is absent
  in /trash — an image on its way out is not one to publish.
  Probe (2026-09-10, agent-driven app on a scratch copy of the vault): with none the inspector read "No booru configured. Add one in Settings · Booru."; with one it offered "Upload to probe-booru". Two sites and /trash are yours.
- [x] 3.5 `packages/app`: submit wiring — in-progress step shown while the upload runs, each
  `UploadStep` failure rendered with the booru's message and its remote link where present, and
  the commentary warning on an otherwise successful post. Verify: observable against the mocked
  responses of task 2.6 driven manually, one per failure step.
  Built short of the task on one point: `booru_upload` runs the four steps behind one call and
  emits nothing while it does, so the step *in progress* cannot be named — only the step that
  failed can. The dialog says what the sequence is doing; a `FIXME` at the site and the handoff
  notes below name the `upload:step` event that would fix it. A processing timeout links
  `<base>/uploads/<id>` from `remoteRef`; no other failure step carries one. A commentary
  failure is read off `BooruUploadOutcome.commentary` and names the post from `outcome.post`.
  Hand check: point a site at a booru that refuses (a wrong address, a duplicate image, a wrong
  key) and see each failure name its step, repeat the booru's own words, and say that nothing
  was recorded.
  Probe (2026-09-10, agent-driven app on a scratch copy of the vault): against `http://nowhere.invalid` the dialog reported "The booru refused the file — error sending request for url (http://nowhere.invalid/uploads.json) — Nothing was recorded against this image." The address and the transport's reason are there; the step label reads as a refusal although the booru never answered — wording yours to judge. A duplicate and a wrong key need a real booru.

## 4. Posted label

- [x] 4.1 `packages/app`: `src/lib/components/booru/PostedLabel.svelte` and its use in the
  Inspector — one entry per site, linking to `<base_url>/posts/<remote_id>`, degrading to the
  stored site key with no link when the site is no longer configured (design D2). Verify:
  observable — an image posted to two sites lists both; removing a site keeps its entry.
  The join and the computed address are `src/lib/components/booru/posted.ts`, covered by
  `posted.test.ts`; the label opens the post in the default browser (`openUrl`), never in this
  webview. The section is absent, not empty, for an image never posted, and is shown in /trash
  too — where an image has been is a fact about it, not an action on it.
  Hand check: an image posted to two sites lists both with their names and post ids; clicking
  one opens that post in the browser. Remove the site in Settings · Booru and the entry stays,
  showing the slug and no link.
- [x] 4.2 `packages/app`: the posted mark on the grid tile (Slot: Grid · tile), following the
  tile's existing hover-overlay rules. Verify: observable — a grid mixing posted and unposted
  images distinguishes them without opening the Inspector.
  A badge beside the rating badge, not a line in the hover overlay: the spec wants the mark read
  at a glance and the overlay is hover-only, so the tile's existing always-visible corner — the
  one the rating already uses, for the same reason — is where it goes. The site is named in the
  badge's title, which is how the rating badge names itself too.
  Hand check: a grid holding both posted and unposted images shows the upload badge only on the
  posted ones, and hovering it names the site and post id.
- [x] 4.3 `packages/app`: withhold the upload action for a site the image already has a post
  record for, showing the record instead (design D12). Verify: observable — with two sites and one
  record, the record shows for the first and the action is offered for the second.
  Hand check: upload an image to the first of two configured sites. Its inspector then shows the
  posted entry for that site and an `Upload to <second site>` action naming only the other one;
  once both have a record, no upload action is offered at all.

## 5. Settings · Booru

- [x] 5.1 `packages/app`: a `Settings · Booru` section (design D13, a sibling of Settings · Rules,
  not a shared section) listing configured sites with name, address, username and credential
  state, plus add, edit and remove. Verify: observable — a site added here is offered in the
  Inspector's upload action; removing it leaves the posted labels standing.
  The list is one store (`src/lib/api/booru.svelte.ts`) read by both this section and the
  inspector's action, so a site added here is offered there without a reload. Removing asks
  first, because it takes the OS credential with it (design D7's shared-entry risk).
  Hand check: add a site, open the library and see it offered in the inspector; remove it and
  see any posted entry for it standing, without a link.
  Probe (2026-09-10, agent-driven app on a scratch copy of the vault): adding a site made the inspector offer it; removing it (confirmed) emptied `booru_sites`. A posted entry surviving removal is yours (needs a real post).
- [x] 5.2 `packages/app`: the site form — API key field (write-only, never read back into the
  form), the insecure-address warning, and the Test connection button showing the three outcomes.
  Verify: observable — testing against a wrong key reports the credential as the problem and
  against an unresolvable host reports the address.
  The key field starts blank on every edit and sends `null` when left blank, so an edit that
  does not retype the key keeps it. A save that rejects reloads the list and, when the submitted
  values are in it, records the rejection as that site's credential problem — the row is written
  before the credential is touched, so a refusal that left no such row (a duplicate address) is
  reported as the message it is, not as a credential.
  Hand check: edit a site without typing a key and confirm uploads still work; type a wrong key
  and Test reports the credential, not the address; point one at `http://nowhere.invalid` and
  Test names the address with the transport's reason.
  Probe (2026-09-10, agent-driven app on a scratch copy of the vault): the `http://` address showed the insecure-address warning in the form; Test on `http://nowhere.invalid` read "Could not reach http://nowhere.invalid — error sending request for url (http://nowhere.invalid/profile.json)". That sentence did not wrap and scrolled the table sideways, taking Test/edit/remove off the card: the address and credential cells now wrap (BooruSection.svelte), at the cost of the address breaking mid-word in the narrow settings column. The wrong-key case needs a real booru.
- [x] 5.3 `packages/app`: credential-refusal surfacing — a site whose credential cannot be read
  lists as unusable with the reason, and choosing it as an upload target reports the same reason
  without contacting the booru. Verify: observable, forced by pointing the credential store at a
  refusing implementation in a dev build; the Rust half is covered by task 1.4's refusing-store
  test.
  Built as the second of the two options the Rust handoff put up, and argued in design D7's
  addendum: the state is surfaced where the credential is actually read — `booru_site_test` and
  `booru_site_save` reject with the reason, and the store remembers it per site for the session,
  so the list reads `Unusable — <reason>` from then on. It reads `Not checked.` before anything
  has looked, because nothing has. The rejected option was probing every listed site with
  `booru_site_test` when the screen opens: it would contact every configured booru to render a
  settings list. The probe that would close the gap honestly is a Rust `Credentials::get` with
  no network behind it — named in the handoff notes below and `FIXME`d in the store.
  Hand check: with a refusing credential store, Test on a site reports the store's reason and
  the row then reads `Unusable — <reason>`; the inspector's upload for that site fails with the
  same reason and the booru is never contacted (nothing in its access log).

## 6. Verification

- [x] 6.1 `mise run check` green — lint, typecheck, `pnpm -r test`, `cargo test`, clippy, builds.
- [x] 6.2 Manual pass against a real Danbooru-compatible instance (see design Open Questions for
  which): confirm the request shapes that no test can prove — the upload's file part name and `post[source]`
  part names on `/uploads.json`, the `upload_media_assets[0].id` field the poll reads,
  `upload_media_asset_id` on `/posts.json`, and the commentary endpoint's body. Verify: one image
  uploaded end to end, its post visible on the instance with the tags and rating sent, and the
  `posts` row present locally. Any mismatch is fixed in the request builder and the `FIXME` from
  task 2.2 is removed. — Done 2026-09-10 against the owner's instance: three images posted
  (posts 1048, 1049, 1050), each holding the library file's exact MD5, the rating, the page
  source, the tags with the artist tag, and the commentary title; three `posts` rows local. The
  part name was the one mismatch (see "Owner's verification pass").
- [ ] 6.3 Manual pass on the failure paths against the same instance: a duplicate image (booru
  refuses the upload), a deliberately wrong API key (credential rejected), and the instance
  stopped mid-upload (processing timeout). Verify: each names its step, and no `posts` row is
  written for any of them. — Duplicate done 2026-09-10: a byte-identical sibling capture was
  refused before the file was sent, "the booru already has this file as post #1048" with the
  post linked, no `posts` row, and post 1048's tags and rating unchanged.
  Hand check: the wrong API key and the stopped instance are still to run — edit the site's key
  in Settings · Booru and upload (expect "The booru rejected the credential"); stop the instance
  after an upload starts (expect the processing-timeout message naming the upload).
- [ ] 6.4 Manual pass on the label: restart the app and confirm the posted label and the withheld
  upload action survive; remove the site and confirm the label remains without a link.

## Owner's verification pass (2026-09-10)

Findings from the owner's first run against their own instance (Danbooru, a 2023-10 build), and
what each became:

- **Upload failed at "authenticate" with `found unpermitted parameter: :file`.** The part name
  the design guessed (`upload[file]`) has been `upload[files][0]` since Danbooru's multi-file
  upload (2022-02-19), and an unpermitted parameter is answered with a 403 — which the client
  classifies as a credential failure. Fixed in the request builder; every other shape was read off
  Danbooru's source at the same time (design D3, Risks) and the client's `FIXME`s are gone. Along
  the way: Danbooru does not refuse a duplicate at the upload, it merges the sent tags and rating
  into the original post at `POST /posts.json` — so the sequence now asks by MD5 first and refuses
  naming the post (design D12, amended with the argument; `booru-upload` spec scenario "Booru
  already holds the file").
- **The tag field must not take the focus when the dialog opens.** The dialog is a confirmation
  of an image the user tagged already; it now opens with the focus on the dialog itself, nothing
  armed (`UploadDialog.svelte`, `onOpenAutoFocus`).
- **A dev build asks the Keychain on every upload.** macOS grants a keychain item to an
  application identity, and an ad-hoc-signed `tauri dev` binary is a new identity on every
  rebuild, so the login-password prompt returns per rebuild — and per read, unless "Always Allow"
  was chosen. Not an app defect (design D7 chose the OS store on purpose); a signed release build
  has a stable identity. Worth knowing before a hand check that runs several uploads.
- **Bulk upload — deferred, not this change.** One image per dialog is enough here. The shape
  the owner expects later: select several images, one upload command, one dialog that previews
  each file with its own tags and rating, and posts them in sequence. That is its own change
  (design D9 already notes a dialog is the shape a later bulk upload would extend), and it needs
  the per-step `upload:step` event the webview handoff lists as the open item, so that a run of
  several can show where it is.

## Handoff notes (Rust)

Groups 1 and 2 are implemented and green (`cargo test`, clippy, `pnpm -r typecheck`, `pnpm -r
test`, lint — see the gate results in the implementing agent's report). Everything below is what
the webview side (groups 3–5) builds against.

### Commands (all registered in `lib.rs`, wrapped in `src/lib/api/booru.ts`)

- `booruSiteList(): Promise<BooruSite[]>` — ordered by name, case-insensitive.
- `booruSiteSave(id: string | null, name: string, baseUrl: string, username: string, apiKey:
  string | null): Promise<BooruSite>` — `id: null` creates; `apiKey: null` leaves whatever
  credential the `(host, username)` account already has untouched (this is what backs the
  write-only API key field in task 5.2 — it should send `null` unless the user typed a new key).
  **The database write always happens before the credential write is attempted.** If the
  credential store then refuses, this call *rejects* with `AppError::Credential`'s message even
  though the site's name/address/username are already saved — on a caught rejection here, reload
  the site list (it will show the new/edited site) and surface the rejection's message as the
  credential problem (task 5.1's "removing it leaves the posted labels standing" pattern; this is
  the equivalent for save).
- `booruSiteDelete(id: string): Promise<void>` — removes the site and its credential; posts already
  recorded against it are untouched (no id column even references `booru_sites` any more once a
  site is gone — the label degrades per `posted-label`).
- `booruSiteTest(id: string): Promise<BooruConnectionTest>` — the three outcomes below. Rejects
  with `AppError::Credential` instead of resolving if the credential itself cannot be read (never
  reaches the network in that case).
- `booruUpload(imageId: string, siteId: string, form: BooruUploadForm):
  Promise<BooruUploadOutcome>` — runs the whole sequence and, on success, has already written the
  `posts` row (nothing further to do besides re-reading the image, or trusting
  `BooruUploadOutcome.post` directly). **Rejects** (promise rejection, not a `'failed'` outcome)
  only for: no library open, image or site not found, and — the one every upload target must be
  prechecked for — a missing or refused credential (`AppError::Credential`). Every failure *of the
  upload sequence itself* (booru rejected the file, processing timed out, etc.) is a normal
  resolved `BooruUploadOutcome` with `outcome: 'failed'`; do not `catch` for those, `switch` on
  `outcome.outcome` instead.

There is no `booru_site` credential-status field and no separate "is this site usable" command:
task 5.3's "site whose credential cannot be read lists as unusable" has no dedicated signal from
`booru_site_list` (that command only reads `booru_sites`'s own columns — never the credential
store, which is the whole point of D7 — a `BooruSite` has no `hasCredential` field). The place a
missing/refused credential actually surfaces is choosing that site as an upload target
(`booruUpload` rejecting) or testing it (`booruSiteTest` rejecting). If task 5.3's Settings-screen
"listed as unusable, with the reason" wants to show this *before* the user tries anything, the
only Rust-backed way today is to call `booruSiteTest` per listed site (which does touch the
network) — there's no side-effect-free check. Flagging this as a real gap between the spec's
wording and what groups 1–2 built: worth a quick decision (call `booruSiteTest` speculatively per
site, or accept that "unusable" only shows up on the first real attempt) rather than silently
picking one while building the Settings screen.

### Types (`packages/shared/src/index.ts`, mirrored in `model.rs`)

- `BooruSite { id, name, baseUrl, username, createdAt, updatedAt }` — no credential field, ever.
- `PostRef { site, remoteId, postedAt }` — `site` is `BooruSite.id`, a slug, not a foreign key; it
  outlives the site row (`posted-label` design).
- `BooruConnectionTest` — tagged union on `status`: `{ status: 'connected' }`,
  `{ status: 'credentialRejected' }`, `{ status: 'unreachable', reason: string }`. The "site
  answered but not usefully" case from design D8 is folded into `unreachable` with the status code
  in `reason` — there is no fourth variant.
- `BooruUploadForm { tags, rating, source, artist, commentaryTitle, commentaryBody }` — `rating` is
  a plain `Rating` (never `null`) because the wire type assumes the webview's own validation (task
  3.2) already refused to send without one; Rust re-checks this anyway (`AppError::BadRequest` if
  `tags` is empty or `rating` isn't one of `g`/`s`/`q`/`e`) and rejects the whole call if it slips
  through — build the form's local editable state as `rating: Rating | null` and only construct a
  `BooruUploadForm` once a rating is chosen. **The `artist` field is folded into `tag_string` by
  Rust** (`booru::upload::run`, appended to `tags` if non-empty and not already present) — there is
  no separate "artist" parameter on any Danbooru call, so the dialog can keep `artist` as its own
  input without needing to also inject it into the tag field.
- `UploadStep` — `'authenticate' | 'createUpload' | 'awaitProcessing' | 'createPost' |
  'commentary'`. **`'authenticate' `is a booru-side 401/403 on any of the four calls, never a local
  credential failure** — a missing/refused credential is `AppError::Credential`, a promise
  rejection, never an `UploadStep`.
- `UploadStepError { step, message, remoteRef? }` — `message` is the booru's own text verbatim
  (task 6.2 will confirm what that text actually looks like against a real instance — right now
  it's a best-effort read of `{message: ...}` / `{errors: ...}` bodies, marked `FIXME` in
  `booru/client.rs`). `remoteRef` is the upload id on a failure of the processing step, absent
  otherwise; a commentary failure carries none — the post it belongs to is `outcome.post`.
- `CommentaryOutcome` — an object tagged on `status`: `{ status: 'skipped' | 'applied' }` or
  `{ status: 'failed', message }` (read `packages/shared/src/index.ts` for the exact shape).
- `BooruUploadOutcome` — tagged on `outcome`: `{ outcome: 'posted', post, commentary }` or
  `{ outcome: 'failed', error }`. Only `'posted'` ever carries a `post`.
- `ImageRecord.posts: PostRef[]` — filled by `ingest::load_records` (see note below), always
  present (empty array, never omitted) even for an image never posted anywhere.

### Where `ImageRecord.posts` is actually filled (task 1.6 correction)

Task 1.6 says "fill `ImageRecord.posts` in `query.rs` alongside the existing tag fill" — by the
time this change was implemented, the tag fill it's referring to had moved to
`ingest::load_records` (in `ingest.rs`), which is what both `query::search` and every single-image
read (`update_tags`, `set_rating`, `thumbnail_path`, etc.) call through. The `posts` fill lives
there instead, for the same reason the tag fill does: one function, one extra statement, no
per-image query, and every caller gets it for free. Nothing in `query.rs` changed for this task.

### Keychain entry naming (design D7, for anyone touching Settings · Booru)

Service is the constant `"BooruBox"`; account is `"<host>/<username>"` where `<host>` is
`booru::sites::host_of(baseUrl)` (the URL's host only — no scheme, no port). This is not surfaced
to the webview as a string anywhere; it only matters if a future screen wants to tell the user
*which* OS-level entry a site is using (e.g. "this matches the key already saved for
danbooru.donmai.us / alice from another library").

### The mocked booru in tests

Every Rust test drives real `BooruClient`/`booru_upload` code against a `wiremock::MockServer`, not
a hand-rolled fake — see `booru/client.rs`, `booru/upload.rs` and the `booru_tests` module in
`commands.rs` for the request/response shapes exercised (happy path, 401, 422 refusal, processing
`error` status, processing timeout, commentary failure). None of this proves the *real* Danbooru
API matches these shapes — see the `FIXME`s in `booru/client.rs` and task 6.2.

### Migration number

This change is schema **v5** (`booru_sites` table only, no changes to any existing table). v1 is
`phase-1-app-mvp`, v2 is `bridge-extension`, v3 is `browse-polish`, v4 is `auto-tag-rules`;
`tags-and-ratings` landed needing no migration of its own. `design.md`'s D1 has been amended to
record this (it originally guessed v4, written before `browse-polish` existed).

### Gate results

`cargo test --manifest-path packages/app/src-tauri/Cargo.toml`: 373 passed, 0 failed. `mise run
clippy`: clean. `pnpm -r typecheck`: clean (required a one-line fix to
`src/lib/domain/image-fixture.ts`'s `img()` fixture, which builds a full `ImageRecord` — it now
sets `posts: []` — this file is outside this agent's assigned ownership but was left non-compiling
by the `ImageRecord.posts` addition, so it was fixed as part of task 1.1). `pnpm --filter
@boorubox/app test` / `--filter @boorubox/shared test`: all green (317 + 1). `mise run lint`: clean
after `mise run format` (ESLint Stylistic wanted the new `index.ts` types reformatted; no manual
style decisions were overridden). Also fixed as a side effect of `ImageRecord` crossing clippy's
`large_enum_variant` threshold: `http::CaptureEvent::Stored` is now `Stored(Box<ImageRecord>)`
(`http/mod.rs`, `http/captures.rs`) — a mechanical box, no behavior change, needed because `posts`
made `ImageRecord` big enough to trip the lint on an unrelated enum.

## Handoff notes (webview)

Groups 3–5 are implemented. `mise run lint`, `mise run typecheck` and `pnpm --filter
@boorubox/app test` are green (340 tests, 30 files). What the next agent should know:

### What was added, and where the decisions landed

- `src/lib/domain/artist-from-url.ts` (+ test) — the legacy rule, one capture narrowed; design
  D11 carries the argument.
- `src/lib/components/booru/upload-form.ts` (+ test) — D10's prefill and the send-time check, in
  one function so the refusal and the form cannot disagree.
- `src/lib/components/booru/posted.ts` (+ test) — the `PostRef` → site join, the computed post
  address, and D12's "which sites are still offered". Read by both the inspector's label and the
  grid tile's badge; nothing else computes a post address.
- `src/lib/api/booru.svelte.ts` (+ test), exported from `src/lib/api/index.ts` as `booruSites` —
  the site list, read once per library and shared by Settings · Booru, the inspector's action and
  the grid tile. Also every piece of per-site session state: the credential refusals (design D7's
  addendum) and the last connection test's outcome. Both are cleared on a library switch and on a
  removal, and neither may live in a component — the section is unmounted around a list that
  outlives it, and the ids are host slugs, so one library's outcome would be shown against
  another's site of the same slug.
- `src/lib/api/opener.ts` — `openExternal(url)`, the one way this app hands an address to the
  browser. It answers with the reason it failed instead of throwing, so a link that did nothing
  says why rather than looking dead.
- `src/lib/components/booru/{UploadDialog,UploadAction,PostedLabel,BooruSection,SiteForm}.svelte`.
- `src/lib/components/common/ConfirmDialog.svelte` — the app's one confirmation, generalised from
  `library/ConfirmDeleteDialog.svelte` (now gone) and used by permanent deletion, rule deletion
  and site removal.
- Touched: `Inspector.svelte` (posted section, upload action, and the in-place record replace an
  upload needs), `Lightbox.svelte` and `LibraryScreen.svelte` (which no longer thread anything for
  an upload), `ImageCard.svelte` (the posted badge, beside the rating badge),
  `rules/RulesTable.svelte` (the shared confirmation), `routes/settings/+page.svelte`.

### What Rust would have to add for the two places this fell short

Neither is a design change; both are a command or an event and a field.

1. **A credential probe with no network behind it** — `Credentials::get` for a site (or one call
   answering for the whole store), so Settings · Booru can list a site as unusable before anything
   is attempted. Today the state only appears once `booru_site_test` or `booru_site_save` has been
   refused; `booru_site_list` never touches the store, by design. See design D7's addendum and the
   `FIXME` in `src/lib/api/booru.svelte.ts`.
2. **An `upload:step` event from `booru_upload`** — the same shape the import, export and rules
   runs already emit, carrying the `UploadStep` about to run. Today only a *failure* names a step,
   so the dialog describes the sequence rather than its position in it; the `booru-upload`
   requirement has been amended to ask for what the form does do, and design D6 records why.
   `FIXME` in `UploadDialog.svelte`. **This is the open item of this change's webview half.**
3. **A credential rejection the webview can match on** — `AppError` arrives as a bare string, so a
   `booruUpload` rejection cannot be told apart from "no library open" or "image not found". The
   dialog therefore shows it and attributes it to no site, and only `booru_site_test` and
   `booru_site_save` feed `booruSites.refused()` (design D7's addendum, narrowed). A tagged error
   — a discriminant on the wire — would let the upload mark its own site unusable.

### Contract notes worth keeping

- `booruUpload` rejecting is never a step failure: `switch` on `outcome.outcome`, and treat a
  rejection as `AppError` (no library, not found, credential). The dialog does both and shows them
  differently — a rejection is not attributed to a step.
- A commentary failure is `BooruUploadOutcome.commentary`, `{ status: 'failed', message }`, and
  the post it belongs to is `outcome.post` — no `remoteRef` crosses the wire for it. The only
  `UploadStepError.remoteRef` the dialog links is `awaitProcessing`'s upload id
  (`<base>/uploads/<id>`); any other step's reference is shown as text, since nothing says what
  address it would be.
- A successful upload writes the `posts` row in Rust, outside `results`, and answers with the
  `PostRef` it wrote. `onposted(post)` carries it to the Inspector, which puts it into the loaded
  record with `results.replace()` — `tags-and-ratings` design D10: a write replaces the record,
  the search is never re-run. Re-running it here would clear the rows out from under the dialog
  that is still reporting the post, empty the Inspector and unmount the dialog with it, so the
  success panel — the post's link, the commentary warning — would never be read.
- `PostRef.site` is a slug, and a post whose site is gone is still shown, without a link. Nothing
  in the webview may treat it as a foreign key.
