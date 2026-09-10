## Why

§1 makes the app "a staging area in front of boorus": images land here, get tagged and rated,
and are then posted to Danbooru or a self-hosted instance. Everything before this change is the
staging half. §6 asks for the other half — the upload flow, and a record per image of *where it
has been posted* — and Phase 1 already built the `posts` relation for it (Phase 1 D2, "created
now so §6's `posted:` filter has a home; Phase 1 never writes it"). This change is what finally
writes that row.

Now, because the two things the form needs — a tag list worth posting and a rating — only exist
after `tags-and-ratings`, and because the legacy upload flow is the one feature of the old
extension that was never exercised end to end (legacy `HANDOFF.md`: "Not fully tested: Danbooru
upload (modal opens/loads; an actual upload was not run)"). Porting it into a place with tests
is how it stops being unverified code.

**Depends on:** `tags-and-ratings` (tag and rating values, `src/lib/components/tags/TagInput.svelte`
for the form's tag field), `app-shell` (Slot: Inspector · actions, Slot: Grid · tile, the
Settings screen).

## What Changes

- **Configured booru sites** (§6 "per configured site — official Danbooru and self-hosted"):
  name, base URL, username, and an API key. Sites live in `library.sqlite`; **the API key goes
  to the operating system's credential store, never to `settings.json` and never to the library
  file** — a library folder is copied and cloud-synced (§7), and a credential must not travel
  with it. "Test connection" says whether the site answers and whether the credentials are
  accepted, before any upload is attempted.
- **Upload one image from the Inspector** (§6 "Phase 2 ports today's upload flow", §8 Phase 2):
  an action in Slot: Inspector · actions opens a form prefilled from the image — tags (sorted),
  rating, source address, an artist candidate derived from the page address, optional commentary.
  Submitting runs the whole booru sequence in Rust: create the upload with the file's bytes, wait
  for the booru to finish processing it, create the post, then optionally set the artist
  commentary.
- **Failures name the step that failed.** Credentials rejected, upload refused, processing timed
  out, post creation failed, commentary failed — each is a distinct outcome with the booru's own
  message, and only the last of them leaves a usable post.
- **A post is recorded when, and only when, one exists** (§6 "a `posts` relation (image ↔ site ↔
  remote id ↔ posted at)"): the row is written in the same transaction that concludes a
  successful upload. No row is written for a sequence that failed part way.
- **"Posted to `<site>` #id"** on the grid tile (Slot: Grid · tile) and in the Inspector, linking
  to the post. An image already posted to a site offers no upload action for that site.
- **New Rust dependencies**: an HTTP client and an OS credential-store binding; a mocked HTTP
  server in dev-dependencies so every step of the sequence, including each failure, is tested
  without a booru.

## Capabilities

### New Capabilities

- `booru-sites`: configuring the boorus this library posts to — name, base URL, username, an API
  key held by the operating system, and a connection test that reports what is wrong.
- `booru-upload`: uploading one image to a configured site from a prefilled form, the sequence's
  per-step failure states, and recording the resulting post against the image.
- `posted-label`: showing where an image has already been posted, on the tile and in the
  Inspector, and withholding the upload action for a site it is already on.

### Modified Capabilities

None. `library-browse`'s shipped requirements describe what the grid shows and what the search
language means; a badge added to a tile and a record added to an image change neither. The
`posted:` / `unposted` filter *would* change "Tag search uses the legacy query language" — it is
Phase 3 and out of scope (Non-goals).

## Non-goals

- **The `posted:` / `unposted` search filter.** §8 puts it in Phase 3 with the pull-back
  feature; this change ships the label half only, and the `posts` rows it writes are exactly
  what that filter will read.
- **Re-uploading, editing or deleting a remote post.** The app records that a post exists and
  links to it; changing it is done on the booru.
- **Bulk upload.** `selection-and-bulk` owns the selection model and its toolbar; a bulk upload
  would need a queue, per-item outcomes and rate limiting, which is its own change.
- **Pulling posts back into the library, tag reconciliation** (§6 "Later", §8 Phase 3).
- **Anything but Danbooru's API.** "Self-hosted booru" here means a Danbooru-compatible
  instance. Other booru software is not a target.
- **Storing credentials when the operating system refuses to hold them.** There is no fallback
  file; the site simply has no usable credential until the credential store works.

## Impact

- `packages/shared`: `BooruSite`, `BooruUploadForm`, `BooruUploadOutcome`, `PostRef`, and
  `ImageRecord` gains `posts`. `model.rs` mirrors this file by hand (Phase 1 D11), so both change
  together.
- `packages/app/src-tauri`: schema migration adding `booru_sites` (design D1); new `booru/`
  module (client, sites, credentials); `commands.rs` gains `booru_site_list`, `booru_site_save`,
  `booru_site_delete`, `booru_site_test`, `booru_upload`; `query.rs` fills `ImageRecord.posts`.
  New dependencies `reqwest` and `keyring`, dev-dependency `wiremock` (design D9, D10).
- `packages/app/src`: `lib/api/booru.ts`; `lib/components/booru/` (upload dialog, posted label,
  the Settings · Booru section); `lib/domain/artist-from-url.ts` lifted from the legacy repo with
  its rules; the Inspector gains its actions row.
- Network: the app makes outbound HTTPS requests for the first time. Until now it bound one
  loopback listener and made no outbound calls at all.
