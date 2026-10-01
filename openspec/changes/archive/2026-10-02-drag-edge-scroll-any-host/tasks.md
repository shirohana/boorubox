> One unit, Sonnet, retried on Opus if the gate fails; the lead drives both hosts on screen
> before anything is reported. Design D1–D2 decide every shape. Owns `common/pointer-drag.ts`
> + its test, `common/reorder.ts` + `reorder.svelte.test.ts`, `tags/PinnedTagsPanel.svelte` +
> its test. Gate: `pnpm lint && pnpm typecheck && pnpm --filter @boorubox/app test`, once at
> the end. Does not commit; does not tick the hand check. No new drag.

## 1. Unit E — one edge scroller (`packages/app/src/lib/components`)

- [x] 1.1 `common/pointer-drag.ts`: `edgeScroller()` per D1 with a header comment on how the
      box is found; `pointer-drag.test.ts`: the box is the nearest scrollable ancestor of the
      element under the pointer (a stubbed `elementFromPoint`, `scrollHeight`/`clientHeight`
      and `getComputedStyle`), falling back to `document.scrollingElement`; a pointer in the
      bottom band advances `scrollTop` on the next frame, outside it does not; `stop` ends
      the loop; the step stops when `scrollTop` no longer changes.
- [x] 1.2 `common/reorder.ts`: `at` on move, `stop` on end per D2; `reorder.svelte.test.ts`
      asserts both calls.
- [x] 1.3 `tags/PinnedTagsPanel.svelte`: its own loop deleted, the scroller used with the
      `onscroll` re-resolve per D2; its tests moved or reduced per D2.
- [x] 1.4 Gate green. Handoff: anything the design did not foresee.
- [ ] 1.5 Hand check (owner): Settings → Pinned tags in a short window — drag a tag to the
      bottom edge and hold: the page scrolls and the drop lands; Settings → Stamps the same
      with a stamp's handle; the dialog still scrolls its own box.
      Seen (lead, on screen 2026-10-02, window 600px tall, webview zoomed 150% so the pages overflow): Settings → Pinned tags — `new_added` dragged to the bottom edge and held 3 s scrolled the column to its end (boorubox-vault/smoke-2026-10-01/r5-02-before.png → boorubox-vault/smoke-2026-10-01/r5-03-after.png); released below the last group it landed nowhere, as designed; released on Games after the scroll it moved there (boorubox-vault/smoke-2026-10-01/r5-05-dropped.png, database). Settings → Stamps — Stamp1's handle to the bottom edge, held, released lower: the page scrolled and Stamp1 went from first to sixth (boorubox-vault/smoke-2026-10-01/r5-07-stamps-dropped.png, database). The dialog — `hololive` to the groups box's bottom band, held, released on Games: the box scrolled and the tag moved (boorubox-vault/smoke-2026-10-01/r5-10-dialog-dropped.png, database). The drive-app helper gained `dragvia` for the hold-then-move gesture.

## Handoff

- Landed 1.1-1.4. `edgeScroller(onscroll?: () => void)` in `common/pointer-drag.ts` returns
  `{ at(x, y), stop() }`; band 40px, step `2 + 14 * (1 - distance / 40)`. The scrolling
  element is measured against the viewport (`0..window.innerHeight`), not its own rect.
- `reorderable` calls `at` on every move, `stop` on end and destroy. The panel's `tagDrag`
  calls `at` on move, `stop` on end and destroy; `onscroll` re-resolves `over`.
- Tests: scroller cases in `pointer-drag.test.ts` (jsdom lacks `document.scrollingElement`, the
  test defines it); the panel's and reorder's tests spy on `edgeScroller` and assert `at`/`stop`.
- Deviation: none. Gate green (lint 0 errors, typecheck, 978 tests). Hand check 1.5 open.
