## Context

See proposal.md — Why. What shapes the approach:

- **The `posts` relation already exists**, created empty by Phase 1 D2:
  `posts (image_id, site TEXT, remote_id TEXT, posted_at INTEGER, PK(image_id, site))`. This
  change is the first writer. Its columns are not up for redesign; what has to be decided is what
  goes in `site`.
- **The legacy flow is the only reference for Danbooru's API, and it was never run.** Legacy
  `HANDOFF.md` lists Danbooru upload under "Not fully tested". So `src/viewer/danbooru.ts` tells
  us the *shape* of the sequence — `POST /uploads.json`, poll `GET /uploads/{id}.json` for
  `upload_media_assets[0].id`, `POST /posts.json` with `upload_media_asset_id`, then
  `PUT /posts/{id}/artist_commentary/create_or_update.json` — and nothing about whether those
  request bodies are accepted. Every request shape in this change is therefore *to be confirmed
  against a real instance*, and there is a task for it (tasks 6.2).
- **The owner's decision of 2026-09-06**: credentials in the OS keychain, upload in Rust, the
  `posts` row written in the same transaction as success. Not re-opened here; D3 and D5 record
  what follows from it.
- **The frame is fixed.** app-shell's slot map gives this change three slots — Inspector ·
  actions, Grid · tile, and a Settings section — and no licence to add regions.
- **This change lands last** in the Phase 2 sequence (app-shell → bridge-extension →
  tags-and-ratings → selection-and-bulk → trash → auto-tag-rules → booru-upload), which is why D1
  can refuse to pin a schema number.

## Goals / Non-Goals

**Goals:**

- One upload path, in Rust, that cannot report success without a post existing.
- Failures that name the step, so "the booru rejected the file" and "the network is down" are
  never the same message.
- A credential that never enters the library folder or the settings file, on any platform, with
  no silent fallback.
- Tests that exercise every step and every failure without a booru.

**Non-Goals:**

- Modelling Danbooru's API beyond the four calls the flow needs. No client library, no typed
  coverage of endpoints nothing calls.
- Queueing, retrying or resuming uploads. One upload at a time, driven by an open dialog.
- Making the upload work for booru software other than Danbooru-compatible instances.

## Decisions

**D1. The migration appends one entry; the version number is read from the list, not written into
this document.**

Schema history so far: v1 is Phase 1 (`images`/`tags`/`image_tags`/`posts`/`images_fts`), v2 is
`bridge-extension` (`images.adapter_json`). The ordering rule the addendum fixes is that each
Phase 2 change that needs a migration appends the next entry in landing order — v3 to
`tags-and-ratings` if it needs one, then `auto-tag-rules` (`rules`, `notes`), then this change.
This was written with only v1 and v2 landed, so "v4" was the honest guess at the time: two more
Phase 2 siblings were still ahead of this one in the sequence, and only one of them was known to
need a migration.

**Actual: v5**, read from `MIGRATIONS` in `db.rs` at implementation time rather than guessed
again: `browse-polish` landed between this doc being written and this change being implemented and
took v3 for `images.file_modified_at` — a sibling this addendum's own list did not yet know about,
since it was not in the Phase 2 sequence named above. `auto-tag-rules` then took v4 (`rules`,
`notes`) as expected, and `tags-and-ratings` landed needing no migration at all, exactly as
guessed. So the count is right (two siblings landed a migration before this one), but the second
of them was a different change than the one this paragraph originally named — the guess about
*how many* held; the guess about *which two* did not, because `browse-polish` was not yet planned
when it was made.

Rather than pin `5` in a test literal, this change appends `SCHEMA_BOORU_SITES` to `MIGRATIONS`
and its test asserts `user_version == MIGRATIONS.len()` plus the presence of `booru_sites`. Then
a sibling landing first costs nothing — which is exactly what just happened.

The alternative the brief raises — replace the `user_version` counter with named migrations so
order stops mattering — was considered and rejected. It reverses Phase 1 D2's runner, which has
shipped: every existing library carries a `user_version` and no names table, so the rewrite would
itself need a migration that maps counts to names, and it would land in the *last* change of the
phase, for the benefit of changes that have already landed. The content here is order-independent
anyway: `booru_sites` is a fresh table that references nothing a sibling migration touches. A
counter is the right runner while migrations are a single linear list maintained in one repo; the
day two branches need to add migrations concurrently is the day to revisit it, and that day is
not this change.

```
booru_sites (id TEXT PRIMARY KEY, name TEXT NOT NULL, base_url TEXT NOT NULL UNIQUE,
             username TEXT NOT NULL, created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL)
```

**D2. `posts.site` holds `booru_sites.id`, and that id is a slug of the site's host — not a
foreign key, not a display name.**

Three things want to be in that column and only one fits: a display name (breaks when renamed), a
row reference (breaks when the site is removed — and `posted-label` requires the record to
outlive the configuration), or a stable key. So: `id` is derived once, at site creation, from the
base address's host (lowercased, non-alphanumerics collapsed to `-`, suffixed `-2` on collision
within the library) and never changes afterwards. The label joins `posts.site` to `booru_sites`
for the display name and base address; with no matching row it shows the slug and omits the link,
which is exactly what the spec permits. No `REFERENCES` clause: a foreign key with `ON DELETE
CASCADE` would delete the history, and one without would block removing a site the user is done
with.

The post's address is `<base_url>/posts/<remote_id>`, computed, never stored — one source of
truth for where a site lives.

**The tile's mark is always drawn, not part of the hover overlay, and `posted-label` has been
amended to say so.** The requirement's "SHALL follow the same hover-overlay rules as the tile's
other information" was right about the tile's rule and right to invoke it: `library-browse` puts a
tile's facts — its title, its date — under the pointer and nowhere else, because the tile is the
image and a caption strip would compete with it. What the sentence did not reconcile is its own
scenario one line below, "the posted ones are distinguishable at a glance": a grid of a hundred
tiles is scanned, not hovered, so a hover-only mark satisfies the rule it cites and fails the
thing it is for. The tile already carries the exception this argument produced once before — the
rating badge, drawn in the corner whether or not the pointer is there, because a rating has to be
readable across a whole grid. The posted mark goes beside it, for the same reason and by the same
means, and names the site in its title: the hover rule still owns the *detail*, and only the fact
that there is a post is always on screen.

**D3. Send the file's bytes as multipart, not a URL for Danbooru to fetch. This departs from the
legacy flow, deliberately.**

Legacy sent `upload[source]` = the image address and `upload[referer_url]` = the page address, and
let Danbooru download the file. That was the only option available to it: the extension held the
image as a blob in IndexedDB behind a browser origin, and pushing bytes from a page script was
harder than handing over a link. The app is in a different position — it owns the file on disk,
which is the whole point of §1's staging area.

Sending bytes is better on three counts, each a real failure of the URL form: (a) the origins this
library collects from — Pixiv, X — refuse requests without the right `Referer`, and the extension
needs a `declarativeNetRequest` rule to fetch them at all, so a booru fetching the same URL gets a
403 or a placeholder; (b) a self-hosted instance on a LAN may have no outbound route; (c) the URL
form posts *whatever is at that address now*, which is not necessarily the bytes the library
holds and the user tagged — and it is the library's copy that the `posts` row claims to be about.

So: `POST /uploads.json` as `multipart/form-data` carrying the file and nothing else — no
`upload[source]`. On that endpoint `source` is the address Danbooru downloads from, not a label,
so sending it beside the bytes at best duplicates the fetch and at worst fails on the very hosts
(a) names. The source the user sees on the post is a string on the post, not on the upload:
`POST /posts.json` carries `post[source]` = the form's source field (the page address, D10),
which Danbooru stores verbatim and never fetches. A local import has no page or image address,
so its source is empty rather than a path on this machine, which no booru could reach and which
would leak the library's location. The part name for the file is `upload[files][0]`. It was
first inferred as `upload[file]` from the endpoint's documented fields; task 6.2 corrected it
from Danbooru's source and the owner's instance (see Risks), and only the request builder
changed.

**Nesting, since `post[source]` above is one instance of a rule and not a special case.** Danbooru
takes a record's own attributes as nested parameters and everything else flat, so
`POST /posts.json` carries `upload_media_asset_id` top-level beside `post[tag_string]`,
`post[rating]` and `post[source]`, and
`PUT /posts/{id}/artist_commentary/create_or_update.json` carries
`artist_commentary[original_title]` and `artist_commentary[original_description]` with the post
named by the path alone — no `post_id` in the body, which the path already said. This was
implemented flat at first (`tag_string`, `rating`, `source`, `original_title`,
`original_description`, plus a redundant `post_id`), reading `post[source]` above as naming one
field rather than the shape of the whole body; a flat body is silently dropped by strong
parameters, so the post would have been created with none of its attributes. The names are still
verified with the rest of D3 by task 6.2, the *shape* follows one rule instead of two, and the
tests match on the exact body (D14) so a drift back to flat fails the suite rather than a real
booru.

**D4. The client is a plain module over `reqwest`, four calls wide, behind one `BooruClient`
struct that takes the base address and the credential.**

No generated client, no `danbooru-rs` dependency. The flow is four requests; a dependency that
models the whole API would be a large surface for that. `BooruClient` holds a `reqwest::Client`, the base address, and the credential; every method
returns `Result<_, UploadStepError>` (D6).

"One per app, connection pool reused" was the intent written here and is not what shipped: a
`BooruClient` is built per upload and per connection test, and its `reqwest::Client` with it, so
the pool is reused across the four calls of one sequence and dropped with it. Holding one for the
app would mean a field on `AppState`, and nothing would use it between sequences — the only
outbound traffic this app has is an upload or a test, each a burst of calls to one host. The pool
belongs in the app state the day something talks to a booru outside one sequence; until then the
per-sequence client is the same reuse where the reuse pays.

`reqwest` features: `multipart`, `json`, `default-tls`. **Platform TLS, not bundled roots** — a
self-hosted instance usually presents a certificate the machine has been told to trust (a company
CA, a locally trusted development CA), and `rustls` with `webpki-roots` would reject exactly the
certificates the user already installed, with no way to fix it from inside the app. Insecure
addresses stay allowed for LAN instances, with the warning `booru-sites` requires.

Timeouts, all named constants: connect 10 s; 30 s for the metadata calls; 120 s for the call that
carries the file, because a large file on a slow uplink is not a hung server. Processing poll:
20 attempts, 2 s apart, the legacy constants, kept because there is no better evidence than
"what the previous implementation used" and changing them without a real instance would be a
guess. Task 6.2 saw the first poll answer on a file upload — Danbooru processes a file inside the
create call itself — so these bound only a slow instance.

**"(40 s)" was wrong, and the correction is a third constant, not a different pair.** 20 x 2 s is
40 s of *sleeping*; each poll is also a request carrying the 30 s metadata timeout, so a booru
that accepts the file and then answers nothing at all held the upload for 20 x (2 + 30) s — about
ten minutes, against a decision that read as forty seconds. The attempt count bounds the number
of answers waited for, which is what the legacy constants were about; it bounds no amount of time.
So the phase carries an overall deadline, `POLL_DEADLINE` = 60 s, and every poll's own timeout is
capped at what is left of it: 40 s of waiting plus room for the polls themselves, and a hard
stop whichever way the booru misbehaves. The deadline and the interval travel together as
`PollSchedule` for the same reason the interval was already a parameter — tests drive the whole
phase in milliseconds (D14) rather than sleeping through it.

**D5. The remote sequence runs entirely outside the database; the transaction is the last step
and covers only the recording.**

"Written in the same transaction as success" cannot mean holding a SQLite transaction open across
40 s of HTTP — one connection behind a `Mutex` (Phase 1 D1) means that would freeze every search
and every capture ingest for the duration. What it does mean, and what the guarantee actually
needs, is that recording a post is one atomic write that happens **only after** the post exists:
the `INSERT INTO posts` and the `images.updated_at` bump are one transaction, taken after the last
network call that can fail hard. Before that point the library is untouched, so any failure leaves
it exactly as it was — which is what the `booru-upload` spec's "records nothing against the image"
scenarios assert.

The consequence to accept: the post can exist remotely while the local write fails (disk full,
library closed mid-upload). That is strictly better than the reverse, and the error names the post
so the user can find it.

The command is `async` and does its own network work; the `Library` mutex is taken twice, briefly
— once to read the image row and file path, once to record — and is never held across an await.

**D6. One outcome type naming the step, not a string.**

```
enum UploadStep { Authenticate, CreateUpload, AwaitProcessing, CreatePost, Commentary }
struct UploadStepError { step: UploadStep, message: String, remote_ref: Option<String> }
enum BooruUploadOutcome { Posted { post: PostRef, commentary: CommentaryOutcome }, Failed(UploadStepError) }
```

`message` is the booru's own text when it gave one, the transport's otherwise — never a
paraphrase, because a Danbooru validation error ("duplicate of post #123") is the most useful
thing the user can be told and rewriting it loses that. `remote_ref` carries the upload id when
`AwaitProcessing` times out, so the failure can point at `<base>/uploads/<id>`, and the post id
when the commentary fails.

`Commentary` is the only step whose failure is not a failure: the post exists, so `Posted` is
returned with `CommentaryOutcome::Failed(message)` and the dialog shows a warning. This is why the
outcome is a returned value rather than `AppError` — `AppError` serialises to a bare string
(error.rs) and cannot carry a step, a remote reference, or a partial success. `AppError` stays for
what it is for: no library open, image not found, credential unavailable, database failure.

**Only a failure names a step; the form cannot say which step is running, and `booru-upload` has
been amended to say what it does instead.** "The form SHALL show which step is in progress" was
right as written: this enum names the five steps, and a form that reports a failure by step can
report progress from the same vocabulary — the requirement asks for nothing the design does not
already have words for. What it assumed is a channel this decision does not create. The outcome is
*returned once*, when the sequence is over; while it runs, nothing crosses the IPC boundary, so
the webview knows only that `booru_upload` has not resolved yet. A form animating through the five
labels on its own would be inventing the one fact the user wants — whether the wait is the file
going up or the booru chewing on it — and would say `CreatePost` while a poll is still timing out.
So the requirement now asks for what is true: the form says the upload is running and what the
sequence does, and the words that name a step stay attached to the step that failed. The shape
that closes it is an `upload:step` event from `booru_upload` carrying the step about to run — the
same shape the import, export and rules runs already emit. It is not built; it is the open item in
tasks.md's webview handoff, and a `FIXME` in `UploadDialog.svelte` says so at the site.

**D7. Keychain entries: service `"BooruBox"`, account `"<host>/<username>"`.**

`keyring::Entry::new(service, user)`. One constant service so all entries group as one app in
Keychain Access and Windows Credential Manager. The account is the host and username together
because *that pair is the booru account the key belongs to* — not the site row. Consequences,
accepted deliberately:

- Two libraries configuring the same account share one entry: the key is entered once. Two
  libraries configuring the same host with *different* accounts get different entries, so neither
  can post as the other. That second case is the one that matters — a silent overwrite there would
  post under the wrong user's name.
- Changing a site's username changes which account it posts as, so the app asks for the key again
  rather than carrying the old one over.
- Removing a site removes its entry, which can inconvenience another library configured for the
  same account (risk below). Alternative — never remove — rejected: "remove the site" leaving the
  secret behind is the wrong default for a credential.
- **Within one library, the entry is removed only when no site still uses that account.** The
  account is `(host, username)` and a site is a base address, so two sites can be one account: the
  same host on two ports is two rows — D2's slug rule even names that case — sharing one entry.
  An unconditional delete would log the surviving site out of a booru the user never touched. The
  same guard covers the edit that moves a site to another account, which previously wrote the new
  entry and left the old one behind for good: the key for the account the site left is removed,
  unless another site is still posting as it. The guard is per library, which is the only scope a
  library can see; the cross-library case above is unchanged and stays a risk.

Rejected alternative: account = a random per-site UUID. It removes the sharing question but makes
the entry meaningless in the OS's own UI ("BooruBox / 7f3a…"), and the user cannot audit or revoke
what they cannot read.

**Refusal.** `keyring` fails when the keychain is locked, the user denies access, or the platform
has no backend (a headless CI runner is the common one). The rule: site configuration is a
database write and never depends on the credential store, so a refusal loses nothing but the key.
Reads report `AppError::Credential { reason }`, the site lists as unusable with that reason, and
`booru_upload` fails on that before opening a socket. There is no fallback store — writing the key
to `settings.json` on a platform where the keychain is unavailable would be exactly the outcome
the decision forbids, and doing it silently would be worse.

**Where "the site lists as unusable" actually happens, since it is not where the sentence above
implies.** That sentence reads as if the site list carried the credential's state, and nothing in
the app can answer that: `booru_site_list` reads `booru_sites`'s own columns and never the
credential store — which is the whole of this decision — so `BooruSite` has no credential field
and there is no side-effect-free command that probes one. The reading was right when it was
written, because the state it describes is real and the spec requires it; what it did not settle
is *who asks*.

The two ways to ask, weighed once the commands existed:

- **Probe every listed site.** `booru_site_test` rejects with `AppError::Credential` before it
  opens a socket exactly when the credential cannot be read, and resolves otherwise — so calling
  it per site on the settings screen would fill the column. It also contacts every configured
  booru whenever that screen is opened, for the sites whose credential is fine: unrequested
  outbound traffic to third parties as the cost of rendering a list, and up to a 30 s wait per
  site offline. Rejected on that.
- **Surface it where the credential is read.** `booru_site_test` and `booru_site_save` reject
  with the reason when the store refuses them, and the settings list keeps what they found, per
  site, for the session: `Unusable — <reason>` once either has been refused, the test's own
  outcome once it has answered, and `Not checked.` before either. An untested site claims nothing
  about its credential, which is honest — nothing has looked.

  **Two sources, not three — narrowed from "every call that reads a credential".** `booru_upload`
  does refuse on the same condition before it opens a socket, so the sentence was right about
  where the app learns things; what it missed is that the upload cannot *say* it learned this one.
  `AppError` reaches the webview as a bare string (D6), so a rejected upload is indistinguishable
  from "no library open" or "the image is gone". Handing every rejection to the store would mark a
  working site unusable for the session on any of those; handing it none loses nothing the test
  does not also find, since the failed upload is one button away from Test. So the dialog shows
  the rejection and attributes it to no site, with the reason written at the state that holds it.
  A tagged credential error from Rust — one the webview can match on rather than read — closes
  this properly, and is a handoff item beside the probe below.

So the second, in `src/lib/api/booru.svelte.ts` (the shared site store) and
`src/lib/components/booru/BooruSection.svelte`. The upload half of the requirement is unaffected
and already holds: choosing an unusable site reports the store's reason and contacts nothing.

The shape this should have is a Rust credential probe — `Credentials::get` with no network behind
it, per site or for the store as a whole — which would let the list say "unusable" before the user
touches anything. It is a command and a field, not a design change; it is not built, so a `FIXME`
in the store says so and the webview handoff notes in tasks.md name it.

**Testability follows from the same place**: credential access sits behind a
`trait Credentials { fn get/set/delete }` with a `KeyringCredentials` implementation and an
in-memory one for tests. Rust tests must not touch a real keychain — on macOS that prompts, and on
CI there is nothing to prompt.

**D8. Test connection is `GET /profile.json`, judged by status code.**

It is the cheapest authenticated read Danbooru offers, and it changes nothing — the spec requires
the test to be free of side effects, which rules out a dry-run upload. Judged by status, not by
body: 2xx → connected and accepted; 401/403 → credential rejected; any other status → the site
answered but not usefully, reported with the status; transport error → unreachable, with the
transport's reason. The body is not parsed beyond confirming it is JSON, because self-hosted forks
differ in what a profile contains and asserting a field would fail against instances that work.

**D9. The form is a dialog opened from Slot: Inspector · actions.**

The action row shows `Upload to <site>` (a dropdown when several sites are configured, per the
`booru-upload` spec) and the posted labels. The form itself is a modal dialog, not inline in the
Inspector: it is seven fields plus a preview, and inlining it would push the identity and tag
slots out of a panel whose whole job is showing them. A dialog is also the shape a later bulk
upload would reuse. Component: `src/lib/components/booru/UploadDialog.svelte`, built on the
shadcn-svelte `dialog` copy-in app-shell already added.

**D10. Prefill rules, and no default rating — reversing the legacy behaviour.**

| Field | Prefilled with |
| --- | --- |
| Tags | the image's tags, sorted by the library's tag order (`sortTags`), space-separated |
| Rating | the image's rating; **nothing if the image is unrated** |
| Source | `pageUrl`, else `imageUrl`, else empty |
| Artist | the artist candidate derived from `pageUrl` (D11); empty when no rule matches |
| Commentary title | `pageTitle` |
| Commentary body | empty |

Legacy defaulted an unrated image to `q` (`const ratingValue = image.rating || 'q'`). Rejected:
that publishes a guess about someone else's artwork under the user's account, and `q` is wrong in
both directions — it mislabels safe images and under-labels explicit ones. An unrated image gets
no preselection and the form refuses to send until the user picks (spec: "An upload will not be
sent without tags and a rating").

The form keeps *one* tag field plus a separate artist field, dropping legacy's separate copyright
and character boxes. Those had no prefill and no validation: they were three extra ways to type a
tag into a string that gets concatenated anyway. The artist field survives because it is the one
that *is* prefilled, from D11.

**D11. Artist extraction is prefill, and stays in TypeScript in the webview.**

CLAUDE.md's rule is that policy lives in the app, not the extension; it does not say policy lives
in Rust. This rule reads a page address and proposes a string the user can overwrite before
sending — it decides nothing the app acts on by itself. Porting it to Rust would put it on the far
side of the IPC boundary from the form that displays it, for no gain. It lands as
`src/lib/domain/artist-from-url.ts`, ported verbatim from the legacy `extractArtistFromUrl`
(Pixiv users → `pixiv_user_<id>`, Pixiv artworks → source only, X/Twitter handle excluding
`i`/`home`/`search`, `<name>.fanbox.cc` → `<name>_fanbox`, DeviantArt handle, ArtStation → source
only), with a test per rule — the legacy module had none.

**Verbatim, with one capture narrowed — and the legacy did have a test.** "Ported verbatim" was
the right instruction while the port was believed to be untested code being given tests for the
first time: with nothing pinning the old behaviour, changing a rule on the way across would have
been an unrecorded rewrite. The legacy module turns out to have `tests/format.test.ts`, and that
test documents the fanbox rule's capture class `[^.]+` as a known quirk: on a full address it runs
back over `https://` to the start of the string, so `https://someone.fanbox.cc/posts/1` yields the
artist `https://someone_fanbox`. That is a bug the old test froze rather than a behaviour worth
carrying, and this port's output is a tag on someone's booru post. So the class is `[^./]+` here —
one host label — and this port's own test asserts `someone_fanbox` for both spellings, naming the
old output. Every other rule, and the order in which a later rule overwrites an earlier one, is
the legacy's unchanged.

`bridge-extension` stores a richer adapter record (`adapter_json`: X → handle, Pixiv → artist),
which is a better artist source than a regular expression over a URL. Not used here: it exists
only for images captured after that change ships, so the URL rules are needed regardless, and
preferring the adapter field where present is a one-line improvement that belongs in a change that
can test it against real captures. Marked `FIXME` at the prefill site.

**D12. An image already posted to a site is not offered that site again; re-upload is not
supported.**

Idempotency here is a UI rule over a database fact, not a protocol: the presence of a
`posts (image_id, site)` row is what withholds the action, and that row is written only on real
success (D5). Making re-upload work would mean deciding what it means (a second post? an edit of
the first? a deletion?), which is a product question §6 does not answer; it is a Non-goal until it
is asked.

The case the app cannot see — the same file posted from another machine, or before this app
existed — was first left to Danbooru's own duplicate detection, on the reading that it refuses the
file at `POST /uploads.json` and the refusal would surface verbatim through D6's `CreateUpload`
message. Task 6.2 read the source instead: Danbooru accepts a duplicate file without complaint
(`MediaAsset.upload!` hands back the asset it already has), and only `POST /posts.json` notices —
by *merging the sent rating and tags into the original post* and answering with a redirect to it.
That is an edit of a post the user never chose, not a refusal. So the sequence asks first:
`GET /posts.json?tags=md5:<checksum of the library's file>&limit=1` (`posts` is keyed by the
file's MD5, and the `md5:` search is open on every version), and a hit is a `CreateUpload`
failure naming the post, with its id as the `remote_ref` the dialog links. Danbooru's post-step
answer remains the backstop for the race between the lookup and the post, which is why the client
never follows a redirect (D4): followed, it would fetch the original post's HTML page under the
credential and the failure would read as an unreadable response.

**D13. Settings gets its own "Booru" section, not a shared one with Rules.**

app-shell's slot map lists `Settings · Rules` as absent-for-now and names two future occupants:
`auto-tag-rules`' rules table and this change's sites. Those are two unrelated tables of
configuration that happen to have been written on one line of a planning table. This change adds
**Settings · Booru** — sites list, add/edit/remove, test connection, credential state — as a
sibling section, and `auto-tag-rules` keeps `Settings · Rules` to itself. Merging them would
produce a screen section named after neither of the things in it.

**D14. Tests use `wiremock` against the real client.**

`wiremock` over `httpmock`: it is async-first and its `MockServer` binds a fresh random port per
instance with no global registry, so each `#[tokio::test]` gets an isolated booru and the suite
stays parallel — which matters because the poll tests are the slow ones. `httpmock`'s ergonomic
path is its blocking API plus an optional shared server, and a shared server across parallel tests
is precisely the cross-test coupling to avoid here. Both are maintained; the tie-break is
isolation under `cargo test`'s default parallelism.

The mocks match on the request body, not only on method and path: the multipart part name on
`/uploads.json`, and the exact set of form fields on `/posts.json` and the commentary call. Every
name in those bodies was settled by task 6.2 (D3); the property the suite holds is
that what the design says is on the wire is what the client puts there — a renamed, dropped or
extra field fails a mock here instead of a real booru later.

What is mocked is the booru, not the client: the tests drive `BooruClient` end to end against
canned responses — happy path, credential rejected, upload refused, poll returning `error`, poll
never completing (poll interval overridden to milliseconds in tests), post creation refused,
commentary refused. The command-level test additionally asserts the `posts` row exists after
success and does not exist after each failure.

## Risks / Trade-offs

- **Every Danbooru request shape in this change was inferred, not verified** (legacy was never
  run end to end) → the mocked tests prove the client's own logic, not the protocol. Task 6.2
  settled each shape against Danbooru's source and the owner's instance (a 2023-10 build): the
  file part is `upload[files][0]` (`UploadPolicy` permits `files` as an index-keyed hash since
  multi-file upload, 2022-02-19; the singular `upload[file]` the design guessed is refused as an
  unpermitted parameter, with a 403 that read as a credential failure); processing is finished
  when `upload_media_assets[0].media_asset_id` is non-null, not when the row exists; a failed
  upload carries its reason in `error`, an exception page in `message`, a validation refusal in
  `errors`; `upload_media_asset_id` top-level and `post[...]` nested on `/posts.json`, and the
  commentary's fields under `artist_commentary[...]`, are as designed. Duplicates are D12's
  amendment. An instance older than 2022-02-19 is out of scope.
- **The post can exist remotely while the local record fails to write** (D5) → the failure names
  the post id and its address, so nothing is lost but the label; the user can see the post exists.
- **A processing timeout may be a slow booru, not a broken one** → the app records nothing and
  points at `<base>/uploads/<id>`; the user finishes on the booru. Never writing a speculative
  `posts` row is the deliberate choice: a label claiming a post that does not exist is worse than
  no label.
- **Removing a site removes a keychain entry another library may share** (D7) → the other library
  reports "credential unavailable" with its reason and the user re-enters the key, which is on the
  booru's own profile page. No data is lost.
- **Basic authentication over an insecure address sends the key in clear text** → the site form
  warns; refusing outright would lock out the self-hosted LAN instance this feature exists for.
- **`keyring` pulls in a platform-specific backend and can behave differently on Linux** → not a
  target platform (§6 is macOS + Windows), and the `Credentials` trait keeps both the tests and a
  future fallback out of the call sites.
- **Outbound network access is new for this app** → confined to one module; the loopback listener
  and its origin rules are untouched.

## Open Questions

- Does the owner have a self-hosted Danbooru instance available for task 6.2, or should the manual
  pass run against the official site with a throwaway upload? Either satisfies the task; it
  changes only which instance is named in the verification note, not what is built.
