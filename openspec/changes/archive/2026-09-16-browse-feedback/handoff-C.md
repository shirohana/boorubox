# Handoff — agent C, task group 3 (the viewer)

## 3.1 `viewer-zoom.ts` + test — done

Rewrote `packages/app/src/lib/components/library/viewer-zoom.ts` per design D7–D8. Final
shape:

- `Size`, `Point` — unchanged.
- `ZOOM_MAX = 8` — unchanged value, `ZOOM_FACTOR` (the old fixed 1.25-per-notch multiplier)
  is gone: the wheel/pinch are continuous now, nothing multiplies by a fixed step.
- `WHEEL_SENSITIVITY = 0.002`, `PINCH_SENSITIVITY = 0.01` — new, per D8.
- `MINIMAP_FRACTION = 1 / 3` — new, per D8.
- `fitScale(natural, viewport)` — unchanged (never upscales).
- `coverScale(natural, viewport)` — new: `max(vw/nw, vh/nh)`, upscales freely.
- `clickTarget(natural, viewport)` — replaces `zoomTarget`: `cover > fit ? cover : 2 * fit`.
- `zoomBy(scale, factor, fit)` — replaces `zoomStep(scale, direction, fit)`: multiplies by a
  continuous `factor` instead of stepping by `ZOOM_FACTOR` in a `direction`, same clamp to
  `[fit, ZOOM_MAX * fit]`.
- `wheelZoomFactor(event: WheelZoomEvent)` — new, `WheelZoomEvent = { deltaY, deltaMode,
  ctrlKey }` (a minimal structural type, not `WheelEvent`, so the module stays DOM-free and
  testable without jsdom). Normalises `deltaY` to pixels (`deltaMode` 1 × 16, 2 × 800), then
  `exp(-pixels * sensitivity)`, `ctrlKey` selecting `PINCH_SENSITIVITY`.
- `panOffset(pointer, viewport, content)` — same signature and return shape; the internal
  `axisOffset` now maps the pointer's fraction across a box centred on the viewport,
  `MINIMAP_FRACTION` of its size, instead of the whole viewport.

19 tests in `viewer-zoom.test.ts`, all passing, covering every number in the task's verify
list (4000×3000 → cover 0.375 = click target; 4000×1000 → cover 0.9; 400×300 → cover 3.75;
matched-aspect image → click target 2×fit; `zoomBy` clamped at fit and ceiling; wheel factors
at 100px/5px/reverse direction/`deltaMode` 1 and 2/ctrl pinch; pan fractions at 300/450/600 of
a 900-wide viewport).

## 3.2 `Lightbox.svelte` — done

- Removed: the `header` element and its four buttons (Previous/Next/Info/Close — now
  keyboard-only, per "the view SHALL draw nothing over the image"), the `chrome` state, the
  `flushSync`-based Tab-reveals-chrome branch, `onimageclick`, `singleClickTimer`, and the
  `click-intent.ts` import. Deleted `click-intent.ts` and `click-intent.test.ts` (grepped the
  repo first — nothing else imported either file).
- `onclick` on the `<img>` is `toggleZoom` directly, which now also sets a `zooming` flag and
  arms a 300ms fallback timer; the flag adds `transition-[width,height,transform] duration-200
  ease-out` to the image's class (Tailwind interpolated the same way `ImageCard.svelte` already
  does it, `class="... {expr}"`, not an array — there's no array/object `class` helper in use
  elsewhere in this package) and is cleared on `ontransitionend` or the timeout, whichever
  first. The wheel handler now reads `zoomBy(displayScale, wheelZoomFactor(event), fit)`
  instead of the old fixed ±1 step; the `deltaY === 0` early-return and its comment are gone
  since a no-op wheel now naturally yields a ×1 factor.
- WebKit pinch: a local `interface GestureEvent extends UIEvent { scale: number; clientX:
  number; clientY: number }` plus a small `GestureEventTarget` interface typing
  `addEventListener`/`removeEventListener` for `'gesturestart' | 'gesturechange' |
  'gestureend'`. Attached/detached on the `viewport` div inside a dedicated `$effect` with a
  cleanup function. All three handlers call `event.preventDefault()`. `gesturestart` records
  `gestureStartScale = displayScale`; `gesturechange` sets `scale = zoomBy(gestureStartScale,
  event.scale, fit)`; `gestureend` only prevents default.
  **Deviation from the literal brief text:** the brief's example interface was `interface
  GestureEvent extends UIEvent { scale: number }` (no position fields). I added `clientX` /
  `clientY` to it and call `updatePointer(event)` from `gesturestart`/`gesturechange`, because
  design D8 says explicitly "Both paths anchor at the pointer through `updatePointer`, as the
  wheel does now" — the minimal interface alone can't satisfy that without the extra fields.
  Safari's real `GestureEvent` does carry `clientX`/`clientY` at runtime; this only makes the
  type reflect it.
- `trapTab`'s doc comment and the `TAB_STOPS` doc comment: rewrote the sentences that explained
  the hidden-chrome rationale (the `offsetParent !== null` filter now reads as "drops stops
  that are not currently rendered — the inspector's controls when it is not showing"); the
  Tab-stops list is now just the inspector's own controls. The `<div bind:this={surface}>`
  comment's "Shift-Tab from `Previous`" example was updated to "from the first control" since
  `Previous` no longer exists.
- Amended: the `zoomed` derived's comment ("double click" → "click"), the viewport comment
  referencing `fitScale`/`zoomTarget`/`panOffset` → `fitScale`/`clickTarget`/`panOffset`, the
  undraggable-image comment ("toggling the chrome or the zoom" → "toggling the zoom"), the
  dialog `onclick` comment's closing list ("the chrome or the inspector" → "or the inspector").
  Deleted the stage `<div>`'s comment about shifting the bar for the macOS traffic lights —
  nothing needs that clearance now that there is no bar.
- Kept as-is (not chrome-related): the tile-double-click-lands-here guard in the dialog's own
  `onclick`, `PAN_MARGIN_PX`/D4 margin comments, `move()`'s D3 comment, the Space-key comment,
  the inspector `onrelease` comment, all arrow/row-step navigation.

## Gate

- `pnpm -r typecheck` — clean, 0 errors (`packages/app typecheck: ... 0 ERRORS 0 WARNINGS`).
- `pnpm lint` — 0 errors. 9 warnings remain, all in files outside this task group's ownership
  (`ImageCard.svelte`, `CollectionsSection.svelte` — agent B's group 2); none in
  `Lightbox.svelte` or `viewer-zoom.{ts,test.ts}`.
- `pnpm --filter @boorubox/app test` — 531 passed, 1 failed
  (`ImageCard.test.ts > opens the tile context menu without the bits-ui group-heading crash`,
  `lifecycle_function_unavailable: mount(...) is not available on the server`). That test
  belongs to agent B's task 2.3 (missing `// @vitest-environment jsdom` pragma, by the look of
  the error) and is outside this group's file ownership — not touched. Running only
  `viewer-zoom.test.ts` directly: 19/19 passed.

## Could not verify without the app

Everything in the 3.2 hand check is unverified by hand, as instructed — no dev server, no app
run:
- The WebKit `gesturestart`/`gesturechange`/`gestureend` path (macOS trackpad pinch): typed
  and wired, but only a real WKWebView dispatches these events, so the ×`event.scale` anchoring
  and the `preventDefault` (stopping the page from zooming) are unverified.
- The Chromium ctrl-wheel pinch path on WebView2 (Windows): `wheelZoomFactor`'s
  `PINCH_SENSITIVITY` branch is unit-tested, but real WebView2 pinch deltas were not measured
  against it.
- The click-to-zoom animation smoothness (`transition-[width,height,transform] duration-200
  ease-out`) and whether `transitionend` fires reliably vs. the 300ms fallback catching it
  every time.
- Tab-cycling behaviour in gallery mode (zero controls now that the header is gone): I left
  `trapTab` as the design specifies, with no added guard for the zero-stops case (previously
  masked by the header always having at least one enabled button). This wasn't called out in
  design D7/D8 as something to add; flagging it here in case the owner's hand check surfaces a
  WebKit focus-escape in gallery mode with nothing to Tab to.

Hand check: (left to the owner, per task 3.2 — macOS trackpad then Windows mouse, the full
sequence written in `tasks.md` 3.2's hand-check line.)
