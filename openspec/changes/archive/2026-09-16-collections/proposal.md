## Why

The owner's Windows pass (2026-09-15): "I want to add a folder feature … an image can be put in
more than one folder. Maybe call it *collection* is better. Why *collection* but not just
tagging it? When I tag an image, the tags will be used to post to booru sites. If I tag it like
`collection:my_favorite`, it'll be treated as a regular tag, that is unexpected." A collection
is a grouping the user owns and never uploads; a tag is a fact about the picture that boorus
receive (§6, booru integration; `booru-upload`). The two must not share a table. A default
"Favorites" exists from the first open, so the feature has a use before the user makes anything.

**Depends on:** `library-sidecars` (every write mirrors to the folder; `library.json` for
library-level state), `selection-and-bulk` (the toolbar), `tags-and-ratings` (the sidebar
sections), archived. Lands after `sidebar-layout` (the sidebar column) and `inspector-polish`
(`activeTerms`, `onrelease`).

## What Changes

- **Collections exist**: named, unordered sets of images a user creates, renames and deletes;
  an image can be in any number of them. Every library has one named **Favorites** from the
  moment it is opened by this version — created for a new library and for a library upgraded
  from an older one, once, and never re-created if the user removes it.
- **A collection is referenced by a stable id, never by its name**: renaming touches the
  library-level file only and no image's describing file. Membership is written into each
  image's describing file as ids; the collections themselves (id, name, times) are written into
  `library.json`. A rebuild restores both; a membership whose collection is not in the
  library-level file comes back under a placeholder named by its id, so nothing is lost.
- **`collection:<name>` filters the search**, with `-collection:<name>` excluding, alongside
  the other metatags. The name in a query is the collection's slug — its name lower-cased with
  spaces as underscores — computed once on the Rust side and handed to the webview, so a
  collection called "My favorites" is `collection:my_favorites`.
- **The sidebar gains a Collections section** between Rating and Tags: every collection with
  its count in the current result, each a click away from filtering, with rename and delete on
  its menu and a way to create one.
- **The app's first right-click menu on tiles**: add to or remove from any collection, for the
  image under the pointer or for the whole selection when the tile is part of it, and create a
  new collection from there.
- **The selection toolbar** offers the same collection actions for the selection.
- **The inspector shows the collections an image is in**, each a search term with the active
  marking, with remove on its menu, and an add action.
- **BREAKING** for the schema: one migration (v6 — the real number is `MIGRATIONS.len()` when
  applied; amend design D1 if it differs). An older build refuses the upgraded library with
  the existing "newer schema" message. The sidecar and library-file format numbers do not
  change: both additions are optional fields an older reader ignores.

## Non-goals

- Ordering within a collection (what would make it a Danbooru pool). Not now.
- Nested collections, smart collections, collection covers or a collection browsing screen:
  the grid filtered by `collection:` is the browsing screen.
- Uploading collections anywhere.
- Tag categories (planned separately; `collection:` stays clear of `artist:`, `character:`,
  `copyright:`, `meta:`, `general:`).

## Capabilities

### New Capabilities

- `collections`: what a collection is, the default one, membership, the filter, and where the
  app offers each action.

### Modified Capabilities

- `library-recovery`: the describing file carries membership; the library-level file carries
  the collections; a rebuild restores both.
- `library-browse`: the query language gains `collection:`.
- `app-frame`: "The frame has fixed regions" — the sidebar's order gains the collections
  between the rating controls and the tag list (carries `sidebar-layout`'s text; archives after
  it).

## Impact

- Rust: `db.rs` (v6), new `collections.rs`, `model.rs` (`Collection`, `CollectionCount`,
  `ImageRecord.collections`, `ParsedTagSearch` two fields, `TagCounts.collections`),
  `ingest.rs` (load membership), `sidecar.rs`, `recover.rs`, `query.rs`, `commands.rs`,
  `lib.rs`.
- Shared: the mirrors of those types.
- Webview: `tag-utils` (prefix, rewriter, `activeTerms`), a `collections` store, a sidebar
  section, a name dialog, the tile context menu (`ui/context-menu`, vendored and unused today),
  the toolbar entry, the inspector section, `LibraryScreen` wiring.
