## Context

- `rules.rs` holds `ARTIST_FIELDS` (`x`→`handle`, `pixiv`→`artist`) and `artist_tag(adapter)`;
  `ingest.rs` `resolve_tag_text` derives the artist tag beside the rules' tags and `insert_rows`
  links it last with `Conflict::Skip` (`auto-artist-tag` D2, D3). `input.page_url` is available
  there. Records are `SiteAdapterRecord { site, fields }`, stored verbatim as `adapter_json`.
- Schema is v11 (`MIGRATIONS.len()`); `library.json` is `sidecar::LibraryFile` with additive
  `Option<Vec<_>>` keys under `#[serde(default)]`, written by `write_library`, restored by
  `recover::rebuild`. `model.rs` and `packages/shared/src/index.ts` are mirrored by hand, with a
  `…crosses_the_wire_in_camel_case` test per type.
- `tags.rs`: `link_tags(conn, image_id, &TagText, Conflict)`, `remove_tags`, `collect_orphans`
  (general, unpinned, carrier-less rows only), `apply_edit` (bulk, one transaction, sidecars
  after commit), `canonical`, `underscored`; `tags(name UNIQUE, category, pinned_group)`.
- `query.rs` `host_and_path(url)` splits scheme, authority and path by hand; there is no `url`
  crate and no normaliser (`facts.rs` argues against one).
- The Pixiv fixture has `<a data-gtm-value="3439325" href="/users/3439325">` inside `main h2`;
  `pixiv.test.ts` asserts whole-object equality of `extract(...)`.
- The Inspector's tag menu (`Inspector.svelte` ~line 945) has `image: ImageRecord | null` in
  scope with `image.adapter` and `image.pageUrl`. `menu-polish` is reordering that menu right
  now; this change's UI unit runs after it lands.

## Decisions

**D1. One table, `artist_urls (url TEXT PRIMARY KEY, tag TEXT NOT NULL)`, grouped into
entries.** An artist entry on the wire is `ArtistEntry { tag: string, urls: string[] }`,
`artists::list` groups the rows by tag, ordered by tag then URL. A separate `artists` table
would add nothing: an artist with no URL matches nothing, and the tag's category lives in
`tags` already. `url` is stored normalised (D3) so the primary key is also the uniqueness rule
"one owner per URL": upserting a URL another artist owns is refused naming that artist rather
than moved silently; two lines of one list that normalise to the same URL are one URL; a tag
that exists under a category other than artist is refused. `tag` is stored canonical (`tags::underscored`). Migration `SCHEMA_V12`
(the real number is `MIGRATIONS.len()` when unit R lands; amend this sentence, never pin it)
creates the table and an index on `tag`.

**D2. `library.json` carries `artists: Option<Vec<ArtistEntry>>`** under `#[serde(default)]`,
like `stamps`: `None` for a file older than this change, `Some(empty)` for none. `write_library`
includes it; `recover::rebuild` deletes and reinserts when `Some`. Every write to `artist_urls`
rewrites `library.json`, as `set_category` does — entries are vocabulary, not per-image state.

**D3. URL normalisation and prefix matching, in `artists.rs`, no crate.** `normalized(url) ->
Option<String>`: accept a missing scheme by treating the text as `https://` + text; require a
host; lowercase the host; drop a leading `www.`, `mobile.` or `m.`; map `twitter.com` to
`x.com`; drop query and fragment; drop every trailing `/`; lowercase the path (X handles are
case-insensitive and Pixiv ids are digits; a path elsewhere is lowercased too, accepted and
said in the doc comment). The result is `host/path` with no scheme, so `http` and `https`
compare equal. The text is trimmed first; a host without a dot or with whitespace in it, and
an empty path, are refused (`None`): `metaljelly` is a name, not a URL, and a bare `x.com`
would own every X capture. `owns(entry_url, candidate) -> bool` is `candidate == entry_url ||
candidate.starts_with(&format!("{entry_url}/"))` — the boundary is what keeps
`x.com/metaljelly0811` from owning `x.com/metaljelly08110`. `host_and_path` moves from
`query.rs` into this module (or a tiny shared `urls.rs`) and `query::x_account` calls it there;
one splitter, two readers.

**D4. Derivation: the entry that owns the profile URL first, the field second.**
`artists::derive(adapter, entries) -> Option<String>` builds the one candidate — the author's
profile URL from the record (`PROFILE_URLS`: `x` → `https://x.com/{handle}`, `pixiv` →
`https://www.pixiv.net/users/{userId}`; a row per site the app reads), normalised — and takes
the entry that owns it (with nested owners, the longest). The page URL and the record's post
URL are not candidates: the page URL is the tab's address, which on X is `x.com/home`, a
search, or another user's profile when a repost is captured — an entry made to own it would
tag every later timeline capture with that artist, and the rename dialog would have offered it
as a prefill (reviewer, 2026-09-25); the post URL is built from the same handle as the profile
URL and adds nothing. So a record with no profile URL (Pixiv before `userId`, any other site)
consults no entry and falls back as before. The fallback is today's `artist_tag` (moved here
with `ARTIST_FIELDS` and its tests). An entry whose tag exists under a category other than
artist (`upsert` and `rename` refuse one, but `set_category` can change a tag later) is skipped
and the field decides, so a capture never loses its artist tag to a stale entry.
`ingest::resolve_tag_text` reads the entries with `artists::list(conn)` and calls `derive`;
nothing else in ingest changes — the `Conflict::Skip` link, the order after the rules, the
legacy-bundle exemption all stand (`auto-artist-tag` D2, D3, D5). Every X and Pixiv capture
still yields exactly one artist name, and the category rule decides whether it is linked, as
before (owner, 2026-09-25: "make sure every save got one auto-tagged artist"). A record from
any other site yields none, entries or not (`capture-ingest`, "A site with no author field the
app reads").

**D5. `rename_artist` is one transaction: the entry, then the retag.**
`RenameArtistInput { from: string, to: string, urls: string[] }` →
`RenameArtistReport { retagged: number, merged: boolean }`. Steps, refusing before any write:
`from` must be an artist tag (the proposal's non-goal: only an artist renames);
`to` canonicalised with `underscored`, empty refused; `to` existing under a category other than
artist refused with the reason (the editor's `Conflict::Refuse` wording); a URL that does not
normalise refused naming it; a URL owned by a third artist refused naming that artist. Then:
rows with `tag = from` become `to`; each given URL is upserted to `to`; the carriers of `from`
(trash included — a restored image must not come back with the stale name) are retagged: when
no `to` row exists, `UPDATE tags SET name = to, category = 'artist' WHERE name = from` keeps
the pinned group and every link in one statement (`merged: false`); when it exists, links move
row by row (`INSERT OR IGNORE` into `image_tags`, delete the old links), a pinned group on
`from` moves to `to` if `to` has none, and the `from` row is deleted (`merged: true`) — the
whole point is that the old name has no carrier-less row left. `mark_updated` for every carrier;
commit; then `sidecar::write_for_records` for the carriers and `write_library`. Returned to the
webview so the dialog can say what happened. `rename_artist_preview(input: { from, adapter,
pageUrl }) -> { carriers: number, urls: string[] }` is the second command, answered before
anything is written: the carrier count (trash included) and the profile URL D4 builds from
that image's record — one line, or none for a record with no profile URL — normalised and
shown with `https://` in front. The webview
never builds a profile URL itself: one runtime knows how a record becomes a URL, so what the
dialog prefills is exactly what the match will read.

**D6. The door is the inspector's artist tag, and only there.** In `Inspector.svelte`'s tag
menu, for a tag whose category is `artist`, one item "Rename artist…" with `PencilIcon`
(`menu-polish` D4's well-known set) opens `RenameArtistDialog.svelte`
(`components/tags/`), placed in `Inspector.svelte`'s own menu right after the two search items,
before `TagVocabularyMenuItems` — not inside that shared component, which also backs the
pinned chip's and the sidebar row's menus, where the item must not appear (an image is needed
for the URL and the count). The sidebar's
row and the pinned chip do not offer it: the URLs come from an image, and those two have none
in scope. The dialog: "New name" (prefilled with the old, focused, selected), "Profile URLs"
(a textarea, one per line, prefilled with `rename_artist_preview`'s `urls` — the profile URL,
or empty for a record without one, where the user may paste the artist's page), a line "N
images carry `from` and will be retagged `to`. Future captures from these URLs are tagged
`to`." with N from the same preview, a refusal line for the command's
errors, Cancel, and "Rename N images". After success: `vocabulary.refresh()` and the current
search re-run through the store's existing refresh (the UI unit names the method in its
handoff), so the panel and the grid show the new name.

**D7. Settings → Artists, above Rules.** `components/artists/ArtistsSection.svelte`: heading,
one paragraph saying what an entry does and the notice "Changing URLs applies to future
captures only; images already in the library keep their tags. To retag images, rename the
artist from an image's inspector." Then a stacked list like `rules-panel-layout` D1: per
artist, the tag (in the artist colour, `CATEGORY_TEXT_CLASS.artist`) with edit and delete at
the right, the URLs under it as `font-mono text-xs break-all` lines. Edit opens a form (tag
read-only, URLs textarea) → `artists_upsert(entry)` replaces the tag's URL set. "Add artist"
opens the same form with the tag editable. Delete confirms with "Future captures from these
URLs fall back to the handle or display name; no image changes." A FIXME on the section names
the missing "run over existing images" pass (D9's shape in `rules.rs`).

**D8. Commands and wire types.** `artists_list() -> ArtistEntry[]`, `artists_upsert(entry:
ArtistEntry) -> ArtistEntry[]`, `artists_delete(tag: string) -> ArtistEntry[]`,
`rename_artist_preview(input: RenameArtistPreviewInput) -> RenameArtistPreview`, `rename_artist(input: RenameArtistInput) ->
RenameArtistReport`; TS wrappers in `api/commands.ts` with the invoke tests; `ArtistEntry`,
`RenameArtistPreviewInput`, `RenameArtistPreview`, `RenameArtistInput`, `RenameArtistReport` in `model.rs` and `shared/src/index.ts` with the wire
test. The Pixiv record gains `userId` (extension `pixiv.ts`: the digits after `/users/` in the
`main h2 a[href^="/users/"]` href; fixture assertions updated); `site-adapters` spec lists it.

**D9. `auto-artist-tag` D4 is amended, not reversed.** D4 answered a renamed artist with a
manual bulk retag and said a stable id was unnecessary as the tag. The tag stays the readable
name; what changes is that the correction is an app action that also remembers the source
(the URL with the id) so the next capture is right. Recorded in the archive under D4.

## Risks / Trade-offs

- [The retag walks every carrier in one transaction] → the owner's largest artist is hundreds
  of images, and `apply_edit` already does the same per bulk edit; a rename is rare.
- [Lowercasing the path in `normalized`] → wrong only for a case-sensitive profile path on a
  site the app does not read today; said in the doc comment.
- [Old Pixiv captures have no `userId`] → the rename's retag by name fixes them; new captures
  need the updated extension (hand check).
- [`host_and_path` moving modules] → `query.rs` tests for `x_account` keep passing unchanged.
