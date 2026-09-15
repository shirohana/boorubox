## Context

See proposal.md — Why. `Lightbox.svelte` is a native `<dialog>` at 96vw × 96vh with a
`flex-col`: a `<header>` (title, counter, four ghost buttons; `ps-16` on macOS for the traffic
lights) and then the `stage`, a `flex-1` box the `<img>` is fitted into with `max-h-full
max-w-full object-contain`. The header is a flex child, so the stage is what is left. Clicks:
the dialog's `onclick` closes when the target is the dialog (the backdrop) or the stage (the
empty space) and ignores `event.detail > 1` because the second click of the tile's double
click lands here. Keys: `trapTab` cycles the dialog's tabbable elements; Space closes when the
target is the surface or the dialog. Moving (`move`) sets `index`. The tile's click rules live
in `tile-click.ts`, a pure module with tests, because "a decision about a press … could only be
found by hand" inside the component. `zoom.ts` is whole-app zoom and is not involved.

## Goals / Non-Goals

**Goals:** the pure parts (click intent, fit, zoom step, pan mapping) testable without a DOM;
the component only wires events to them; every existing close and focus rule holds.

**Non-Goals:** animating the zoom; touch gestures; keeping any of it across images.

## Decisions

### D1. The chrome is an absolutely positioned bar inside the stage, `hidden` by default

The `<header>` moves inside the stage box (`relative`) as `absolute top-2 left-2` (plus the
macOS inset, which its `max-width` is reduced by in the same breath — a long title otherwise
pushes the bar off the stage's right edge by exactly the inset), a rounded translucent bar with the title, the counter and the four buttons at
`xs` size. Hidden with the `hidden` attribute, not opacity: a hidden control must not be a Tab
stop, and `trapTab` filters stops by `offsetParent !== null`, which `display: none` satisfies
and opacity does not. `chrome` is component state, reset to hidden on open (session state
would be a claim the owner has not made).

### D2. A click on the image goes through `click-intent.ts`

Pure: `clickIntent(detail, pending)` — a click with `detail === 1` schedules the single-click
action after `DOUBLE_CLICK_MS` (250, the platforms' default); a click with `detail === 2` while
one is pending cancels it and is the double click; a `detail === 2` with nothing pending is
ignored, because the first click was somewhere else — the tile that opened this dialog. The
timer is the component's (a `setTimeout` it clears on destroy, and before scheduling another —
a click far enough from the last one restarts the engine's `detail` count, so a second
`detail === 1` can arrive while one is pending); the decision is the module's and tested. The single-click action toggles the chrome; the double-click action toggles the
zoom.

*Alternative rejected:* toggling on the first click and letting a double click zoom after —
the flash the spec forbids, and the reason the owner named the X app.

### D3. Zoom and pan are one pure module, `viewer-zoom.ts`

- `fitScale(natural, viewport)`: the scale that fits, `min(vw/nw, vh/nh, 1)` — never upscaled
  at the fit, which is what `object-contain` does today.
- `zoomTarget(natural, viewport)`: the double-click scale — `1` (natural pixels) when
  `1 > fitScale`, else `2 × fitScale`.
- `zoomStep(scale, direction, fit)`: multiply by `ZOOM_FACTOR` (1.25) per wheel notch, clamped
  to `[fit, ZOOM_MAX × fit]` with `ZOOM_MAX = 8`; a step that would go below the fit lands on
  the fit.
- `panOffset(pointer, viewport, content)`: for each axis, `(0.5 − fraction) × overflow`, where
  `fraction` is the pointer's position across the viewport clamped to `[0, 1]` and `overflow`
  is `max(content − viewport, 0)` — the image's translation *off its centred position*. At
  fraction 0 the near edges of image and viewport meet, at 1 the far edges do, at 0.5 the image
  is centred, and once the content no longer overflows every fraction is that centre. The
  "minimap" the owner describes is exactly this linear map.

  *Amended in review, from `−(fraction × overflow)`.* That was the same map measured from a
  top-left baseline, and it was right for a component that laid the image out in the viewport's
  corner. The component does not: the viewport centres its child (`flex items-center
  justify-center`), which is what keeps a fitted image — and an image smaller than the viewport
  — in the middle at no cost and with no branch. Read against that baseline the old formula
  takes the corner for the origin and needs a `(viewport − content) / 2` centring term added
  back on top; the two differ by exactly that term. One formula that pans *and* centres is the
  faithful reading of "a content smaller than the viewport centres" — stated there as a
  consequence, not as a special case.

The component keeps `scale` and the last pointer position; the `<img>` is drawn at `natural ×
scale` with `transform: translate(offset)` inside a viewport box, and `pointermove` on the
viewport updates the pointer — only while the image overflows it, since at the fit every move
would cost a `getBoundingClientRect` and a style write to land back on the same centred image.
`scale` is `number | null`, `null` reading as "at the fit", so `move()` forgets the scale
rather than computing the next image's fit before its `load` event has given a natural size;
until both that size and the viewport are measured the `<img>` keeps its `max-h-full max-w-full
object-contain` sizing, which draws the same size the fit does. The gesture that zooms asks
`displayScale > fit`, not `scale !== null`: a wheel step down clamps to the fit *as a number*,
and a double click there has to zoom in.

### D4. The margin is the viewport's inset, and the stage keeps the close gesture

The stage gains an inner viewport box inset by `PAN_MARGIN_PX` (24) on every side with
`overflow: hidden`; the image lives in it. The strip between the viewport and the stage's edge
is the stage itself, so the existing rule — a click whose target is the stage closes — holds
without a rectangle test. At the fit the viewport is what the image is fitted into, so the
margin is present at every zoom, as the spec says. A click on the viewport box beside a
centred, un-zoomed image is a click on the viewport, not the stage: the viewport forwards
those (target is itself) to the same close, so the empty space beside the picture still
closes as today.

### D5. Wheel

`wheel` on the viewport, `preventDefault` (always, so the page behind cannot scroll out from
under a still-loading picture), `deltaY < 0` zooms in, and `deltaY === 0` — a trackpad's
horizontal swipe — is no notch at all. The pointer position is
already the pan anchor, so nothing needs to be re-anchored after a step. Cmd/Ctrl-wheel is
left to the whole-app zoom (unchanged, it is bound on keys not wheel).

### D6. The counter and the title keep their place in the bar; the Tab reveal

Tab with the chrome hidden sets it shown, `flushSync()`, then runs `trapTab` on the same
event, so the first Tab lands on Previous. Shift-Tab likewise.

## Risks / Trade-offs

- [The 250 ms delay makes the bar feel slow to appear] → the owner chose it knowingly ("add
  delay on it, it's the behavior from X app"); the constant is one place to tune.
- [A `pointermove` per pixel while zoomed re-renders the transform] → it is one style write on
  one element; if it stutters on the 4000-pixel image, throttle to animation frames in the
  component, not the module.
- [`hidden` chrome is unreachable for a keyboard-only user who does not know Tab] → Escape
  and the arrows still work without it; the bar is discoverable by Tab and by click.
- [Reversing two non-goals] → the argument is in the proposal; the archived designs are not
  edited, the delta and this file are where the reversal is recorded.
