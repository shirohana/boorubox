## Context

`stamps` (table `id, name, text, created_at, updated_at`, schema v8) are listed by `created_at,
rowid` (`stamps::list`), answered to the webview as `Stamp[]` through `stamps.svelte.ts`, drawn
by `StampsTable.svelte` (settings) and `StampBar.svelte` (edit mode). `library.json` carries
`stamps: Option<Vec<Stamp>>` in list order and `recover::insert_stamps` inserts them back in
file order. Before this change the schema was v16; the migration is v17 (`MIGRATIONS.len()` once unit R
landed). The app has no drag-and-drop code: `SectionResizer` uses pointer events
to resize, nothing reorders.

The `stamps` proposal's non-goal "Reordering stamps by drag; they are listed in creation
order" was right at four stamps: creation order was the order the owner made them in, and a
position column was machinery for a list that fit on one line. It stopped being right once
the bar held a daily working set whose most-used chips sat in the middle (owner, 2026-10-01).

## Decisions

**D1. Schema v17: `ALTER TABLE stamps ADD COLUMN
position INTEGER NOT NULL DEFAULT 0`, then `UPDATE stamps SET position = (SELECT COUNT(*)
FROM stamps AS earlier WHERE earlier.created_at < stamps.created_at OR (earlier.created_at =
stamps.created_at AND earlier.rowid < stamps.rowid)) + 1`.** Every existing stamp keeps the
order it had, numbered from 1. `list` orders by `position, created_at, rowid`: position
first, the old key as the tie-break, so a row with a stale `0` (none after the migration; the
tie-break is there so the order is total, never for data this build writes) still lists
deterministically.

**D2. A new stamp lands last: `upsert`'s insert branch writes `position = (SELECT
COALESCE(MAX(position), 0) + 1 FROM stamps)`; the update branch leaves `position` alone.**
Delete leaves a gap, and a gap is harmless: the order is by position, not by position being
dense, and compacting on every delete would be a write nobody reads. `Stamp` carries no
`position` on the wire — the list's order is the order, and a number the webview could
disagree with is one more thing to keep in step.

**D3. One write for a reorder: `stamps::reorder(library, ids: &[String])`, command
`stamps_reorder(ids)`.** `ids` is the whole list in its new order; the write sets `position =
index + 1` for each in one transaction and refuses — nothing written — when `ids` is not
exactly the set of the library's stamps (a duplicate, a missing id, an unknown id): a partial
order would interleave with the positions it did not name in a way the user did not see.
Answers the list as it now stands and rewrites `library.json`, as `upsert` and `delete` do.
Not a `move(id, to)`: the webview computes the full order from the drag or the arrow, and a
single shape serves both.

**D4. `library.json` lists stamps in position order and `recover::insert_stamps` writes
`position = index + 1` in file order.** A file written before this change is in creation
order, which is what the migration numbers too, so an old file rebuilds to the same order the
migration would have given. `sidecar.rs`'s doc line on `stamps` says "in their order" instead
of "by creation order".

**D5. The reorder primitive lives in `components/common/reorder.ts` and
`ReorderHandle.svelte`, shared with `pinned-group-management`.** `moveItem<T>(list, from, to):
T[]` is pure and tested: a new array with the item at `from` placed at `to`, unchanged
identity when `from === to` or either is out of range. `reorderable` is a Svelte action for a
list element: rows carry `data-reorder-index`; a `ReorderHandle` (a `GripVertical` icon
button, `aria-label="Drag to reorder"`, `draggable`) starts a native HTML5 drag carrying the
index; `dragover` on a row computes the target index from the pointer's half of the row;
`drop` calls the action's `onmove(from, to)`. Native drag and not a pointer-event
reimplementation: the list is short, the rows are uniform, and the webview is WebKit and
WebView2, both of which drag a `draggable` element with the OS ghost image for free.
The row's context menu — `StampsTable` gains one, the bar's chips already have one — offers
"Move up" and "Move down", disabled at the ends, so a keyboard user has the same reach
(`app-frame`'s "no control appears before it does something": disabled, not hidden, because
the item's position in the menu is what a repeat user's hand learns).

**D6. `StampsTable` owns the drag and asks the store after the write.** On `onmove` it
computes `moveItem(stamps, from, to).map((s) => s.id)`, calls `stampsReorder`, then
`onchanged` (which refreshes the store), the same `run` shape its delete uses: the list comes
back from Rust whether the write landed or not. No optimistic reorder: the store is the one
list both the table and the bar read, and a local copy would be a second order for the length
of a round trip. The table gains a leading handle column, 8px of width, nothing else moves.

## Risks / Trade-offs

- [A drag in WebView2 that shows no ghost image] → the handle still works by the arrows;
  the hand check on Windows is the owner's.
- [`position` gaps after deletes] → ordering is total by D1's key; gaps are never read.

## Migration

Schema v17 (D1), automatic. `library.json` order is unchanged in meaning; a rebuild from an
old file lands in the order the migration gives.
