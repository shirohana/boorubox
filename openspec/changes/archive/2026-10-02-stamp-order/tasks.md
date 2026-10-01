> Two units in sequence, Sonnet each (retried on Opus if the gate fails): unit R (Rust +
> `packages/shared` + the TS invoke wrapper), then unit U (webview) after R lands. Design D1–D6
> decide every shape; do not re-decide them. The migration's number is whatever
> `MIGRATIONS.len()` is when unit R lands (planned v17; 16 today): amend design D1's sentence
> to the real number, never pin the planned one. Gates: unit R `mise run check` (it owns
> Rust); unit U `pnpm lint && pnpm typecheck && pnpm --filter @boorubox/app test`. No unit
> commits or ticks a hand check. `stamp-bar-clear` owns `StampBar.svelte`; this change never
> edits it — the bar follows the order through the store. `pinned-group-management` unit R
> edits `db.rs`, `commands.rs`, `lib.rs`, `model.rs`, `recover.rs`, `sidecar.rs` after this
> unit R has landed, and its unit U reuses D5's `reorder.ts` and `ReorderHandle.svelte`.

## 1. Unit R — the column, the write, the wire (`packages/app/src-tauri`, `packages/shared`, `packages/app/src/lib/api`)

- [x] 1.1 `db.rs`: `SCHEMA_V17` (the real number) per D1 with a doc comment naming
      `stamp-order` design D1, appended to `MIGRATIONS`; the v8 doc comment's "creation order
      is the only order a stamp ever has" rewritten to say position is the order since v17.
      Test `a_v16_library_migrates_numbering_stamps_by_creation` in the shape of the v15→v16
      test: three stamps with two sharing a `created_at`, positions 1..3 in `created_at,
      rowid` order, `user_version == MIGRATIONS.len()`.
- [x] 1.2 `stamps.rs`: `list` orders per D1; `upsert` per D2; `reorder` per D3 with the
      refusal shape of `require_stamp`'s (`AppError::Invalid` or the module's existing
      refusal variant — say which in the handoff). Tests: `a_new_stamp_lands_last`;
      `reorder_rewrites_positions_and_answers_the_new_order`;
      `reorder_refuses_a_list_that_is_not_the_whole_set` (missing, duplicate, unknown — three
      cases, nothing written); `editing_a_stamp_keeps_its_position`;
      `reorder_rewrites_library_json`; the existing
      `stamps_are_listed_by_creation_order_not_by_name` renamed to say "by position".
- [x] 1.3 `recover.rs` `insert_stamps` writes `position = index + 1` per D4; `sidecar.rs`
      `LibraryFile.stamps` doc per D4. Tests: `a_rebuild_restores_the_stamp_order` (file order
      Bird, Cat, Dog, created in another order); the existing stamps-rebuild test green.
- [x] 1.4 `commands.rs` + `lib.rs`: `stamps_reorder(ids: Vec<String>) -> Vec<Stamp>`,
      registered; test in the shape of the existing stamps command test.
      `packages/app/src/lib/api/commands.ts`: `stampsReorder(ids: string[]): Promise<Stamp[]>`;
      `commands.test.ts`: its invoke test. `packages/shared` `Stamp` is unchanged (D2).
- [x] 1.5 Gate `mise run check` green. Handoff: the real migration number, the refusal
      wording, and any deviation.

## 2. Unit U — the primitive and the table (`packages/app/src/lib/components`)

- [x] 2.1 `common/reorder.ts`: `moveItem` per D5 with its doc comment; `reorder.test.ts`:
      forward, backward, same index, out of range, identity rules.
- [x] 2.2 `common/reorder.ts` `reorderable` action and `common/ReorderHandle.svelte` per D5,
      header comments saying why native drag and what the row contract is
      (`data-reorder-index`). `reorder.svelte.test.ts` (jsdom): a synthetic `dragstart` on a
      handle then `drop` on another row calls `onmove(from, to)` with the indexes.
- [x] 2.3 `stamps/StampsTable.svelte`: the handle column, `reorderable` on the body, the
      row's context menu with Edit…, Delete… and "Move up" / "Move down" per D5–D6 (the
      pencil and trash buttons stay as they are), the write through `stampsReorder` then
      `onchanged` per D6; the header comment's "reordering is a non-goal" replaced by D6's
      rule. `stamps.svelte.ts`'s doc line "by creation order" becomes "in their order".
- [x] 2.4 Gate green. Handoff: anything the design did not foresee, and the exact row
      contract `pinned-group-management` unit U must follow to reuse the primitive.
- [ ] 2.5 Hand check (owner): on Settings → Stamps drag a stamp by its handle to the top;
      the table and, in edit mode, the bar both show the new order; "Move down" on the row
      menu moves it one down; a newly saved stamp is last; Rebuild library index keeps the
      order. On Windows, the drag shows a ghost and lands.
      Seen (lead's smoke 2026-10-02): the Stamps table shows a handle column; the row menu offers Edit…, Delete…, Move up, Move down (boorubox-vault/smoke-2026-10-01/17-stamp-menu.png); Move up on Stamp1 put it second in the table and the database, and the edit-mode bar listed the chips in the new order (boorubox-vault/smoke-2026-10-01/18-stamp-moved.png, boorubox-vault/smoke-2026-10-01/20-edit-mode.png). Dragging and a rebuild were not driven (no drag helper on this machine); Windows is the owner's.

## Handoff

- Unit R: migration is v17 (`SCHEMA_V17`, `MIGRATIONS.len()` = 17); design D1/Context amended to the number. Refusal: `AppError::BadRequest("the new order must list every stamp exactly once")` for a missing, duplicate or unknown id (nothing written). `stamps_reorder(ids)` registered; `stampsReorder` in `api/commands.ts`. `packages/shared` untouched.
- Unit U: `common/reorder.ts` (`moveItem`, `dropIndex`, `reorderable`), `ReorderHandle.svelte`, `StampsTable` handle column + row context menu (Edit, Delete, Move up/down, disabled at the ends). `reorderable` sits on the table's wrapper div, not the tbody (any ancestor of the rows works).
- Reorder contract for reuse: put `use:reorderable={{ onmove }}` on an element containing the rows; each row has `data-reorder-index={index}` and a `<ReorderHandle />` inside (only the handle starts a drag). `onmove(from, to)`: `to` is the final index after the move, i.e. pass to `moveItem(list, from, to)`; skip the write when `moveItem` returns the same list. Drop feedback: the hovered row gets `data-reorder-drop="before"|"after"` (style it, as StampsTable does with `data-[reorder-drop=...]:*:shadow-[...]`).
- Gates: `mise run check` green after unit R (Rust 903 passed, app 75 files); unit U `pnpm lint` (0 errors, 2 warnings in others' files), typecheck 0 errors, app tests 917 passed (77 files).
- Hand check 2.5 left open for the owner.
- Review fixes applied: 1–8.
