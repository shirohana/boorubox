## Context

See proposal.md — Why. Every archived change has landed; `MIGRATIONS.len()` is 9 in the
checked-out tree (v9 is `lowercase-tags`' Rust step). What the code holds:

- `collections (id TEXT PK, name, slug UNIQUE, created_at, updated_at)` and
  `image_collections` (v6). `collections.rs`: `list` (by name), `create`, `rename`, `delete`
  each rewrite `library.json`; `add` / `remove` (split into `add_in` / `remove_in` for
  `tags::apply_edit`) never move `images.updated_at` and never rewrite `library.json`.
- `library.json` is `sidecar::LibraryFile`, Rust only (the shared package has no mirror of it).
  Its `collections: Option<Vec<Collection>>` serializes the model struct itself, and
  `recover::insert_collections` restores each entry **by its id, verbatim** (`collections`
  design D5; spec `collections`, "A collection survives a rebuild by its id"). A membership
  whose collection the file does not list becomes a placeholder named by the id.
- Pinned tags (`tag-vocabulary` D1, D8): `tags.pinned` (v7), `tags::set_pinned(library, name,
  pinned)` refusing an unknown name with `NotFound` and answering the whole vocabulary; command
  `set_tag_pinned`; store `vocabulary.pinned` / `setPinned`. The inspector's `pinnedChip`
  snippet draws a chip per pinned tag: one image reads `tags.includes(tag)` and writes the
  whole toggled set through `results.saveTags`; a selection reads `selectionTagCounts(ids, 0,
  names)` in one `$effect` keyed on the pinned names, `selection.generation` and
  `results.generation`, and writes through the inspector's `onedit(ids, add, remove)` → the
  screen's `editSelectionTags` → a `PendingWrite` `edit` (asks past one image) → `applyEdit`
  → `afterWrite`. The single-image strip is drawn only while the tag editor is closed.
- `ImageRecord.collections` carries an image's collection ids, so one image's membership is
  known in the webview. A selection is not: `SearchResults` holds loaded pages, and a range
  selection (select-all over 25,000) spans rows the webview never loaded — the same reason
  `CollectionMenuItems` shows Add/Remove submenus rather than checkboxes for the toolbar
  (`collections` D8, amended). There is no per-collection count over a set of ids today:
  `query.rs`' `collection_counts` counts over the *search's* matched set, not a selection.
- Collection writes in the inspector go through `collection-actions.ts`: `toggleCollection(
  target, id)` over the inspector's `collectionTarget` (`resolveIds` = the image,
  `memberships` = its own set, `onwritten` = `results.replaceMany` + `onrelease`).

## Goals / Non-Goals

**Goals:** the collection pin is the tag pin's twin in storage, in the menu and in the chip, so
nothing about pins has two answers; no new write door — the one-image chip writes through
`collection-actions`, the selection chip through `apply_edit`'s collection lists; one fetch
effect in the inspector for both kinds of chip.

**Non-Goals:** a generic "pinnable" abstraction over tags and collections (two tables, two
keys, two stores; the shared part is the chip's drawing and the tri-state rules, and those are
shared below); pin order other than name order.

## Decisions

### D1. `collections.pinned`, one migration

Migration v10 (`SCHEMA_V10` in `db.rs` — `MIGRATIONS.len()` was 9 when unit R landed, so this
is the tenth entry):

```sql
ALTER TABLE collections ADD COLUMN pinned INTEGER NOT NULL DEFAULT 0;
```

with a doc comment in `SCHEMA_V7`'s voice: the pin is a property of the collection, not of any
membership, so it sits on the row; the default is the unpinned collection every existing row
already was, so an upgraded library's collections all read back unpinned. `Favorites` is
seeded unpinned by v6 and is not pinned by this migration: pinning is the user's choice, and
the seed was never a claim about which collection the user drops images into.

`Collection` (`model.rs`, `packages/shared`) gains `pinned: bool` / `pinned: boolean`.
`COLLECTION_COLUMNS` and `row_to_collection` gain the column. `CollectionCount` does not: the
sidebar's counts row never needs it.

*Why not a settings key (per machine):* a pin is library state — it has to follow the library
folder to the other machine and come back from a rebuild, exactly as a tag pin does. `settings.json`
is per machine and is not in the folder.

### D2. `library.json` carries the pin on each collection entry, keyed by id

No new key on `LibraryFile`: `collections: Option<Vec<Collection>>` already serializes the
struct, so `pinned` rides on each entry the moment the struct has it. On the model field:
`#[serde(default)]`, so a file written before this change (entries with no `pinned`) reads
every collection as unpinned — the same answer the migration's default gives — and the format
stays 1 (an old reader ignores the unknown key). `recover::insert_collections` writes the
column (`INSERT … pinned` from `collection.pinned`); `insert_placeholder_collections` leaves it
at the default, so an orphan id comes back unpinned.

*By id or by slug:* by id — or rather, not keyed separately at all. The brief's worry that "ids
may not survive a rebuild" does not hold in this tree: ids are restored verbatim from the file
(`collections` D5), and every membership in every sidecar is already keyed by that id. A
separate `pinnedCollections: [slug]` list would be a second source of truth for a fact the
collection entry can carry itself, would have to be rewritten on every rename (the slug moves
with the name), and would need its own rule for a slug the collections list does not name.

*Why not follow `tags`' exceptions-only shape:* the vocabulary lists only the non-default tags
because a library has thousands of general tags; it has tens of collections and already lists
every one of them.

### D3. `collections::set_pinned` and the command `set_collection_pinned`

`collections::set_pinned(library, id, pinned) -> Result<Vec<Collection>>`: `require_collection`
first (`NotFound("collection <id>")` for an unknown id — the twin of `tags::set_pinned`'s
refusal, keyed by id because the collection's id is its identity and its name is not), `UPDATE
collections SET pinned = ?1 WHERE id = ?2`, `sidecar::write_library`, then `list(conn)` as the
answer. It does **not** move `collections.updated_at`: that time records the row's own name
edits (`rename` moves it), and a pin is a display preference, the same reason `tags::set_pinned`
touches no time. No sidecar is written: no image changed (spec, "Pin touches one file").

Command `set_collection_pinned(id: String, pinned: bool) -> Vec<Collection>`, registered in
`lib.rs` beside the collection commands; `api/commands.ts` `setCollectionPinned(id, pinned)`.

*Why answer the whole list:* the store replaces `list` with the answer, the vocabulary store's
own shape (`tag-vocabulary` D5), so no refresh round trip follows a pin.

### D4. The selection's tri-state: `collections::selection_counts`, a Rust count over the ids

`collections::selection_counts(conn, ids, collection_ids) -> Result<Vec<CollectionCount>>`: for
exactly the named collections, how many of `ids` are in each. Empty `ids` or empty
`collection_ids` answers empty. Chunked by `query::ID_CHUNK` with `query::placeholders`, summed
in memory, the same shape as `tags::selection_tag_counts` and for the same reason (a whole-library
selection is past SQLite's variable limit):

```sql
SELECT collections.id, collections.name, collections.slug, COUNT(*)
FROM image_collections JOIN collections ON collections.id = image_collections.collection_id
WHERE image_collections.image_id IN (…) AND image_collections.collection_id IN (…)
GROUP BY collections.id
```

`query.rs`' private `text_values` becomes `pub(crate)` and this function uses it; `tags.rs`'
private copy of it goes, and `tags.rs` uses the one in `query.rs` too. A collection none of `ids` is in is
absent from the answer, as a tag is from `selection_tag_counts`. Command
`selection_collection_counts(ids: Vec<String>, collection_ids: Vec<String>) ->
Vec<CollectionCount>`; `api/commands.ts` `selectionCollectionCounts(ids, collectionIds)`.

*Why not the selection's `ImageRecord.collections`:* the records exist only for loaded pages;
a range selection over rows never loaded would read as "none of them" and the next click would
add rather than remove — a wrong write, not only a wrong fill. *Why not widen
`selection_tag_counts`:* it answers `TagCount { name }` over `image_tags`; a collection is keyed
by id over a different table, and a flag switching the table is two functions in one body.
*Why reuse `CollectionCount`:* it is already "one collection plus how many of a set are in it"
(`collections` D7); a new type would be the same four fields.

### D5. The store: `collections.pinned` and `collections.setPinned`

`api/collections.svelte.ts` gains `pinned = $derived(this.list.filter((c) => c.pinned))` — the
list is already in name order from Rust, so the chips need no sort of their own — and
`setPinned(id, pinned)` which sets `list` from `setCollectionPinned`'s answer and `error` from a
refusal, the vocabulary store's `setPinned` shape. Deleting a pinned collection needs nothing
new: `CollectionsSection` already refreshes the store after a delete, the collection leaves
`list`, and its chip leaves `pinned` with it. A rename updates the chip's name the same way.

### D6. One menu item: `components/tags/CollectionPinMenuItem.svelte`

A `ContextMenu.Item` reading `Pin` or `Unpin` from `collection.pinned`, calling
`collections.setPinned(collection.id, !collection.pinned)`; prop `collection: Collection`.
Snippet-free for `TagVocabularyMenuItems`' reason: every mount is a `ContextMenu.Root`. Mounted
in three places:

- `CollectionsSection.svelte`'s row menu, first, then a separator, then Rename… / Delete…
  (the tag row's menu leads with Pin the same way).
- The inspector's collection badge menu, after "Remove from this collection" and a separator.
- The collection chip's own menu (D7), where it always reads Unpin, by construction.

*Why its own file rather than inlined:* three mounts of one label-and-toggle is the
duplication the tag side already extracted into `TagVocabularyMenuItems`.

### D7. The chip: the `pinnedChip` snippet gains a `mark` and a text class

`pinnedChip` is generalised rather than copied: it takes `{ key, label, state, onactivate,
kind: 'tag' | 'collection', menu: Snippet }` (or an equivalent parameter list — the snippet's
body stays one). For a tag the text class is `CATEGORY_TEXT_CLASS[vocabulary.categoryOf(tag)]`
and the glyph is `PinIcon`; for a collection the text class is `text-foreground` and the glyph
is `BookmarkIcon` — the icon `ImageCard` already uses as its "in a collection" mark, so the chip
reads as the tile's mark made clickable. Fill states are shared unchanged: the glyph solid for
`all`, outlined for `none`, half-solid for `some`, with the same ring / dashed outline, and
`aria-pressed` carrying the tri-state.

*Why neutral text and not a sixth hue:* all five category hues are taken, `general`'s blue
included (`tag-panel-polish` D2), so any colour would read as "a tag of some category" at a
glance. Plain foreground text plus a different glyph is the one combination no tag chip ever
draws. *Why the bookmark and not a folder or a pin:* the pin says "this is pinned", which both
kinds are; what has to differ is "tag or collection", and the bookmark already means
"collection" on every tile.

**Order:** every pinned tag in the sidebar's order — `CATEGORY_ORDER` first, alphabetical
within a category (owner, 2026-09-24: the strip was alphabetical alone, and reads better the
way the left panel does) — then every pinned collection by name, in one wrapping `<ul>`. The
tag order is made once, in the vocabulary store's `pinned` derivation (`groupByCategory` over
the pinned entries, each group through `sortTags`, flattened), because that list is the one
source both placements' chip rows read; the store already knows each entry's category, so no
reader has to look it up again. Tags first because the strip was the tags' first and the
owner's eye has learned their places; collections after, so pinning a collection never moves
a tag chip. No separator: the glyph and the text colour already split the two runs.

**Single image** (`Inspector.svelte`'s read-mode branch): the strip is drawn when
`vocabulary.pinned.length > 0 || collections.pinned.length > 0`; a collection chip's state is
`ownCollections.has(id) ? 'all' : 'none'` and its activation is `toggleCollection(
collectionTarget, id)` — the inspector's own collection door (`collection-actions.ts`), which
answers written records into `results.replaceMany`, moves no `updated_at`, and reports a refusal
in `collectionsError`. The strip stays hidden while the tag editor is open, both runs together:
`collectionTarget.onwritten` calls `onrelease`, which on the grid refocuses the grid and would
blur an open editor mid-draft.

**Selection** (the multi branch): the section heading reads **Pinned** rather than Tags, since
it can now hold both kinds; drawn under the same "anything pinned" condition. State from
`fillOf(countOf(id), selection.count)`; activation builds
`{ add: [], remove: [], addCollections, removeCollections }` from `toggledSelection(state,
collection.slug)` and calls `onedit(ids, spec, collection.name)`.

### D8. `pinned-state.ts`: the fill rule once, over a count

`fillOf(count, total): FillState` is extracted from `fillState`; `fillState(tag, counts, total)`
becomes `fillOf(counts.find(…)?.count ?? 0, total)`. The collection chip calls `fillOf` with the
count from `selectionCollectionCounts`' answer looked up by id. `toggledSelection(state, name)`
is reused as it stands for a collection's slug: it already means "add unless all, else remove"
over any name.

### D9. One fetch effect for both kinds of count

The inspector's existing selection effect is widened, not duplicated: its key becomes the
pinned tag names **and** the pinned collection ids (joined text, the same "stable key over a
fresh array identity" reasoning), and it fetches `Promise.all([names.length ?
selectionTagCounts(ids, 0, names) : [], collectionIds.length ?
selectionCollectionCounts(ids, collectionIds) : []])` over one `selection.peekIds()`. One
`pinnedCountsKnown` covers both, so a click on either kind of chip is ignored until both counts
describe the current selection; `pinnedCountsError` reports either failure. The effect bails out
while nothing of either kind is pinned. The `FIXME` on the fetch (a Rust count over the search
plus a row range, so no ids cross the wire) stays and now covers both calls.

*Why one effect:* two effects would each hold their own copy of the generation guard and the
`current` flag, and a click could land with one set of counts fresh and the other stale.

### D10. The selection write: `onedit` carries a `TagEditSpec`

`Inspector`'s `onedit` widens from `(ids, add, remove)` to `(ids, spec: TagEditSpec, label:
string)`. The tag chip builds `{ add, remove, addCollections: [], removeCollections: [] }` with
the tag as the label; the collection chip builds the collection lists with the collection's
name. `LibraryScreen.editSelectionTags` becomes `editSelection(ids, spec, label)` (the same body:
`needsConfirmation` → `pendingWrite`, else `writeEdit`), and `writeEdit` → `applyEdit` →
`afterWrite` is untouched. `apply_edit` with only collection lists already moves no
`updated_at` (`stamps` spec, "An apply is one write").

`pending-write.ts`' `confirmPrompt` gains a single-collection case beside `singleTagEdit`:
`singleCollectionEdit(spec)` matches exactly one entry in `addCollections` or
`removeCollections` and nothing else, and the prompt reads **Add N images to “\<label\>”?** /
**Remove N images from “\<label\>”?**, description "Nothing else about them changes.",
confirm **Add** / **Remove**, not destructive. The label, not the slug, is named, since the slug
is the query's spelling and the name is what the chip shows.

*Why ask at all, when the toolbar's Add selection does not:* the chip sits in a row where its
neighbours ask past one image, and a one-click target over a whole selection is exactly the
stray click `bulk-confirm` answers; the toolbar's two-step menu is its own confirmation.
*Why the stamp door and not `collection-actions` for the selection:* `apply_edit` resolves the
slug and writes in one transaction over any number of ids, and the `edit` pending write is
already the confirmation path the tag chip uses; `collection-actions` has no confirmation.

## Risks / Trade-offs

- [A selection chip's click resolves ids and the counts it drew came from `peekIds`] → the
  same gap the tag chip already has; `pinnedCountsKnown` refuses a click while counts are
  stale, and the selection's generation key refetches on every selection write.
- [The slug named in the pending write is renamed between the click and the confirmation] →
  `apply_edit` refuses "no collection named `x`" with nothing written, shown in the screen's
  action error; the user clicks again. Rare enough not to key the edit by id.
- [The heading change from Tags to Pinned in the selection panel] → cosmetic, and the single
  image branch has no heading of its own for the strip; the change is named here so a review
  does not read it as drift.
- [A library with many pinned collections pushes the tag list down] → the same trade the tag
  pins already accept; pinning is the user's own act, and Unpin is on the chip.

## Migration Plan

The migration only adds a defaulted column; an older build opening a migrated library refuses
it by `user_version` as every schema bump does. A rebuild of a library whose `library.json`
predates this change brings every collection back unpinned (D2).
