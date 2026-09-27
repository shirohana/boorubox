> One unit, Opus (judgment: store logic and a re-read ordering with no gate that sees the race).
> Design D1–D9 decide every shape; do not re-decide them. Gate: `pnpm lint && pnpm typecheck &&
> pnpm test` from the repo root. The store keeps its index contract: `focus`, `anchor` and
> `has(index, id)` stay as they are, and `LibraryGrid.svelte` is not edited. Vitest compiles
> runes for the server (`ids()`'s comment): never compare a `$state` field against an object
> literal in a test — assert through `count`, `has`, `focus`, `anchor` and `previewIds`. No unit
> commits or ticks a hand check.

## 1. Unit S — the selection pins by id (`packages/app`)

- [x] 1.1 `api/selection.svelte.ts`: `LoadedRows`, `PositionResolver`, `Pinned`,
      `SelectionPin`; the constructor's third argument; exported pure `shiftPast(index,
      positions)`, `rowOf(rows, id)` and `relocate(pinned, rows, position, shiftBy)` per D1, D3,
      D4. Tests in `selection.svelte.test.ts` (fixture `selection()` gains a `position` spy;
      a `rows(ids)` helper builds a `LoadedRows` with `undefined` for unloaded rows):
      `shiftPast moves an index past every inserted row at or before it` (two positions, one
      above and one below), `relocate finds a loaded row without asking for its position`,
      `relocate asks for the position of a row that is not loaded`, `relocate keeps the index of
      an image that left the result`, `relocate shifts a pinned index that had no id`.
- [x] 1.2 `Selection.pin` and `Selection.repin` per D1, D2, D4. Tests: `pin turns a loaded
      range into its ids without a round trip` (resolve not called), `pin leaves a range that
      reaches past the loaded rows a range`, `the owner's repro: a capture above a loaded range
      keeps the same three images` (rows `i0..i6`, range `[0, 3)`, capture `new` re-read at row 0:
      `has(1, 'i0')`, `has(3, 'i2')`, not `has(0, 'new')`, count 3), `repin puts the focus and
      the anchor back on their images` (loaded after the re-read; position not called),
      `repin asks for the position of a focus off the loaded rows`, `a gesture during the
      re-read keeps its focus` (`focusAt` between `pin` and `repin`), `select-all stays a
      range across a capture above it` (count unchanged, resolve not called, `has(0, 'new')`
      false), `a capture below a range leaves it alone`, `a capture inside a range is resolved
      out of it` (resolve called with the widened window, the capture's id not selected, count
      unchanged), `an unloaded focus follows the capture's row`, `the strip shows the same images
      after a capture` (`previewIds` before and after, equal arrays of strings),
      `repin returns the inserted rows' positions, sorted`.
- [x] 1.3 `components/library/LibraryScreen.svelte`: the third resolver per D1;
      `rereadKeeping(inserted?)` per D6 and D7 with the serialising chain and the capture batch;
      `refreshResults` and `afterWrite` rewritten onto it (their doc comments say why the pin
      comes before the refresh — the commit-before-event argument of D3, not a design number
      alone); the import path's `FIXME` per D5 and `afterWrite`'s backstop `FIXME` per the
      design's risk; `onCaptureStored` passes the record's id. Verify: gate green, and the
      existing `searchKeeping` path untouched.
- [x] 1.4 `Lightbox.svelte` `onartistrenamed` prop passed to its `Inspector`;
      `LibraryScreen.svelte` passes `afterWrite`; `Inspector.svelte`'s fallback branch removed,
      `onartistrenamed` required, `rowWithId` replaced by `rowOf(results, id)` per D3 and D7.
      Verify: typecheck catches any mount missing the prop; gate green.
- [x] 1.5 `openspec/changes/archive/2026-09-10-selection-and-bulk/design.md` D4: append a dated
      paragraph (2026-09-28) per D9. `selection.svelte.ts`'s header comment and `SelectionState`'s
      doc say a range is re-pinned across a re-read, naming `selection-by-id`.
- [x] 1.6 Gate green (`pnpm lint && pnpm typecheck && pnpm test`). Handoff: the final
      signatures, where the serialising chain lives, and anything the hand check should watch.
- [ ] 1.7 Hand check (owner): the repro on the real vault with a live capture, and the viewer
      across a capture.
      Hand check: open the real vault in capture order, click the first tile, shift-click the
      third (tiles 1–3 marked), then capture an image from the browser extension; confirm the new
      tile appears first and unmarked, the same three images are still marked at tiles 2–4, the
      count reads 3, the inspector's strip shows the same three thumbnails, and the arrow keys
      move from the image that was focused. Then open the viewer on tile 2, capture again, and
      confirm the viewer still shows the same image, left arrow goes to the image that was tile 1,
      and closing the viewer lands the grid focus on that same image's tile. Optional: select
      all, capture, and confirm the count rose by nothing and the new tile is unmarked.
      Not seen (lead's smoke 2026-09-28): a capture cannot be sent without the extension, and no other re-read inserts a row above a live selection (a Trash restore leaves the library page, and returning to it clears the selection), so the shift was not approximated.

## Handoff

**Signatures** (`packages/app/src/lib/api/selection.svelte.ts`):
`new Selection(resolve, matches, position: PositionResolver)`;
`pin(rows: LoadedRows): SelectionPin` (sync, promotes a fully loaded range through `#promoted`);
`repin(pin, rows, inserted?: string[]): Promise<number[]>` (sorted positions of `inserted`,
`[]` without it). Exported pure helpers: `shiftPast(index, positions)`, `rowOf(rows, id)`,
`relocate(pinned, rows, position, shiftBy = [])`. `rowOf` replaced `Inspector.svelte`'s
`rowWithId` (both its callers). The range shift lives in the private `#shiftRange`, called only
when `inserted` was passed and `#state` is still `pin.state`.

**Serialising chain** (`LibraryScreen.svelte`): `rereadKeeping(inserted?)` appends to the
script-level `rereads` promise; capture ids arriving while a capture re-read is queued go into
`batchedCaptures` and are taken by that queued run. The body is `rereadNow`, which catches into
`actionError` so a failed re-read never breaks the chain; the viewer is moved by `keepViewer`.
`positionOf` is the one `searchPosition` closure, shared by the store and `keepViewer`.
`refreshResults(inserted?)` (capture: `[image.id]`; import: none) and `afterWrite` both go
through it; `searchKeeping` is untouched. FIXMEs: the import call site (design D5) and the
`selection.ids()` backstop inside `rereadNow` (design Risks).

**Outside the owned list:** `grid-focus.test.ts` gained the third constructor argument and
`Inspector.svelte.test.ts` the now-required `onartistrenamed` prop; nothing else. No design
decision was changed.

**For the hand check:** `rereadNow` also calls `results.ensureRange` on the relocated focus (the
design's Risks line), which loads a page but must not scroll. `afterWrite` still ends with
`grid.focusCard`, so a rename from the viewer scrolls the grid behind the modal (design D7
accepts this). Watch the new tile's flash: `SearchResults.refresh` still empties rows before
page 0 answers (proposal Non-goals).
