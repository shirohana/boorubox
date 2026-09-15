> Depends on `browse-polish`, archived; lands after `inspector-polish` (shared `Lightbox.svelte`
> and viewer spec text). One implementing agent, webview only; group 1 first, then group 2.
> Gate: `pnpm -r typecheck`, `pnpm lint`, `pnpm --filter @boorubox/app test`, then `mise run check`.

## 1. The pure parts (agent A)

- [x] 1.1 `packages/app/src/lib/components/library/click-intent.ts` + test: `DOUBLE_CLICK_MS`
      and `clickIntent(detail, pending)` per design D2. Verify: tests — a first click schedules;
      a second within the window cancels and is the double; a `detail === 2` with nothing
      pending is ignored; a `detail === 1` after the window schedules again.
- [x] 1.2 `packages/app/src/lib/components/library/viewer-zoom.ts` + test: `fitScale`,
      `zoomTarget`, `zoomStep`, `panOffset`, the constants (design D3). Verify: tests — fit never
      upscales; a 4000×3000 image in 1500×900 zooms to 1; a 400×300 image zooms to 2× its fit;
      steps clamp at the fit and at the ceiling; pan at fraction 0, 0.5 and 1 shows left,
      middle and right; a content smaller than the viewport centres; pointer outside the
      viewport clamps.

## 2. The component (agent A)

- [x] 2.1 `Lightbox.svelte`: the chrome inside the stage, `hidden` by default, Tab reveals
      (design D1, D6); `trapTab` filters hidden stops. Verify: typecheck, lint pass.
- [ ] 2.2 `Lightbox.svelte`: the viewport box with the margin, the image drawn at `natural ×
      scale` with the pan transform, click intent wired (single → chrome, double → zoom), wheel,
      pointer pan, reset on `move` (design D3–D5); the close rule extended to the viewport box
      (D4). Verify: typecheck, lint and tests pass.
      Hand check (Windows and macOS): open a tall image — it fills the view's height, no bar;
      click once — the bar appears top-left after a beat, click again — gone; double-click with
      the bar hidden — it zooms, the bar does not flash; at zoom, move the pointer to each edge —
      the far edges of the image come into view, and a dark margin stays around it; click that
      margin — the viewer closes; reopen, zoom, press Escape — closes; zoom, press → — next image
      at the fit; wheel up and down — grows and shrinks, never below the fit; Tab with the bar
      hidden — it appears and Previous has the focus; on macOS the bar clears the traffic lights;
      in inspect mode the bar and zoom behave the same beside the panel; the tile's double click
      still opens the viewer without zooming it.

## 3. Change-level verification (owner)

- [ ] 3.1 `mise run check` green; the hand check passes on Windows.

## Handoff

Groups 1 and 2 are implemented and reviewed (2.2's code is done; its box stays unticked for
the Hand check). Gate after the review pass, all green in the working tree: `pnpm lint`,
`pnpm --filter @boorubox/app test` (511 tests) and `pnpm -r typecheck` (0 errors, 0 warnings —
the `collections` errors the implementer hit were that change's shared-type fields, landed
while this review ran).

Fixed in review, in `Lightbox.svelte` unless said otherwise: a double click after a wheel step
down to the fit did nothing, because the zoom read `scale !== null` and the wheel clamps to the
fit as a number — the predicate is now `displayScale > fit` (`zoomed`); `pointermove` no longer
measures and re-styles while there is no overflow to pan; a second `detail === 1` inside the
double-click window left the first timer unreachable and toggled the bar twice, so scheduling
now clears a pending timer; a trackpad's horizontal swipe (`deltaY === 0`) no longer reads as a
zoom-out notch; a bound-but-unread `imgEl` is gone.

- **Fitted size and pan, from the `<img>`'s natural size.** `Lightbox.svelte` tracks
  `naturalSize` (set from the `<img>`'s `load` event: `naturalWidth`/`naturalHeight`) and
  `viewportSize` (`bind:clientWidth`/`clientHeight` on the inset viewport box, which excludes
  the `PAN_MARGIN_PX` margin by construction — no separate constant needed in TS, the CSS
  `inset-6` class is the one source of that 24px). `fit = fitScale(naturalSize, viewportSize)`;
  `scale` is `number | null`, where `null` reads as "at the fit" so resetting on `move()` never
  has to know the next image's fit in advance. `content = natural × displayScale` (where
  `displayScale = scale ?? fit`) is the `<img>`'s explicit pixel `width`/`height`; `panOffset`
  is applied as `transform: translate(...)` on top of the flex-centred layout the viewport box
  gives it for free.
- **`panOffset`'s baseline is the centred position, not top-left.** `axisOffset = (0.5 -
  fraction) * overflow`, against a viewport that is `flex items-center justify-center` at every
  zoom: 0 → `+overflow/2`, 0.5 → `0`, 1 → `-overflow/2`, so the near and far edges of the image
  meet the viewport's exactly and a content smaller than the viewport centres with no branch.
  The review checked the arithmetic against that layout and amended design D3's formula
  sentence to this one, carrying why the top-left reading was written and why it does not hold
  here; `viewer-zoom.test.ts` asserts the mapping.
- **Before the image has loaded.** `naturalSize` is `null` until the `<img>`'s `load` fires
  (its `src` is set immediately once `libraryPath`/`image` resolve, well before decode
  finishes). While `naturalSize` or `viewportSize` is unknown (`measured` is false), the `<img>`
  falls back to the original `max-h-full max-w-full object-contain` sizing with no inline
  transform — the pre-existing behaviour — so the picture still renders progressively; zoom and
  pan only take effect once both are known. `move()` resets `naturalSize` to `null` too, so the
  fallback rendering reappears for a beat on every navigation until the new image's `load`
  fires.
- **`-0` in `panOffset`.** `(0.5 − fraction) * overflow` produces `-0` when `overflow` is 0 and
  `fraction > 0.5`; normalized to `0` so a centred content never reports a distinct signed
  offset. Written as `shift === 0 ? 0 : shift` rather than `|| 0`, which would have turned a
  `NaN` — a real defect, if one ever reached here — into a plausible-looking zero.
- **a11y warnings suppressed, not fixed.** The viewport's `wheel`/`pointermove` and the image's
  `click` are mouse-only by design (Non-Goals: no touch, no keyboard zoom; Tab already reaches
  the chrome). Suppressed with `svelte-ignore` comments (`a11y_no_static_element_interactions`,
  `a11y_click_events_have_key_events`, `a11y_no_noninteractive_element_interactions`), matching
  the one existing precedent in `LibraryGrid.svelte`. The review removed all three and re-ran
  `svelte-check`: each is a warning it really raises, none is a dead suppression.
- **`trapTab` change.** Added `stop.offsetParent !== null` to its stop filter (design D1), so a
  hidden chrome's buttons leave the Tab cycle the same keystroke that reveals it — untested by
  a unit test since it needs layout (`offsetParent`); covered by the 2.2 hand check instead.

Hand check: item 2.2's Hand check is unverified — I do not run the app. Everything above (tall
image fills the stage, click toggles the bar after the delay, double click zooms without a
flash, pan at the edges, margin click closes, Escape closes while zoomed, → resets to fit,
wheel clamps at the fit, Tab reveals the bar onto Previous, macOS traffic-light clearance,
inspect-mode behaviour, the tile's own double click still opening without zooming) needs eyes
on the running app, on both platforms per the task.
