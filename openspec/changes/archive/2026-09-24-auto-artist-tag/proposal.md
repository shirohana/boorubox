## Why

Every capture from X or Pixiv already carries its author — the extension's adapters read the
X handle and the Pixiv display name and hand them over in the adapter record — yet the image
arrives with no artist tag, and the owner types one by hand on every image (2026-09-23).
Requirements §4 already gives the app this job: the adapter record is extraction, and "the
app decides what to do with it (tags, artist field, etc.)". Requirements §6 (Danbooru-style
tagging): the artist category exists since `tag-vocabulary`, so the tag can be created where
it belongs instead of as a general tag.

## What Changes

- **A capture is stored with an artist tag named after its author**: from X, the handle; from
  Pixiv, the display name; lower-cased, runs of whitespace read as `_`, the way every tag and
  collection name is spelled. A capture from any other site, or whose record lacks the field,
  gets none.
- **The derived tag never takes over a name the vocabulary already uses**: when a tag of that
  name exists under any category other than artist — general included — the artist tag is
  left off entirely, not linked under the existing category. This is a third conflict policy
  (Skip) beside the editor's Refuse and the rules' Keep; the rules keep Keep. The image stays
  findable by `arttags:0` (`category-count-search`), which is where the owner tags it by hand.
- **Always on, no setting.** Legacy bundle imports are exempt, as they are from the rules:
  what the bundle carries is what is stored. A re-delivered capture gains nothing.
- **A capture's 2xx never depends on it.** The derivation is pure and the link skips rather
  than refuses; the only failure left is the database's own, which already fails a capture.
- **The inspector's X Account row stays**, and the spec sentence that kept the handle out of
  the tags is amended: once the handle *is* the artist tag, it is no longer a second artist
  among the tags.

### A reversal, recorded

`tag-editing`'s "An X account on screen is a search term" put the handle beside the page's
facts and not among the tags, "with artist tags in the vocabulary, a handle among the tags
read as a second artist" (owner, 2026-09-23). That was right while the only artist tag on an
image was one the owner typed, and a handle tag next to it was a second name for the same
person. It stops being right here: the handle is now the artist tag itself, so there is one
artist among the tags, spelled as the handle. The Account row keeps its place for the reason
it always had — it is a fact of the page address, it searches `account:` (which reads the
address, not the tags), and it is present on images captured before this change or whose
artist tag was skipped. The delta spec carries the argument in the requirement.

### A non-goal, honoured

`bridge-extension`'s non-goal "Auto-tag rules, artist extraction, rating and tag derivation
stay in the app" (`archive/2026-09-07-bridge-extension/proposal.md`) is honoured, not
reversed: the extension does not change, the adapters still interpret nothing
(`site-adapters`, "Adapters produce a plain record and interpret nothing"), and the derivation
is app policy in Rust (CLAUDE.md: extraction lives in the extension, policy lives in the app).

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `capture-ingest`: a new requirement — a capture is stored with the artist tag its adapter
  record names, skipped on a category conflict, absent for other sites, missing fields and
  bundle imports, never failing the capture.
- `tag-editing`: "An X account on screen is a search term" — the argument for keeping the
  handle apart from the tags is replaced, and "the handle SHALL never be stored as a tag"
  becomes "the Account row never writes a tag; the handle reaches the tags only as the derived
  artist tag".

## Non-goals

- **Tagging images already in the library.** No data patch and no button. The rules engine's
  on-demand run (`rules::run`, "Rules can be run over images already in the library") is the
  door an "apply to existing" action would use later; the stored `adapter_json` holds what it
  would need.
- **A user id as the tag, or a rename map.** The owner's answer to a renamed artist is
  Danbooru's: search the old tag, bulk-retag to the new one (the design records why this makes
  the display name the tag). Extracting a Pixiv user id would be an extension change, and is
  not made.
- **A setting to turn it off.** Owner, 2026-09-23: always on.
- **The booru upload prefill.** `upload-form.ts` carries a `FIXME(booru-upload D11)` saying the
  adapter record beats `extractArtistFromUrl` as the upload's artist source. This change does
  not touch the upload form, so that FIXME stays whole rather than half-resolved; it can read
  the image's artist tag in its own change.
- **More sites.** Only X and Pixiv carry an author field today.
- **Any extension or webview behaviour change.** The one webview edit is a comment in
  `Inspector.svelte` that repeats the reversed argument.

## Impact

- Rust (`packages/app/src-tauri`): `rules.rs` (a pure `artist_tag(adapter)` beside
  `haystacks`), `tags.rs` (`Conflict::Skip`; the whitespace-to-`_` spelling shared with
  `collections::slug`), `collections.rs` (`slug` calls the shared spelling), `ingest.rs`
  (`resolve_tag_text` answers the artist tag beside the rule text; `insert_rows` links it in
  its own `link_tags` call), `http/captures.rs` tests.
- Webview: one comment in `packages/app/src/lib/components/library/Inspector.svelte` (the
  Account row's). `pinned-collections` edits the same file in other regions.
- No migration, no wire change, no shared type change, no extension change.
- `library.json` is rewritten when a capture creates an artist tag, as it already is when a
  rule creates a categorised one.
