> Two units in sequence, Sonnet each (retried on Opus if the gate fails): unit R (Rust +
> shared: migration, model, sidecar round-trip, commands), then unit W (webview: store, menus,
> chips, the widened `onedit`), after R lands. Design D1–D10 decide every shape; do not
> re-decide them. Gate for every unit: `mise run check` green. No unit ticks a hand check: an
> agent that cannot run the app writes what it saw under the `Hand check:` line and leaves the
> box open. The migration's number is whatever `MIGRATIONS.len()` is when unit R lands (planned
> v10 by queue position; 9 today): amend design D1's sentence to the real number, never pin the
> planned one. Sibling changes in flight (`category-count-search`, `auto-artist-tag`,
> `tag-category-visibility`) do not touch these files except `commands.rs` / `lib.rs` /
> `model.rs` / `packages/shared/src/index.ts`, where each adds its own entries; rebase, do not
> merge by hand.

## 1. Unit R — the column, the pin and the counts (`packages/app/src-tauri`, `packages/shared`)

- [x] 1.1 `db.rs`: `SCHEMA_V10` (the real number) per D1 with its doc comment, appended to
      `MIGRATIONS`; `model.rs` `Collection.pinned` with `#[serde(default)]` (D2);
      `packages/shared` `Collection.pinned: boolean`; `collections.rs` `COLLECTION_COLUMNS` /
      `row_to_collection` read it; `recover::insert_collections` writes it,
      `insert_placeholder_collections` leaves the default. Tests:
      `a_v9_library_migrates_reading_every_collection_unpinned` (in `db.rs`, the shape of
      `a_v6_library_migrates_to_v7_reading_every_existing_tag_general_and_unpinned`);
      `model.rs` `a_collection_crosses_the_wire_in_camel_case_and_defaults_to_unpinned`
      (new: no `Collection` wire test exists yet);
      `sidecar.rs` `a_library_file_without_collection_pins_reads_every_collection_unpinned`
      (an old `library.json` whose collection entries lack `pinned` parses); `recover.rs`
      `a_rebuild_restores_pinned_collections` ("Pinned collections come back": `Cute` pinned,
      `Queue` not) and `a_placeholder_collection_comes_back_unpinned`. Verify:
      `cargo test db:: sidecar:: recover:: model::` pass.
- [x] 1.2 `collections.rs`: `set_pinned(library, id, pinned) -> Result<Vec<Collection>>` per D3;
      `selection_counts(conn, ids, collection_ids) -> Result<Vec<CollectionCount>>` per D4 on
      `query::ID_CHUNK` / `query::placeholders` / `query::text_values` (made `pub(crate)`).
      Tests: `set_pinned_toggles_and_answers_the_list`,
      `set_pinned_for_an_unknown_collection_is_refused` (`NotFound`),
      `set_pinned_rewrites_library_json_and_no_sidecar` ("Pin touches one file": the sidecar's
      mtime/bytes unchanged, `library.json` lists `"pinned": true`),
      `set_pinned_leaves_updated_at_alone`,
      `selection_counts_counts_only_the_named_collections_over_the_ids`,
      `selection_counts_with_no_ids_or_no_collections_is_empty`,
      `selection_counts_reaches_every_id_past_the_sqlite_variable_chunk_size` (the shape of
      `apply_edit_reaches_every_id_past_the_sqlite_variable_chunk_size`). Verify:
      `cargo test collections::` pass, `mise run clippy` clean.
- [x] 1.3 `commands.rs` + `lib.rs`: `set_collection_pinned(id, pinned)` and
      `selection_collection_counts(ids, collection_ids)`, registered beside the collection
      commands. Tests: `set_collection_pinned_reaches_the_open_library_through_the_command`,
      `selection_collection_counts_reaches_the_open_library_through_the_command`, and both
      added to `the_collections_commands_need_a_library_before_they_answer`. Verify:
      `mise run check` green.

## 2. Unit W — store, menus, chips (`packages/app`), after unit R

- [x] 2.0 `api/vocabulary.svelte.ts`: `pinned` derived in category order then alphabetical
      per D7 (`groupByCategory(entries.filter(pinned), e => e.name, e => e.category)`, each
      group's names through `sortTags`, flattened); doc comment says it is the sidebar's order
      and why it lives here. Test in `vocabulary.svelte.test.ts`: `pinned lists category order
      first, then alphabetical` (the spec's four tags → `kantoku, azur_lane, 1girl, tagme`).
      Verify: `pnpm --filter @boorubox/app test vocabulary` passes.
- [x] 2.1 `api/commands.ts`: `setCollectionPinned(id, pinned)` and
      `selectionCollectionCounts(ids, collectionIds)` with invoke-shape tests in
      `commands.test.ts`; `api/collections.svelte.ts`: `pinned` (`$derived`) and `setPinned`
      per D5, tests in `collections.svelte.test.ts` (`pinned follows the list in name order`,
      `setPinned replaces the list with the answer`, `a refused setPinned sets error and keeps
      the list`, `a deleted collection leaves pinned on the next refresh`). Verify:
      `pnpm --filter @boorubox/app test collections` passes.
- [x] 2.2 `components/library/pinned-state.ts`: `fillOf(count, total)` extracted per D8,
      `fillState` rewritten on it; `pinned-state.test.ts` gains `fillOf` cases (0 of 12 →
      none, 4 of 12 → some, 12 of 12 → all, 0 of 0 → none) and the existing `fillState` cases
      still pass. `components/library/pending-write.ts`: `singleCollectionEdit` and its prompt
      per D10; `pending-write.test.ts` gains `a single collection add names the collection and
      the count`, `a single collection remove reads Remove … from`, and `an edit with a
      collection and a tag falls back to the Apply prompt`. Verify:
      `pnpm --filter @boorubox/app test pinned-state pending-write` passes.
- [x] 2.3 `components/tags/CollectionPinMenuItem.svelte` per D6, mounted in
      `CollectionsSection.svelte`'s row menu (first, then a separator) and the inspector's
      collection badge menu (after "Remove from this collection" and a separator). Verify:
      `mise run check` green.
      Hand check: right-click `Cute` in the Collections section — Pin is first; choose it,
      right-click again — it reads Unpin; `library.json` shows `"pinned": true` on `Cute` and
      no image's `.json` changed; right-click `Cute` among an image's collections in the
      inspector — the same item, reading Unpin.
- [x] 2.4 `Inspector.svelte`: `pinnedChip` generalised per D7 (tag: `PinIcon` + category
      colour; collection: `BookmarkIcon` + `text-foreground`; the chip's menu a snippet —
      `TagVocabularyMenuItems` or `CollectionPinMenuItem`); the single-image strip draws tag
      chips then collection chips under the "anything pinned" condition, a collection chip
      toggling through `toggleCollection(collectionTarget, id)`; the selection branch's
      heading reads Pinned, its effect widened per D9 (one key, one `Promise.all`, one
      `pinnedCountsKnown`), collection chips filled by `fillOf` and activated through the
      widened `onedit(ids, spec, label)` per D10; `LibraryScreen.svelte`
      `editSelectionTags` → `editSelection(ids, spec, label)`, its doc comment rewritten to the
      new shape. Verify: `mise run check` green.
      Hand check: pin `Cute` with no tag pinned — the strip appears with one bookmark chip in
      plain text; pin `tagme` too — `tagme` comes first, `Cute` after; on an image not in
      `Cute`, click the chip — the badge `Cute` appears below, the tile gains its collection
      mark, the chip fills, "Changed last" order does not move; click again — all three undo.
      Select twelve with four in `Cute` — the chip is half-filled; click — the dialog reads
      "Add 12 images to “Cute”?"; confirm — the chip fills; click and confirm — none of the
      twelve is in `Cute`. Open the tag editor — the whole strip hides. Rename `Cute` to
      `Kawaii` — the chip follows; delete it — the chip goes, and with nothing else pinned the
      strip and the selection panel's Pinned heading go too. Open the viewer's inspector — the
      chip is there and toggles the image on screen.
      Hand check: in dark mode, the collection chip is readable in all three fill states and
      is not mistaken for a general (blue) tag chip.
