## Context

See proposal.md — Why. Facts from the code (2026-09-15): schema v5, `MIGRATIONS` in `db.rs`
run one transaction per entry. Tags are a name-keyed table plus a join, ids regenerated on
rebuild; rules and sites carry `TEXT` uuid ids restored verbatim from `library.json` because
things reference them. `Sidecar` (format 1) and `LibraryFile` (format 1) use `#[serde(default)]`
optional fields; `check_version` refuses a different number, so an additive field must not
bump either. `recover::rebuild` opens the new database through `db::open` (migrations run,
so a migration-time seed runs there too), inserts sidecars, then reads `library.json`.
`query.rs` compiles `ParsedTagSearch` (TS parses, Rust never sees the string); `has_any_tag`
is the membership shape; `tag_counts` returns `TagCounts { tags, ratings }` per search.
Bulk writes take `ids: Vec<String>`, commit, then `sidecar::write_for(ids)` (accepted cost:
one file per image, `library-sidecars` Risks). No context menu exists in app code;
`ui/context-menu` is vendored. `activeTerms` and `onrelease` arrive with `inspector-polish`;
the sidebar column with `sidebar-layout`.

## Goals / Non-Goals

**Goals:** ids are the only reference anywhere on disk; the name is data in one row; the slug
is computed in one place; one counts round trip per search; every action reachable where the
image is.

**Non-Goals:** ordering, nesting, a collection screen.

## Decisions

### D1. Schema v6: `collections` with id, name and slug; `image_collections` with a time

```
CREATE TABLE collections (
    id         TEXT PRIMARY KEY,
    name       TEXT NOT NULL,
    slug       TEXT NOT NULL UNIQUE,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);
CREATE TABLE image_collections (
    image_id      TEXT NOT NULL REFERENCES images (id) ON DELETE CASCADE,
    collection_id TEXT NOT NULL REFERENCES collections (id) ON DELETE CASCADE,
    added_at      INTEGER NOT NULL,
    PRIMARY KEY (image_id, collection_id)
);
CREATE INDEX image_collections_by_collection ON image_collections (collection_id, image_id);
INSERT INTO collections (id, name, slug, created_at, updated_at)
VALUES ('favorites', 'Favorites', 'favorites', <now>, <now>);
```
`<now>` is `CAST(strftime('%s','now') AS INTEGER) * 1000` — the migration text has no Rust
`now_ms`, and SQLite can spell the same unit. Seeding inside the migration is what makes it
run exactly once per database, for a fresh and an upgraded library alike, and never again:
"deleted stays deleted" falls out of it. The fixed id `favorites` is what lets `library.json`
and the seed agree on a rebuild (D5). Uniqueness is on the slug, not the name: `Queue` and
`queue` are one collection to a query.

### D2. Ids are uuids; the slug is computed in Rust, once

`collections::slug(name)`: trim, lower-case, every run of whitespace to `_`. Names are stored
as typed. The slug is what `collection:` compiles against and what the webview shows in a
query — it reaches the webview on every `Collection` and `CollectionCount` record, so
TypeScript never computes it. An empty slug (a blank name) is `BadRequest`.

### D3. `collections.rs`, shaped like `tags.rs` + `rules.rs`

- `list(conn) -> Vec<Collection>` by name.
- `create(library, name) -> Collection`: uuid id, `BadRequest` naming the holder on a slug
  clash. Then `sidecar::write_library`.
- `rename(library, id, name)`: same clash rule; `updated_at`; `write_library`. **No sidecar
  write** — the owner's rule and the spec's.
- `delete(library, id)`: read the member ids, delete the row (cascade), commit, then
  `sidecar::write_for(members)` and `write_library`. Deleting Favorites is allowed.
- `add(library, ids, collection_id)` / `remove(library, ids, collection_id)`: one transaction,
  `INSERT OR IGNORE` / `DELETE` per id, `added_at = now`, commit, `sidecar::write_for(ids)`.
  Membership does **not** stamp `images.updated_at`: favouriting is not an edit of the picture,
  and "Changed last" must not reorder on it (Danbooru's favourites do not touch the post).
  Every function refuses an unknown collection id — **as `NotFound`, not `BadRequest`**
  (amended when the code landed): `rules.rs` already answers `NotFound("rule {id}")` for
  exactly this, and a command rejects with the message either way, so spelling it
  `BadRequest` here would have been one module disagreeing with the rest about what an id
  nothing holds means. `BadRequest` stays what a *blank name* and a slug clash are.

### D4. The record and the sidecar carry ids; `library.json` carries the list

`ImageRecord.collections: Vec<String>` (ids, sorted), filled in `ingest::load_records` the way
tags are (one statement for the slice). `Sidecar.collections: Vec<String>` with
`#[serde(default, skip_serializing_if = "Vec::is_empty")]`: an old sidecar reads as "in
none", an old reader ignores the field; format stays 1. `LibraryFile.collections:
Option<Vec<Collection>>` with `#[serde(default)]`, always written by this build (even as
`[]`); format stays 1. `write_library` lists them.

**Amended when the code landed:** the field was to be a plain `Vec` with `#[serde(default)]`,
which was right while the only two cases considered were "the file lists the collections" and
"there is no file". There is a third: a `library.json` written by a pre-v6 build has no
`collections` key at all, and a defaulted `Vec` reads that as `[]` — "the user deleted every
collection" — so D5's `DELETE FROM collections` would drop the seed and an upgraded library
that has not rewritten its library-level file since would lose `Favorites` to its own rebuild.
`Option` is what tells the two apart: `Some([])` is an answer, `None` is a file that says
nothing.

### D5. Rebuild: the file wins, the seed yields, an orphan id becomes a placeholder

`recover::rebuild` after `insert_sidecars`: when `library.json` parses **and carries the
`collections` key** (D4's `Some`), `DELETE FROM collections` (removes the seed) and insert the
file's collections verbatim (ids, names, slugs recomputed by D2, times). When it does not — the
file is missing, unreadable, or was written before v6 — the migration's own seed is left
standing, so a rebuild of such a library ends with the one collection a freshly opened library
has rather than none. Then, for every membership id seen in the sidecars that is not in
`collections`, insert a placeholder `{ id, name: id, slug: slug(id) }` — including the case
with no `library.json` at all. Memberships are inserted per sidecar in `insert_sidecar`
*after* collections exist, so the pass is: images and tags and posts first (as today),
collections, placeholders, then memberships from the sidecars in a second walk over what
`insert_sidecars` already parsed — keep the parsed `Sidecar`s (only those naming a collection:
a library that uses none holds nothing, and one that does never holds an adapter record for the
sake of a list of ids) rather than re-reading the files. `RebuildReport` gains `collections: i64`.

### D6. The query: two fields, one clause, compiled against the slug

`ParsedTagSearch.collections` / `exclude_collections` (both sides, serde camelCase). Parser:
`-?collection:([^\s,]+(?:,[^\s,]+)*)` beside `account:`, lower-casing what is typed so
`collection:Favorites` finds `favorites`.

**Amended at review:** the parser shipped with `[a-z0-9_]+`, which was right about what a
slug usually looks like and wrong about what one *can* be. D2's `slug` only trims,
lower-cases and turns whitespace into `_`, so `To-upload` is the slug `to-upload` and `Café`
is `café`: the narrow class read the term the sidebar itself had just written as
`collection:to`, a filter matching nothing on a row that then never showed as active.
A space separates terms and a comma separates the list, so those two are the only characters
the class may refuse — anything else belongs to the slug, and the slug is Rust's to define
(D2).

Compiler: `push_collections` beside `push_accounts`:
`EXISTS (SELECT 1 FROM image_collections JOIN collections ON collections.id =
image_collections.collection_id WHERE image_collections.image_id = images.id AND
collections.slug IN (…))`, and `NOT EXISTS` for the exclusions. A slug nothing has matches
nothing, by the `IN` alone.

### D7. Counts ride on `tag_counts`

`TagCounts.collections: Vec<CollectionCount { id, name, slug, count }>`: every collection,
`LEFT JOIN` on memberships within the matched set, counted with the rating clause included
(the same question the tag list answers: "how many of what I am looking at"), ordered by
name. The webview's `results.counts` already refreshes per search and per `replace`, so the
section and the inspector need no store of their own for counts. The list of collections for
the menus comes from a small `collections.svelte.ts` store (`list`, `refresh()`, `byId(id)`),
refreshed after every create/rename/delete and on library switch (the sidebar's existing
switch effect).

### D8. Where each action lives (webview)

- **`CollectionsSection.svelte`** (sidebar, between `RatingPills` and `TagSidebar` in the
  screen's sidebar snippet): rows from `results.counts.collections`, active-first by
  `activeTerms` (extended with `collections` / `excludedCollections` slug sets), click toggles
  `toggleCollectionInQuery(query, slug)`, row menu: Rename…, Delete… (the `ConfirmDialog`
  with the count), heading action: New collection…. Rename and create share
  **`CollectionNameDialog.svelte`** (a `Dialog` with one field; Enter saves; the refusal
  message shows in the dialog).
- **`CollectionMenuItems.svelte`**: the shared submenu body — one checkbox item per
  collection (checked when every named image is in it; indeterminate is not needed: the
  action is "add all" when unchecked, "remove all" when checked), a separator, "New
  collection…". Used by the tile's context menu, the toolbar's dropdown and the inspector's
  add menu, each passing the ids it acts for and the memberships it knows (the inspector and
  the tile know one record; the toolbar knows none and shows every item unchecked).
- **`ImageCard.svelte`**: the tile's `<button>` wrapped in `ContextMenu.Root` (the comment
  at the press handler already anticipates it; the trigger is `tabindex="-1"` as that
  comment assumes). Items: the collection submenu, a separator, Move to trash / Restore (the
  overlay's own pair, so the menu is not a dead end). Ids: the selection's when the tile is in
  it (`selection.has(index, id)`), else this image's — and a right-click on a tile outside the
  selection makes it current (`onselect({})`), as a left click would.
- **`SelectionToolbar.svelte`**: a "Collection" dropdown (`ui/dropdown-menu`) with the same
  items, acting on `await selection.ids()`.
- **`Inspector.svelte`**: a "Collections" section after Tags: badges (name, active marking
  from `activeTerms` by slug, click toggles the query through `onquery`, menu: Remove from
  this collection), and an "Add to…" dropdown with the shared items. Absent when the image is
  in none and there is nothing to add to (impossible while Favorites exists; the guard is
  for a library whose user deleted every collection).
- Every action ends in `results.refresh()`? No: `add`/`remove` answer nothing, and the
  changed rows are the ones the caller named. The store re-reads `load_records` for those ids
  through the existing `search`… — simpler and already there: after a membership write the
  screen calls `results.refresh()` for a bulk write (the toolbar's pattern) and
  `results.replace(record)` for one image, where `collection_add` returns the record when
  given one id. Decision: `collection_add` / `collection_remove` return `Vec<ImageRecord>` for
  the ids written (one `load_records`), and the caller `replace`s each — no refresh, no
  re-run, the D10 rule of browse kept. Counts refresh through `replace` as they do for tags.

**Amended at review (what the webview really is):**

- **`CollectionMenuItems.svelte` renders items only.** bits-ui unmounts a menu's content the
  moment the menu closes, and the item that opens the "New collection…" dialog is what closes
  it — a dialog mounted inside the menu is destroyed in the same breath and never appears. So
  the dialog is the caller's, mounted outside its menu (`LibraryGrid` owns the one the tiles
  share, the toolbar and the inspector own their own), and the write the items and the dialog
  both end in moved to **`collection-actions.ts`**: `CollectionTarget` (`resolveIds`,
  `memberships`, `onwritten`, `onerror`), `toggleCollection`, `addToCreated`. The component
  takes one `target` prop and an `onnew`, so the ids rule is still written once.
- **One dialog per grid, not per tile.** The grid mounts and destroys tiles as it scrolls; a
  `Dialog` per tile is machinery on that path for a dialog at most one tile ever shows. The
  tile hands its own `CollectionTarget` up (`onnewcollection`), which is also what keeps
  design D8's "the selection's ids, or this image's" rule in the tile that knows it.
- **The caller `replaceMany`s the answer, not `replace` per record.** `SearchResults.replace`
  refetches the counts, so a loop over a bulk write's records asked Rust for the counts once
  per image — 25,000 round trips for the one number they all change. `replaceMany` swaps every
  row through one index and loads the counts once.
- **`CollectionsSection` refreshes both readers.** A create, rename or delete changes the
  counts this section draws (its `onchanged` prop, wired to `results.refresh()` — not a
  re-issued query, which would reset the selection) *and* the store every menu lists from, so
  it calls `collections.refresh()` itself. One without the other leaves the tile menu offering
  a collection the sidebar just renamed or deleted.
- **"New collection…" means different things in a menu and in the sidebar, on purpose.** From
  one of the three menus it creates *and adds* the ids the menu was opened for: every other row
  in that list writes membership, and creating without writing would make the one thing the
  user opened the menu to do take two trips. The section's own heading action has no ids in
  scope and creates an empty collection.
- **The delete confirmation names the search's count, and says so.** The only count in hand is
  D7's, which is per search; nothing answers "how many in the library" while a search is on, so
  the sentence names which number it is rather than implying the other one.
- **The toolbar offers Add/Remove per collection, not a checkbox.** "The toolbar knows none and
  shows every item unchecked" (above) was right for what T2 built from it: every item unchecked
  reads as "add", and the spec sentence in view at the time only asked for adding from the
  toolbar. It stopped being enough once the requirement was read whole: "An image is put into
  and taken out of collections from where it is shown" names the selection toolbar among the
  places offering *both*, and an always-unchecked item can never drive a remove — the range a
  toolbar selection can name spans rows the app has not loaded, so there is no membership to
  check a box against in the first place, not just none rendered. `CollectionMenuItems` now
  branches on `target.memberships() === null`: with memberships (the tile, the inspector) a row
  stays the checkbox item; with none (the toolbar) a row opens a submenu of two plain items,
  "Add selection" and "Remove selection", each resolving ids the same way the add does today
  and `replaceMany`-ing the answer. "Remove" on images not in the collection is a no-op on the
  Rust side already — `collections::remove` deletes what exists, nothing to guard in the
  webview. The branch lives in `CollectionMenuItems` itself (it already branches on `target`
  for the checkmark), behind an optional `sub` snippet prop so the tile's and the inspector's
  menus, which never hit the `null` branch, need no change.

### D9. Bulk sidecar cost, accepted and named

Adding 25,000 images to Favorites writes 25,000 sidecars while holding the library (the
`library-sidecars` risk, unchanged). The owner knows ("it's included in my plan"). Not built:
the per-item-lock shape from `backfill_sidecars`. `FIXME` at the call site naming that shape.

## Risks / Trade-offs

- [Schema v6 locks out older builds] → the existing `SchemaTooNew` path already tells the user
  which build to use; the release notes name it.
- [A rename changes the slug, so a saved query stops matching] → the sidebar shows the new
  slug's row at zero when the search names an old one (the tag list's own rule, "an active
  term with no matches is still listed"); the user re-clicks.
- [The placeholder name is a uuid] → visible only after a rebuild with a lost `library.json`,
  and renameable; better than silently dropping memberships.
- [`ContextMenu` around every tile] → bits-ui renders the trigger inline and the content only
  when open; the grid draws only visible tiles. Measured in the hand check on the 25k vault.

## Open Questions

None that change the specs or the split. The migration number is claimed by queue position
(v6); the implementing agent amends D1 if `MIGRATIONS.len()` says otherwise.
