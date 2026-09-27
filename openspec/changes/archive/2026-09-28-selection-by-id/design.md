## Context

- `packages/app/src/lib/api/selection.svelte.ts`: `Selection` holds `focus` and `anchor` as row
  indices (:67, :69) and `#state` as `{ kind: 'ids', ids }` or `{ kind: 'range', start, end }`
  over row indices (:19-22). `ids()` (:320-336) resolves a range through the injected
  `IdResolver` — `searchIds` over the current request — and leaves the store in id mode.
  `#editIds` guards against a gesture during its round trips by comparing `#state` before and
  after (`#promoted` marks the one state `ids()` may leave behind). The constructor takes
  `(resolve, matches)`; the tests build it in `selection()` at the top of
  `selection.svelte.test.ts`, with `idAt(n) = 'i<n>'`.
- `LibraryScreen.svelte` (`packages/app/src/lib/components/library/`): `refreshResults`
  (:261-268) runs on `capture:stored` (:586) and on `imports.onfinished` (:595); `afterWrite`
  (:373-385) runs after every write that re-reads the search (bulk edit, rating, trash, restore,
  delete, the sidebar's collection change, the grid inspector's artist rename). Both call
  `await selection.ids()` then `await results.refresh()`. `lightboxIndex` (:149) is the viewer's
  row; `focused` (:213) is `results.at(selection.focus)`. `searchKeeping` (~:283-331) already
  maps one id to its new row with `searchPosition` and guards on `results.generation`.
- `SearchResults.refresh()` (`api/search.svelte.ts`) runs `#start`, which empties every loaded
  row before page 0 answers; after it only page 0 (and whatever a consumer re-asks for) is
  loaded. `setSort`, `setGroup` and `run` are new lists: the screen resets the selection for
  those and they are not refreshes.
- `capture:stored` is emitted from `http/captures.rs` `store` after `store_now` returned
  `Ingested::Created` — after the SQLite transaction committed — carrying the new
  `ImageRecord`. By the time the webview hears it, `searchIds` and `searchPosition` answer in
  the new order. An import's `onfinished` carries nothing, and fires after its rows committed.
- Sort fields are `captured | updated | size | dimensions | trashed`: under `updated`, a write
  moves the rows it touched to the top; under `size`/`dimensions` a capture can land mid-result.
- `Inspector.svelte`: `rowWithId` (:194) walks `results.at` by id; its artist-rename handler
  (:1405-1415) calls `onartistrenamed` when given (the grid's mount passes `afterWrite`) and a
  bare `results.refresh()` otherwise — the viewer's mount (`Lightbox.svelte` :498) passes none.
- `LibraryGrid.svelte` reads `selection.focus` as an index (:154, 169, 229, 249, 266, 356);
  `focusCard(index)` is `selection.focusAt(index)` plus a scroll.

## Goals / Non-Goals

**Goals:** every re-read of the same search leaves the selection, the focus, the anchor and
the open viewer on the images they were on (specs `selection`, `library-browse`), with the
store's index contract unchanged.

**Non-Goals:** see the proposal; additionally, no change to `SearchResults` (the refresh keeps
emptying its rows) and none to Rust.

## Decisions

**D1. The store keeps indices; `pin` and `repin` carry them across a re-read by id.**
The public fields stay `focus: number`, `anchor: number`, and `has(index, id)`; every reader of
an index keeps its contract. Two new methods translate around a re-read, and the store gains a
third injected resolver:

```ts
/** What a store needs of the loaded rows — `SearchResults` fits as is. */
export interface LoadedRows {
  total: number
  at: (index: number) => { id: string } | undefined
}
/** The row `id` holds in the current request, or `null` — `searchPosition` in the app. */
export type PositionResolver = (id: string) => Promise<number | null>
/** An index and the image its row held when it was pinned, if that row was loaded. */
export interface Pinned { index: number, id: string | undefined }
export interface SelectionPin { focus: Pinned, anchor: Pinned, state: SelectionState }

constructor(resolve: IdResolver, matches: MatchResolver, position: PositionResolver)
pin(rows: LoadedRows): SelectionPin
repin(pin: SelectionPin, rows: LoadedRows, inserted?: string[]): Promise<number[]>
```

`pin` is synchronous and makes no round trip: it reads the focus's and the anchor's ids off
`rows` (`undefined` for a row not loaded or an index of `-1`), promotes the range per D2, and
returns `state` read back from `#state` after that (the proxy rule in `ids()`'s comment). `repin`
runs after the re-read: each pinned index goes through `relocate` (D3) and is written only when
the field still holds `pin`'s index — a gesture made during the re-read owns the field, the same
rule `#editIds` keeps for `#state`; the range follows D4 only when `#state` is still
`pin.state`. It returns the inserted rows' sorted positions (D4; empty without `inserted`), so
the screen relocates the viewer with the same shift and no second round of `position` calls. The app passes `(id) => searchPosition(buildSearchRequest(results.inputs,
currentView(), 0), id)` as the third argument, beside the two it passes today.

Alternative — make the store hold ids for the focus and the anchor and derive the index —
rejected: the grid, the inspector and the viewer read the focus as an index in a dozen places
and every keyboard rule is arithmetic on it; an id would need a synchronous id→row map the
webview does not have for unloaded rows. The index is right between re-reads; only the re-read
moves it.

**D2. A range whose rows are all loaded is pinned from those rows.** `pin` walks
`start..end`; when every `rows.at(i)` answers, the store moves to `{ kind: 'ids' }` with those
ids through `#setState`, and sets `#promoted` to the read-back state as `ids()` does, so an
`#editIds` in flight still accepts it. The repro's range (three tiles on screen) is this case.
A range with any unloaded row stays a range for D4 or D5 to handle.

**D3. Loaded rows are the only trustworthy source, because the event arrives after the commit.**
`capture:stored` is emitted once `store_now` has committed (Context), and an import's finish
after its rows committed, so every database answer — `searchIds`, `searchPosition` — describes
the new order; the old code's `await selection.ids()` in `refreshResults` resolved the range in
that new order, which is the bug. The rows `SearchResults` holds were read before the write and
are unchanged until `refresh()` empties them, so they are the one snapshot of the order the
indices were taken in. Hence `pin` reads rows only and runs before `results.refresh()`.
After the re-read, `relocate(pinned, rows, position, shiftBy: number[] = [])` (exported, a
function of its arguments alone) finds the image again: the loaded-row walk (`rowOf(rows, id)`, which also replaces
`Inspector.svelte`'s `rowWithId`, one walk in the codebase), then `position(id)` for a row
outside what is loaded, then — for a `Pinned` with no id — the insertion shift of D4 when a
capture supplied positions; a `null` position (the image left the result) and every other case
keep `pinned.index`, today's behaviour.

**D4. On a capture, what the rows cannot answer follows the capture's own row.** A capture only
inserts rows: old rows keep their relative order under every sort field. `repin`'s `inserted`
is the stored capture ids; the store asks `position` for each, drops `null`s (not in this
search), and sorts the rest ascending. `shiftPast(index, positions)` (exported, pure) moves an
old index past each inserted row at or before it: `for p of positions: if (p <= n) n++`. A range
still in range mode maps `start` and `end - 1` through it: when no inserted position falls in
`[start', last']` the store keeps `{ kind: 'range', start: start', end: last' + 1 }` with no
round trip — select-all over a capture stays a range and its count is unchanged — and when one
does, the store resolves `#resolve(start', last' - start' + 1)` minus the inserted ids and moves
to id mode, guarded like `ids()`. An unloaded focus or anchor (no id in its `Pinned`) moves by
the same shift.

Alternative — resolve the range with `searchIds` before the refresh, as `refreshResults` did —
rejected by D3: it runs against the committed order. Alternative — promote every range to ids
on a capture — rejected: select-all over twenty thousand images would pull twenty thousand ids
per capture, which `selection-and-bulk` D3 exists to avoid.

**D5. An import has no ids to shift by, so its unloaded range keeps today's resolve.** The
finish handler passes no `inserted`; `repin` then relocates by id and by position as D3 says,
and a range `pin` could not promote is resolved with `selection.ids()` before the refresh
exactly as today — correct for the rows an import did not move, wrong by the rows it added
above. A `FIXME` at that call names the right shape: the import run reporting its created ids
so the finish handler passes them as `inserted` like a capture; not built because `onfinished`
carries no report today and an import under a partly unloaded range is rare.

**D6. One screen function re-reads for everyone, one at a time.** `LibraryScreen.svelte` gets
`rereadKeeping(inserted?: string[])`: `pin` the selection, pin the viewer (D7), `await
selection.ids()` only when `inserted` is absent (D5, and the `afterWrite` backstop its
comment already argues), `await results.refresh()`, read `results.generation` right after
`refresh` starts (as `searchKeeping` does) and stop if a newer run replaced it, then
`await selection.repin(pin, results, inserted)` and relocate the viewer. It never calls
`grid.focusCard` or scrolls: a capture is not the user's gesture. Callers of
`results.refresh()`, and which go through it:
- `refreshResults` (capture, import finish): yes — both are the drift. Becomes
  `rereadKeeping(ids)` for captures and `rereadKeeping()` for an import.
- `afterWrite`: yes — under `updated` a write reorders, and a filter can drop rows above the
  focus. `afterWrite` becomes `rereadKeeping()`, then `keepMatching`, then its existing
  `grid.focusCard` (which re-anchors at the focus, as today).
- `Inspector.svelte`'s bare `results.refresh()` after an artist rename from the viewer: yes
  (D7) — replaced, not wrapped.
- `run`, `setSort`, `setGroup`: no — new lists; the selection resets.
Re-reads are serialised: one promise chain in the screen's script, and capture ids that
arrive while one runs are batched into the next `rereadKeeping(ids)`, so no re-read pins from
rows another has emptied and two captures in a burst are one refresh with two positions.

**D7. The viewer keeps its image; its inspector's rename goes through `afterWrite`.** With the
viewer open, `rereadKeeping` pins `{ index: lightboxIndex, id: results.at(lightboxIndex)?.id }`
and after the re-read, when the viewer is still open and `lightboxIndex` still equals the
pinned index, sets it to `relocate(...)` and calls `results.ensureRange(row, row + 1)` (the
row is usually off page 0; `searchKeeping` does the same). `Lightbox.svelte` gains an
`onartistrenamed` prop passed to its `Inspector`; `LibraryScreen` passes `afterWrite`, as it
does to the grid's inspector. `Inspector.svelte`'s fallback branch goes and its
`onartistrenamed` prop becomes required: both mounts pass it, and a rename is a write like any
other (`afterWrite`'s one-path argument). `afterWrite`'s `grid.focusCard` behind the open modal
scrolls the grid under it, which the viewer's own `onmove` already does.

**D8. The strip and the inspector need no change of their own.** `SelectionThumbs` reads
`selection.previewIds(limit, at)`: ids after D2 or D4's promotion, or the shifted range read
against the re-read rows — the right images either way. The inspector's `focused` is
`results.at(selection.focus)` and follows the relocated focus. Both are asserted through the
store's tests (`previewIds` after a capture) rather than by editing them.

**D9. `selection-and-bulk` D4 is amended, not reversed.** D4 argued that ids survive an action
because an action resolves before it writes, and rejected keeping a range and re-resolving it
against the new result. Both stand. A refresh the user did not start arrives after its write,
so it cannot resolve first; the store pins from the loaded rows instead, and a range kept across
a capture is moved by the capture's known row, not re-resolved blind. Recorded under D4 in the
archive with the date.

## Risks / Trade-offs

- [A focus or anchor whose row is not loaded at pin time, on an import or a write] → no id to
  find it by; it keeps its index (today's drift). `rereadKeeping` calls `results.ensureRange`
  for the relocated focus so the next re-read finds it loaded; the grid re-asks for its visible
  window, where the focus nearly always is.
- [A capture stored between the batch's `position` calls] → it joins the next batch; the
  positions this batch read include its row, so the shift can be one too many for rows below
  it. Rare (two captures within one round trip), and the next re-read's pins come from rows.
- [`relocate`'s row walk is O(total)] → the same walk `rowWithId` does today; ≤ 3 walks per
  re-read.
- [`afterWrite`'s `selection.ids()` backstop resolves after the write for the two writers that
  do not resolve first (the sidebar's collection change, the artist rename)] → pre-existing;
  D2 now covers any range on screen. A `FIXME` on that backstop names the right shape: those
  two writers resolve the selection before they write, as `write()` does.
- [A capture or write arriving while `searchKeeping` runs] → closed. Before this change the
  re-read's `results.refresh()` bumped the generation under the in-flight run, so
  `searchKeeping` dropped the image it was keeping and an open viewer stayed on a stale index.
  `rereadKeeping` records the generation when it is asked, and `rereadNow` skips when a run
  (not the chain's own refresh) started since — that run read after the commit. A re-read asked
  for while `searchKeeping` runs waits for it on the chain, and the open capture batch closes
  when `searchKeeping` starts.
- [A ⌘-click, checkbox or strip-remove whose resolve spans a capture's range shift] → the
  shifted range is not the user's gesture: `#editIds` re-resolves the moved range
  (`#shifted`) instead of dropping the edit.
