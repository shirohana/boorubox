> Three units. Unit R (Rust + shared + the TS invoke wrappers) and unit X (extension) run in
> parallel, Sonnet each, disjoint packages; unit U (webview) runs after R has landed and after
> `menu-polish` has landed (both edit `Inspector.svelte`'s tag menu). Retried on Opus if the
> gate fails; one Opus review of the whole change. Design D1–D9 decide every shape; do not
> re-decide them. The migration's number is whatever `MIGRATIONS.len()` is when unit R lands
> (planned v12; 11 today): amend design D1's sentence to the real number, never pin the planned
> one. Gates: unit R `mise run check` (the whole gate — it owns Rust); unit X `pnpm lint &&
> pnpm typecheck && pnpm --filter @boorubox/extension test`; unit U `pnpm lint && pnpm
> typecheck && pnpm test`. No unit commits or ticks a hand check.

## 1. Unit R — entries, matching, derivation, rename, commands (`packages/app/src-tauri`, `packages/shared`, `packages/app/src/lib/api`)

- [x] 1.1 `artists.rs` (new, registered in `lib.rs`'s module list): `host_and_path` moved
      here from `query.rs` as `pub(crate)` (and `query::x_account` calls it; its tests stay
      green unchanged), `normalized`, `owns`, `PROFILE_URLS`, `ARTIST_FIELDS` and `artist_tag`
      moved from `rules.rs` with their tests, `candidates(adapter, page_url) -> Vec<String>`,
      `derive(adapter, page_url, entries) -> Option<String>` per D3/D4. Tests: the spec's
      "A prefix owns the post" cases (`twitter.com` + capitals + query, `http://www.` +
      trailing slash, the `08110` neighbour), a schemeless URL, a URL with no host is `None`,
      the longest owning entry wins, fallback to the field when nothing owns, Pixiv `userId`
      candidate, a record without `userId` yields no Pixiv profile candidate.
- [x] 1.2 `db.rs`: `SCHEMA_V12` (the real number) per D1 with its doc comment; test
      `a_v11_library_migrates_gaining_the_artist_urls_table` in the shape of the v10→v11 test.
- [x] 1.3 `model.rs` + `packages/shared/src/index.ts`: `ArtistEntry { tag, urls }`,
      `RenameArtistPreviewInput { from, adapter: SiteAdapterRecord | null, pageUrl: string |
      null }`, `RenameArtistPreview { carriers, urls }`, `RenameArtistInput { from, to, urls }`,
      `RenameArtistReport { retagged, merged }`; wire test
      `artist_entry_and_rename_types_cross_the_wire_in_camel_case`.
- [x] 1.4 `artists.rs` store: `list(conn)`, `upsert(library, entry)` (replace the tag's URL
      set; refusals per D1/D5: empty tag, a URL that does not normalise, a URL owned by another
      artist — each naming the offender), `delete(library, tag)`; every write ends in
      `sidecar::write_library`. `sidecar::LibraryFile.artists: Option<Vec<ArtistEntry>>`
      (`#[serde(default)]`), written by `write_library`, restored by `recover::rebuild` per D2.
      Tests: list groups by tag in order; upsert replaces; the one-owner refusal names the
      owner; an old `library.json` without the key restores none; a rebuild restores two URLs.
- [x] 1.5 `ingest.rs` `resolve_tag_text`: `artists::list(conn)` + `artists::derive` per D4,
      doc comment updated to say the entry is read first and why. Tests (the four new
      `capture-ingest` scenarios): entry owns the handle → `metaljelly`, not `metaljelly0811`;
      entry owns the Pixiv user via `userId`; the neighbour handle is not owned; an old Pixiv
      record without `userId` falls back to the display name.
- [x] 1.6 `artists.rs`: `rename_preview(conn, input)` and `rename(library, input)` per D5,
      one transaction, refusals before any write, sidecars and `library.json` after commit.
      Tests: fresh rename keeps the pinned group and every link and reports `merged: false`;
      merge into an existing artist moves the links, the pin moves when the target has none,
      the old row is gone, `merged: true`; a name taken by a character is refused and nothing
      changes; a trashed carrier is retagged; the given URLs are owned by `to` afterwards and
      URLs `from` owned moved with it; each carrier's `updated_at` moved; `library.json` lists
      the entry; preview counts trash and lists the candidates with `https://`.
- [x] 1.7 `commands.rs` + `lib.rs`: `artists_list`, `artists_upsert`, `artists_delete`,
      `rename_artist_preview`, `rename_artist` per D8, registered; one commands test in the
      shape of `set_tag_category_and_set_tag_pinned_group_reach_the_open_library…`.
      `packages/app/src/lib/api/commands.ts`: the five wrappers, exported wherever the rules
      wrappers are; `commands.test.ts`: one invoke test per wrapper.
- [x] 1.8 `openspec/changes/archive/2026-09-24-auto-artist-tag/design.md` D4: append a dated
      paragraph per D9. `rules.rs` module doc no longer claims the artist derivation.
- [x] 1.9 Gate `mise run check` green. Handoff: the real migration number, the exact command
      names and wrapper names, the refusal messages' wording, and how `rename` reports the
      carriers (count only).

## 2. Unit X — the Pixiv user id (`packages/extension`)

- [x] 2.1 `adapters/pixiv.ts`: `userId` from the `main h2 a[href^="/users/"]` anchor's href
      (the digits after `/users/`), with the same fallback anchor `readArtist` uses; the
      header comment names it. `pixiv.test.ts` whole-object assertions gain `userId:
      '3439325'` (the fixture's id — verify it in the fixture, do not assume). A test for a
      page whose artist anchor is missing yields no `userId`.
- [x] 2.2 Gate green (`pnpm lint && pnpm typecheck && pnpm --filter @boorubox/extension test`).
      Handoff: the field name as shipped.
- [ ] 2.3 Hand check (owner): load the rebuilt extension, capture from a Pixiv artwork, the
      stored record shows `userId` in the inspector's capture details.
      Hand check: load the rebuilt `packages/extension` build, capture an image from a live
      Pixiv artwork page, and confirm the stored capture's record shows `userId` (the digits
      of the artist's `/users/<id>` path) in the inspector's capture details.

## 3. Unit U — the rename dialog and Settings → Artists (`packages/app`)

- [x] 3.1 `components/tags/RenameArtistDialog.svelte` per D6: props `{ from, image, onclose,
      onrenamed }`; on open calls `renameArtistPreview`, prefilled name and URLs, the count
      line, the refusal line, "Rename N images" disabled until the preview answers; on success
      `onrenamed(report)`. Follow `CollectionNameDialog.svelte`'s dialog shape.
- [x] 3.2 `Inspector.svelte` tag menu: "Rename artist…" with `PencilIcon` for
      `vocabulary.categoryOf(tag) === 'artist'`, placed per D6; after a rename
      `vocabulary.refresh()` and re-run the current search through the search store's existing
      refresh method (find it in `api/search.svelte.ts`; name it in the handoff). The dialog is
      mounted outside the menu content (bits-ui destroys menu content on close — the
      `CollectionNameDialog` mount in the inspector is the pattern).
- [x] 3.3 `components/artists/ArtistsSection.svelte` (+ `ArtistForm.svelte` if the form is
      more than a few lines) per D7; mounted in `routes/settings/+page.svelte` above
      `RulesSection` with its own "Slot Settings · Artists" comment; the FIXME per D7.
- [x] 3.4 Gate green. Handoff: the search refresh method used; anything the smoke test should
      look at.
- [ ] 3.5 Hand check (owner): on the real vault, right-click an X artist tag → Rename artist…
      shows the handle's profile URL and the carrier count; renaming retags the images and the
      sidebar shows the new name; a fresh capture from that handle carries the new name;
      Settings → Artists lists the entry, deleting it restores the handle for the next capture.
      Hand check: open a library with an X-captured image, right-click its artist tag (the
      red-text one), choose "Rename artist…", confirm the name field opens with its text
      selected, the dialog shows the handle's `https://x.com/<handle>` URL and the correct
      carrier count (not "Reading…" stuck forever), rename it, and confirm the tag text on screen
      (panel and grid) updates without a manual reload. Then open Settings → Artists, confirm the
      new tag is listed with that URL, edit its URLs, delete it, and confirm the list and the
      width both look right (no sideways scroll).
      Seen (lead's smoke, 2026-09-25, on the X-captured image of test-1): the dialog opened from
      the viewer's inspector above the viewer, the name selected, `https://x.com/iv70311741`
      prefilled, "1 image carries “smoke_artist” and will be retagged …"; Cancel left the
      viewer open. Renaming to `smoke_renamed` from the grid inspector updated the inspector,
      the tile's tag line, the sidebar and the pinned chip, and `smoke_artist` was gone.
      Settings → Artists listed the entry as `x.com/iv70311741` (the stored, normalised
      form); editing added a URL; delete asked with the future-captures wording and removed
      it. Not seen: a fresh capture from the renamed handle. Screenshots:
      `boorubox-vault/smoke-2026-09-25/D9-rename-dialog-in-viewer.png`, `D14-after-rename.png`, `D16`, `D19`.

## Handoff

- Unit X: the Pixiv record's new field is `userId` (a string of digits, e.g. `'3439325'`),
  read by `readUserId` in `packages/extension/src/adapters/pixiv.ts` from the
  `main h2 a[href^="/users/"]` anchor's href, falling back to the same
  `main a[href^="/users/"] img[src*="/user-profile/"]` anchor `readArtist` falls back to (its
  closest ancestor anchor, via `:has()`). No deviation from the design: the fixture's id
  matched D8's quoted `3439325`. `PROFILE_URLS`'s Pixiv template in D4
  (`https://www.pixiv.net/users/{userId}`) matches this field name.

- Unit R (for unit U): the real migration number is **v12** (`SCHEMA_V12` in `db.rs`,
  `MIGRATIONS.len() == 12`), exactly the planned number — no amendment to D1's sentence was
  needed.
  - Commands (Rust, registered in `lib.rs`) and their TS wrappers (`packages/app/src/lib/api/
    commands.ts`), one to one: `artists_list` → `artistsList()`, `artists_upsert(entry:
    ArtistEntry)` → `artistsUpsert(entry)`, `artists_delete(tag: string)` → `artistsDelete(tag)`,
    `rename_artist_preview(input: RenameArtistPreviewInput)` → `renameArtistPreview(input)`,
    `rename_artist(input: RenameArtistInput)` → `renameArtist(input)`. All five, and the five
    wire types (`ArtistEntry`, `RenameArtistPreviewInput`, `RenameArtistPreview`,
    `RenameArtistInput`, `RenameArtistReport`), are exported from `@boorubox/shared` and
    `$lib/api` exactly where the `rules*`/`Rule*` ones are (`export * from './commands'` in
    `api/index.ts` already covers the new wrappers; no separate export list to edit).
  - Refusal wording (`AppError::BadRequest`, read by the dialog/section as plain text):
    - Empty tag/name: `"an artist needs a tag"` (`artists_upsert`), `"an artist needs a name"`
      (`rename_artist`).
    - A URL that does not normalise: `"{url:?} does not look like a URL"` (the URL in Rust
      `Debug` quoting, e.g. `"https:///no-host" does not look like a URL"`).
    - A URL another artist owns: `"{url:?} is already owned by {owner:?}"`, e.g. `"https://
      x.com/alice_art" is already owned by "alice"` — `owner` is the tag name, unquoted content
      but Rust-`Debug`-quoted like the url.
    - Renaming onto a name a non-artist category already owns: `tags::category_conflict`'s
      existing wording, unchanged — e.g. `"miku" is a character tag and cannot become an artist
      tag; use another name, e.g. miku_(artist)"`.
    - Renaming a `from` that has no entry/tag row at all: `AppError::NotFound("artist tag
      {from}")` — not a `BadRequest`; the dialog should treat this the same as any other
      "image not found"-style refusal it doesn't expect to hit from a real artist tag menu.
  - `rename_artist`'s `RenameArtistReport` is `{ retagged: number, merged: boolean }` —
    `retagged` is the carrier **count**, not the list of ids; the dialog has no per-image detail
    to show beyond the count `rename_artist_preview` already gave it.
  - `rename_artist_preview`'s `RenameArtistPreview.urls` are already prefixed `https://` and
    already deduplication-free in whatever order `candidates()` built them (profile URL, then
    `postUrl`, then `pageUrl`, each only if it normalises) — safe to show one per line as is.
  - Deviations from the design: none in shape. One test-only surprise worth knowing for unit U's
    own hand-check design reading: `tags::place_pinned`'s `PinTarget::Group(n)` clamps `n` down
    to "one past the last existing group" — so pinning the *first* tag in a library straight into
    "group 2" actually lands it in group 1. Not a rename-specific behaviour, pre-existing in
    `pinned-tag-groups`, but it tripped one of this unit's own test fixtures.
  - Open items for unit U: none from this unit's side. `Inspector.svelte`'s tag menu and the
    search-refresh method are unit U's own reads (`menu-polish` will have landed on that file by
    the time unit U starts, per the coordinate brief).

- Unit U: the search-refresh method used after a rename is `results.refresh()`
  (`api/search.svelte.ts`, "Re-runs the current query, for after a command changed the library" —
  the same one `saveTags`/`saveFacts` leave to their own callers rather than calling themselves,
  since a rename is not a per-image write). `RenameArtistDialog`'s `onrenamed` in `Inspector.svelte`
  calls both `vocabulary.refresh()` and `results.refresh()`.
  - "Rename artist…" sits inside the groupedTags tag menu, not inside
    `TagVocabularyMenuItems.svelte` — that shared component also backs the pinned chip's own menu
    (`Inspector.svelte` line ~787), which the spec says must never offer this item, and it has no
    extension point (no snippet prop) to insert into. It is its own item + separator, right after
    the "Search for this tag"/"Exclude from the search" pair and before `TagVocabularyMenuItems`,
    guarded the same as everywhere else (`vocabulary.categoryOf(tag) === 'artist'`), absent from
    the sidebar row and the pinned chip. D6 now records this position as the design (amended
    after this unit landed) — not a deviation from it.
  - Design note for the smoke test: `design.md` D4/D5 make the profile URL the *only*
    `derive`/preview candidate — the page URL and the post URL are dropped, so
    `rename_artist_preview`'s `urls` is zero or one entry, never more. `RenameArtistDialog`
    renders whatever `RenameArtistPreview.urls` comes back, one per line, so it works either way —
    the 3.5 hand check should expect the profile URL prefilled, or none for a record without one,
    not "up to three".
  - No Svelte component tests exist anywhere in `packages/app` (checked before writing this
    unit) — only pure-logic `.ts` modules are unit-tested. `RenameArtistDialog.svelte`,
    `ArtistForm.svelte` and `ArtistsSection.svelte` have no tests of their own, matching
    `CollectionNameDialog`/`RuleForm`/`RulesSection`'s own precedent; the gate is lint + typecheck
    + the existing suite staying green.
