## Why

The owner browses the library at random and drops images into a topic collection as they
go: one click, no menu to wait for (2026-09-23). Today that is right-click → Add to… →
the collection, three moves per image, or a stamp, which is the bulk/edit-mode door and asks
for the mode first. Pinned tags already give a tag exactly this one-click toggle in the
inspector; a collection has no such chip. Requirements §6 (Danbooru-style tagging, bulk ops,
sidebars) and §7 (what `library.json` carries for a rebuild).

## What Changes

- **A collection can be pinned.** Pin / Unpin on the context menu of a collection's row in the
  sidebar's Collections section and of a collection badge in the inspector, the same item the
  tag menus carry. The pin is a property of the collection, stored on its row and written to
  `library.json` with it, so a rebuild brings it back.
- **A pinned collection is a chip in the inspector's pinned strip**, after the pinned tags,
  drawn as a collection (bookmark mark, neutral text) rather than in any of the five category
  colours. For one image the chip shows membership and one click adds the image to the
  collection or takes it out. Over a selection it shows all / some / none of the selected
  images in it; a click adds every selected image unless all are in, in which case it takes
  every one out, asking first past one image — the pinned tag chip's rule, word for word.
- **Deleting a pinned collection takes its chip with it.** Renaming it renames the chip.
- No pseudo tag: a pinned collection does not appear in the Tags list. `collection:cute`
  already searches it, and the Collections section already lists it with `+ − name count`
  (owner: "a bad idea").

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `collections`: a collection can be pinned and unpinned from the sidebar row and the
  inspector badge; a pinned collection is an inspector chip that toggles membership for one
  image and tri-states over a selection (new requirement); the library-level file carries the
  pin, so a rebuild restores it ("A collection survives a rebuild by its id").
- `tag-vocabulary`: "A tag can be pinned for one-click editing" — the pinned strip is shared
  with pinned collections: tag chips first, and the strip is absent only while neither a tag
  nor a collection is pinned.
- `library-recovery`: the library's own file lists each collection's pin; a rebuild restores
  it, and a file written before this change brings every collection back unpinned.

## Non-goals

- A pseudo tag, or any collection row, in the sidebar's Tags list. The owner rejected it; the
  Collections section is the collections' own list.
- A pinned-collections section anywhere but the inspector. The `tag-vocabulary` non-goal "a
  separate pinned-tags section elsewhere than the inspector" is honoured, not reversed: the
  chips join the strip that non-goal kept in the inspector.
- Reordering pinned chips, or pinning from the tile's context menu or the selection toolbar.
  The badge and the sidebar row are where a collection is already acted on by name.
- Pinned collections sorted to the top of the Collections section or of the Add to… menus. The
  section keeps name order (`collections`, "An active collection stays in place").
- Replacing stamps. A stamp stays the door for a multi-part edit in edit mode; the chip is the
  casual, one-collection door.

## Impact

- Schema: one migration (v10, `MIGRATIONS.len()` when unit R landed) adding `collections.pinned`.
- `packages/app/src-tauri`: `db.rs`, `collections.rs` (`set_pinned`, `selection_counts`),
  `model.rs` (`Collection.pinned`), `sidecar.rs` / `recover.rs` (the pin round-trips through
  `LibraryFile.collections`), `commands.rs` + `lib.rs` (`set_collection_pinned`,
  `selection_collection_counts`), `query.rs` (`text_values` made `pub(crate)` for reuse).
- `packages/shared`: `Collection.pinned`.
- `packages/app`: `api/commands.ts`, `api/collections.svelte.ts` (`pinned`, `setPinned`),
  `components/tags/CollectionPinMenuItem.svelte` (new), `components/tags/CollectionsSection.svelte`,
  `components/library/Inspector.svelte`, `components/library/pinned-state.ts`,
  `components/library/pending-write.ts`, `components/library/LibraryScreen.svelte`, and the
  tests beside them.
