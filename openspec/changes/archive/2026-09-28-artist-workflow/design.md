## Context

- `artist-entries` (archived 2026-09-25) shipped `artist_urls (url TEXT PRIMARY KEY, tag TEXT
  NOT NULL)` at schema v12 (`db.rs`), and `artists.rs`: `normalized`, `owns` (prefix past a
  `/`), `PROFILE_URLS` (X `handle` → `https://x.com/{}`, Pixiv `userId` →
  `https://www.pixiv.net/users/{}`), `ARTIST_FIELDS`, `artist_tag(adapter)` (the derived
  name), `candidate(adapter)` (the record's own profile URL, normalised — never the page or
  post URL, `artist-entries` D4), `derive` (longest owning entry, skipped when its tag is no
  longer an artist tag, else `artist_tag`), `list`, `upsert` (replaces a tag's URL set;
  refuses an empty tag or URL list, a URL that does not normalise, a URL another artist owns,
  a tag of another category), `delete`, `rename_preview`, `rename` (one transaction, refuses
  `to == from` by answering `{ retagged: 0 }`, retags carriers trash included, merges;
  sidecars via `ingest::load_records` + `sidecar::write_for_records` and `write_library`
  after the commit).
- Commands `artists_list`, `artists_upsert`, `artists_delete`, `rename_artist_preview`,
  `rename_artist` (`commands.rs`, each through `with_library_off_main_thread`); wrappers in
  `api/commands.ts`; wire types `ArtistEntry`, `RenameArtistPreviewInput`,
  `RenameArtistPreview`, `RenameArtistInput`, `RenameArtistReport` in `model.rs` and
  `packages/shared/src/index.ts`.
- `rules::run` is the only pass over stored images: it reads `id, page_title, adapter_json,
  rating FROM images WHERE deleted_at IS NULL`, links with `tags::Conflict::Keep`, calls
  `tags::mark_updated`, and writes `sidecar::write_one` per image (and `write_library` when a
  categorised row was born) after each image's own commit.
- Inspector: the tag badge menu carries "Rename artist…" for `vocabulary.categoryOf(tag) ===
  'artist'`, outside `TagVocabularyMenuItems` (`artist-entries` D6); `renamingArtist = { tag,
  image }` is a snapshot; `RenameArtistDialog.svelte` is mounted unconditionally with
  `portalTo`, and `onartistrenamed` (LibraryScreen's `afterWrite`) or `results.refresh()`
  runs after it. The Account row is `{#if image.account}` (Rust's `x_account(page_url)`), a
  button toggling `account:` with `toggleAccountInQuery`. The pinned-counts effect drops a
  stale answer with a `pinnedFetch` ticket. `TagVocabularyMenuItems` takes only `name` and is
  mounted by the sidebar row (`TagSidebar.svelte`), the pinned chip and the badge
  (`Inspector.svelte`).
- Settings → Artists (`ArtistsSection.svelte`, `ArtistForm.svelte`) carries a FIXME naming the
  missing "run over existing images" pass and a paragraph saying only a rename retags.
- Three sibling changes land first and touch the same files: `tag-row-and-inspector-fixes`
  (the inspector's editor-reset effect, a second facts pencil), `settings-pages` (Artists moves
  to `routes/settings/artists`, `ArtistsSection` gains a filter box), `tag-notes`
  (`TagEntry.note`, "Edit note…" in `TagVocabularyMenuItems`, a tag-note dialog hosted by the
  inspector, the note under the tag in the Artists list). This design is written against
  today's code; unit U re-reads those files as landed.

## Goals / Non-Goals

**Goals:** one dialog that edits an artist from wherever its tag is seen; a fact row that says
who owns the image's author and makes creating that owner one click; applying an entry to
stored images as an explicit, per-artist, additive act.

**Non-Goals:** a library-wide re-derivation; removing the derived tag when applying; any change
to capture-time derivation, `account:`, or the Account row; a profile URL for Pixiv records
without `userId`.

## Decisions

**D1. "Edit artist…" on every artist-tag menu — reversing `artist-entries` D6.** D6 kept the
item in the inspector badge's menu only, and out of `TagVocabularyMenuItems`, because the
dialog's two inputs came from an image: the prefilled URL was the image's profile URL and the
count came from the same preview call keyed by that image. That was right at the time: an
entry existed only for artists already renamed, so the one URL worth offering was the one on
screen, and the sidebar row and the pinned chip had nothing to offer. It stopped being right
once the job became "maintain an artist" rather than "correct a name once": adding the
second X account of an artist already corrected starts from the artist, not from an image of
the old account, and the entry's own URLs — which the old dialog never showed — are what the
user edits. Neither input needs an image any more: the URLs are the stored entry's
(`artists::list`), the count is a carrier count by tag name (`rename_preview` already counts
by name alone). The image, when there is one, contributes only an optional extra line (D3).
So the item moves into `TagVocabularyMenuItems`, shown for an artist-category tag, labelled
"Edit artist…" with `PencilIcon`, first among the vocabulary items (before the Danbooru
look-up) so it stays where "Rename artist…" was on the badge menu. The badge menu's own
copy of the item and its comment go. Also recorded under D6 in the archive (task 1.8).

**D2. The menu item opens a host's dialog through one callback.**
`TagVocabularyMenuItems` gains `oneditartist: (tag: string) => void`, required like
`tag-notes`' `oneditnote` so a mount that forgets it fails typecheck rather than dropping the
item (all three mounts carry artist tags); the item renders only when the tag's category is
artist. The component cannot host the dialog
itself: bits-ui destroys a menu's content when the item closes it (the reason
`RenameArtistDialog` is mounted beside the menu today). Each host mounts one `ArtistDialog`
and passes a callback that takes a snapshot: `Inspector.svelte` for the badge and the pinned
chip (`{ mode: 'edit', tag, adapter: image?.adapter ?? null }` — the chip has an image only
while one image is inspected; with a selection it opens without a candidate), and
`TagSidebar.svelte` for the row (`adapter: null`). A callback per dialog kind rather than one
`onopen(kind, tag)` because each host decides which dialogs it hosts; this is the same shape
`tag-notes` gives "Edit note…" — if `tag-notes` landed a different shape, unit U follows
that one rather than adding a second. The snapshot rule of `renamingArtist` (a capture's
`results.refresh()` must not change what an open dialog reads) carries over unchanged, as
`editingArtist`.

**D3. `artist_preview` replaces `rename_artist_preview`.** `artist_preview(input:
ArtistPreviewInput { tag, adapter: SiteAdapterRecord | null }) -> ArtistPreview { carriers,
urls, candidate: string | null }`: `carriers` is the carrier count, trash included (what a
rename retags); `urls` is the tag's entry's stored URLs with `https://` in front, empty for a
tag with no entry; `candidate` is `artists::candidate(adapter)` with `https://` in front, or
`null` when there is no adapter, no profile URL, or **any** entry already owns it — this one
(already in `urls`) or another (appending it could only end in the "already owned by"
refusal; the image belongs to another artist, and the Artist row, D5, already says which).
Ownership is decided by one function, `owning_entry(candidate, entries) ->
Option<&ArtistEntry>` (the longest owning URL), extracted from `derive`, which then applies
its category skip on top; `artist_preview`, `artist_match` and `apply` call it too, so "who
owns this URL" has one answer. The dialog shows `urls` then `candidate` as its lines. The old
command, its wrapper and `RenameArtistPreviewInput`/`RenameArtistPreview` are removed: the
dialog was their only caller. `rename_artist` is unchanged.

**D4. `ArtistDialog.svelte`: two modes, one plan, pure.** `RenameArtistDialog.svelte` is
renamed (`git mv`) to `components/artists/ArtistDialog.svelte` beside the settings section
that shares `lines.ts`. Props `{ open, request: ArtistDialogRequest | null, portalTo?,
onclose, onsaved }` where `request` is `{ mode: 'edit', tag, adapter }` or `{ mode: 'create',
tag: derived, url }`. Fields: Name (prefilled with `tag`, focused, selected), Profile URLs
(textarea), the count lines, the refusal line, Cancel, and the confirm button. Everything the
button says and does is a pure function in `components/artists/artist-dialog.ts`, tested:

- `confirmLabel({ mode, from, name, carriers, applyImages })`: edit → `"Save"` when
  `underscored`-equivalent trimmed name equals `from`, else `` `Rename ${carriers} images` ``
  (carriers from the preview; "1 image" singular like the existing line); create →
  `` `Create and tag ${applyImages} images` ``, `"Create artist"` while `applyImages` is 0
  or unknown; a create that renames the derived tag (below) →
  `` `Create and rename ${carriers} images` ``, the derived tag's carriers
  (`artistPreview({ tag: derived, adapter: null })`, read on open), shown also as a line above
  the apply line, "`derived` is renamed to `name` on N images": the rename is the step that
  changes existing tags, so it is the count the button names — the apply count cannot be
  named honestly beside it, since the renamed carriers are then skipped by apply. Every "N"
  is singular at 1. "The same name" throughout is `underscored` in `artist-dialog.ts`, the
  mirror of `tags::underscored` (trim, lowercase, whitespace runs → `_`, `__` kept), and every
  name a step carries is spelled by it: a looser or stricter comparison than Rust's turns a
  case-only retype into a rename Rust answers with `{ retagged: 0 }`, dropping its URLs.
- `savePlan({ mode, from, name, urls, entries, apply })` → the ordered steps:
  edit, same name → `upsert({ tag: from, urls })`, then `apply(from)` when checked;
  edit, new name → `rename({ from, to: name, urls })`, then `apply(name)` when checked;
  create, chosen name same as the derived tag, or the derived tag does not already exist as an
  artist tag → `upsert({ tag: name, urls: union(entryOf(name)?.urls, urls) })`, then
  `apply(name)`. The union is the one place the webview merges URL sets: `upsert` replaces,
  and creating under a name that already has an entry (the spec's "Create under another
  name") must add, not wipe that entry's URLs. `entries` is read with `artistsList()` when
  the dialog opens in create mode.
  Create, chosen name differs from the derived tag **and** the derived tag already exists as
  an artist tag (`vocabulary.categoryOf(derived) === 'artist'`, design D8's own note on why
  this is asked of the vocabulary store rather than a second existence check inside
  `artist_match`) → `rename({ from: derived, to: name, urls })` first, then `apply(name)` —
  amended in place (unit U, 2026-09-28; the lead's `tasks.md` `## Handoff` note before unit R
  started carries the argument in full): a plain `upsert` under the chosen name would create
  the new entry while leaving the derived handle tag exactly where capture put it, on every
  image it already tagged. Nothing in "Apply is additive" removes that leftover tag — apply
  only ever adds — so the handle tag would sit beside the chosen name on every one of those
  images until the owner renamed it by hand, which is the correction the design set out to
  make unnecessary in the first place. Renaming the derived tag onto the chosen name, instead
  of upserting a second entry beside it, is what a rename already does for exactly this
  reason: it moves the old tag's own URLs and retags its carriers in the same transaction that
  creates the new name's ownership of them. `apply(name)` still follows, for the profile's
  images a capture's own `Conflict::Skip` left carrying neither tag. This does not weaken
  "Apply is additive" — apply itself still never removes a tag; it is `savePlan` choosing
  rename over upsert for this one starting condition, the same way edit mode already chooses
  between the two.
- `canConfirm(...)`: false while the preview has not answered, while saving, and in edit mode
  while neither the name nor the URL lines differ from what the dialog opened with (a no-op
  `upsert` would rewrite `library.json` for nothing, and an empty entry is refused anyway).

Rename keeps `rename_artist`'s semantics exactly: every URL the old name owned moves with it,
so a line deleted while renaming is not deleted — the spec says so, and a second Save removes
it. Making the rename replace the set instead would drop a merge target's own URLs, which the
textarea never showed. Edit mode shows two lines: "N images carry `tag`." and, while the URL
lines are non-empty, the apply line (D6) beside an "Apply to existing images" checkbox,
unchecked on every open (owner, 2026-09-28: no migration by default). Create mode shows the
apply line alone. Each landed step bumps `artistRevision` (`api/artists.svelte.ts`), which the
Artist row (D5) is keyed on. After the attempt's last step ran, if any step landed:
`onsaved()`, once — the host runs its search and vocabulary refresh (`onartistsaved`,
`afterWrite` in `LibraryScreen`) — and on full success the dialog closes through `onclose`. A
refusal after a landed step (apply after a saved entry or a rename) keeps the dialog open with
the refusal and what landed; the unlanded steps are kept, and Save retries only those — a
re-plan from the fields would rename a tag the landed rename already consumed. The refusal
line names both, `refusalText`: "Renamed; applying failed: <reason>" ("Saved; …" after an
upsert); a refusal of the first step keeps nothing and shows Rust's reason alone, and its
retry is planned afresh from the fields, since nothing consumed them. While steps are left
the button reads `retryLabel` — "Retry apply", "Retry rename" — not the fresh plan's label,
the count lines show only what is left to run (a landed rename's line goes), and the name and
URL fields are read-only: the retry does not read them, so an edit there would be dropped
silently. `confirmLabel` never shows a count not yet read: "Rename" and "Create artist" stand
in while the carriers preview is pending, and the rename line is hidden until it answers.

**D5. The Artist row reads `artist_match`, once per image.** `artist_match(adapter:
SiteAdapterRecord) -> ArtistMatch | null`, `ArtistMatch { url, owner: string | null, derived:
string | null }`: `null` when `candidate(adapter)` is `None`; else `url` with `https://`,
`owner` the owning entry's tag (via `owning_entry`, category not consulted: the chip names
who owns the URL, and a stale entry is fixed where it is listed), `derived` =
`artist_tag(adapter)`. Called by the inspector, not carried on `ImageRecord`: a per-row field
would read the entries for every image of every search page to draw a row only the inspector
shows. The inspector derives `inspectedId = $derived(image?.id ?? null)` and its adapter, and
one `$effect` on that id (not on `image`, which is reassigned wholesale on every refresh —
the project's `$effect` rule) fetches through `latestOnly` (below); a second trigger is
`artistRevision`, a shared counter every entry writer bumps — the dialog from any host,
Settings → Artists' save, delete and Apply — so a save made from the sidebar re-reads the row
too, which a counter local to the inspector could not see. The row, directly above Account, is
`{#if artistMatch}`: `owner` → a button in `CATEGORY_TEXT_CLASS.artist` with the tag's
`searchMark` over `terms.included`/`terms.excluded`, `onclick = query(toggleTagInQuery(
tagQuery, owner))` (the badge's own call); no owner → a small ghost button "Create artist…"
opening the dialog with `{ mode: 'create', tag: derived ?? '', url }`. Read the same in both
`editingFacts` states, like Account. While the answer is pending the row is absent rather than
a placeholder: a row that appears a frame later is less noise than a "…" in every image.

`latestOnly()` in `lib/api/latest-only.ts` is the ticket as a function: each call takes a
promise and resolves `{ current: true, value }` only if no later call started, else `{
current: false }` (rejections likewise). Tested by resolving out of order. The pinned-counts
effect's `pinnedFetch` ticket adopts it too if it drops in without changing that effect's
shape; otherwise unit U leaves a FIXME there naming the helper. The dialog's apply-preview
refresh (D6) uses it as well.

**D6. `artists_apply` and its preview: one scan, one transaction, additive.** In
`artists.rs`, one scan both use: `from_profiles(conn, urls: &[String]) -> Vec<String>` — read `id, adapter_json` of every
image `WHERE deleted_at IS NULL`, keep the ids whose `candidate` some given (normalised) URL
owns.
`apply_preview(conn, urls) -> ApplyPreview { images, untagged }`: `urls` normalised, ones that
do not normalise ignored (the save names them; a preview that refused would blank the line
while the user is mid-typing); `untagged` = those ids with no linked tag of category artist (one query over `image_tags`
and `tags`). `apply(
library, tag) -> ApplyReport { tagged, skipped }`: `tag` canonicalised; refused when it has
no entry (`AppError::NotFound("artist entry {tag}")`) or exists under another category
(`tags::category_conflict`, the check `derive` makes, so apply never links an image under a
general tag); then in **one** transaction, for each scanned image not already carrying `tag`,
`tags::link_tags(tx, id, &read_metatags(["artist:{tag}"]), Conflict::Keep)` and
`tags::mark_updated`; commit; then, after the commit, `sidecar::write_for_records(
ingest::load_records(touched))` and `write_library` when a row was born — `rename`'s order
exactly. `skipped` counts images already carrying the tag. Trash is excluded, unlike rename:
rename must leave no stale name anywhere, while apply only adds, so a trashed image loses
nothing by being left out and restores as it was.

One transaction, not `rules::run`'s one per image: `run` touches the whole library and
releases the mutex between images so a capture waits for one image only; apply writes only
one artist's images — the owner's largest artist is hundreds, the scale `rename` already
handles in one transaction — and all-or-nothing is the honest answer to "tag these N".
The scan parses every non-deleted record's JSON inside the same `with_library` call; at the
library sizes the owner has this is well under a second, and no progress events are sent.
`artists_apply` runs through `with_library_off_main_thread`, like every artist command.

**D7. Settings → Artists: "Apply…" per entry, the FIXME closed.** Each entry's row gains an
"Apply…" action beside edit and delete; it runs `artistsApplyPreview(entry.urls)` and opens a
confirmation: "N images come from these URLs; M of them carry no artist tag. Applying adds
`tag` to all N; no tag is removed.", buttons Cancel and "Apply to N images" (disabled at 0),
then `artistsApply(tag)` and a one-line result ("Tagged X images; Y already carried it."). The
confirmation draws the last request it was opened for, kept past the close, so its closing
animation never reads "undefined" and zeros.
N is shown on the confirming button, not on the list row: a count per row would be one
library scan per entry on every render of the list. The FIXME is deleted and the paragraph
reads: "Changing URLs applies to future captures only. To tag images already in the library,
apply the artist; applying only adds its tag. To rename an artist and retag its images, use
Edit artist… on its tag." After an apply, `vocabulary.refresh()` (a tag row may have been
born); the library screen refreshes when shown next.

**D8. Commands and wire types.** New: `artist_preview(input: ArtistPreviewInput) ->
ArtistPreview`, `artist_match(adapter: SiteAdapterRecord) -> ArtistMatch | null`,
`artists_apply_preview(urls: string[]) -> ArtistApplyPreview { images, untagged }`,
`artists_apply(tag: string) -> ArtistApplyReport { tagged, skipped }`. Removed:
`rename_artist_preview`, `RenameArtistPreviewInput`, `RenameArtistPreview`. Types in
`model.rs` and `packages/shared/src/index.ts`, camelCase on the wire, with the wire test;
wrappers `artistPreview`, `artistMatch`, `artistsApplyPreview`, `artistsApply` in
`api/commands.ts` with an invoke test each; `renameArtistPreview` and its test go.

**D9. Pixiv.** The candidate for a Pixiv record is `pixiv.net/users/<userId>` via
`PROFILE_URLS`, unchanged. A record stored before the extension read `userId` yields no
candidate: no Artist row, no preview candidate, never matched by apply. Such images are
reached by Edit artist's rename, which goes by tag name — the same answer `artist-entries`
gave them.

## Risks / Trade-offs

- [Apply leaves the derived tag beside the entry's tag when the names differ] → owner's
  call: additive. The counts line says how many carry no artist tag, so the user sees how
  many already carry one; renaming the derived tag with Edit artist is the tool for replacing
  it, and does so in one step. Create artist under another name does not hit this when the
  derived tag already exists as an artist tag: D4's create-mode rule renames it first, and the
  dialog says so with the carrier count before the save.
- [One transaction for a large artist] → hundreds of rows, the scale `rename` already writes
  in one; revisit with `rules::run`'s per-image shape if a single artist ever reaches the
  thousands.
- [A full scan per preview, re-run as URL lines change] → the dialog re-runs it 300 ms after
  the last edit through `latestOnly`, so typing costs one scan per pause, not per key.
- [`artist_match` owner ignores a stale category] → a chip names an entry whose tag became,
  say, general; the Artists list is where it is fixed, and capture already skips it (`derive`).
- [Three sibling changes edit the same Svelte files] → unit U runs after they land and re-reads
  `Inspector.svelte`, `TagVocabularyMenuItems.svelte`, `ArtistsSection.svelte` (its route) as
  landed; the design names roles, not line numbers.
