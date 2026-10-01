## Context

`common/pointer-drag.ts` is the primitive (`pointerDrag(root, handlers, options)`);
`common/reorder.ts`'s `reorderable` builds the Stamps row drag on it; `PinnedTagsPanel.svelte`
builds the tag drag on it and carries its own edge scroll: `EDGE_BAND`, `edgeSpeed`,
`edgeStep(rect, y)`, a `requestAnimationFrame` loop over `[data-groups]`'s `scrollTop`. On
the Settings page the panel's region has no bound of its own and the layout's content column
(`routes/settings/+layout.svelte`, `overflow-y-auto`) is what scrolls; the Stamps table is
inside the same column.

## Decisions

**D1. The edge scroll moves into the primitive: `edgeScroller()` in `pointer-drag.ts`.**
`const scroller = edgeScroller(); scroller.at(x, y); scroller.stop()`. `at` resolves the
scrollable box for the pointer — from `document.elementFromPoint(x, y)` up through ancestors
to the first with `scrollHeight > clientHeight` and a computed `overflow-y` of `auto` or
`scroll`, else `document.scrollingElement` — and, when the pointer is inside that box's 40px
top or bottom band, arms one frame loop that steps its `scrollTop` by `2 + 14 * (1 -
distance / 40)` per frame, re-resolving the box each frame and stopping when the step moves
nothing or the pointer left the band. `stop` ends the loop. The box is resolved from the
pointer, not from a configured region, so the same code serves the dialog's groups box, the
Settings column and any future host without a prop.

**D2. Both consumers use it and own nothing of their own.** `reorderable` calls `at` on every
move and `stop` on end, so the Stamps rows scroll the Settings column; the panel deletes its
`EDGE_BAND`/`edgeSpeed`/`edgeStep`/frame loop and calls the same two methods, re-resolving the
section under the pointer after each frame as before (the scroller takes an `onscroll`
callback for that). The jsdom tests for the loop move to `pointer-drag.test.ts`; the panel's
and the reorder tests assert only that `at`/`stop` are called with the pointer.

## Risks / Trade-offs

- [`elementFromPoint` returns the dragged ghost or the handle] → the dimmed row is still an
  ancestor of the scroll box's descendant; the walk reaches the box.
- [Nested scroll boxes] → the innermost one under the pointer wins, which is the one the
  user is looking at.
