// The viewer's zoom and pan arithmetic (design D7-D8). Pure and tested without
// a DOM: the component only measures the natural image, the viewport box and
// the pointer, and draws the result — the fit, the cover, the click target,
// the wheel/pinch factor and the pan mapping are decided here.

export interface Size {
  width: number
  height: number
}

export interface Point {
  x: number
  y: number
}

/** The zoomed ceiling, as a multiple of the fit. */
export const ZOOM_MAX = 8

/** Per-pixel sensitivity of a plain wheel notch. */
export const WHEEL_SENSITIVITY = 0.002
/** Per-pixel sensitivity of a ctrl-wheel pinch (WebView2's deltas are small). */
export const PINCH_SENSITIVITY = 0.01

/** How far, as a fraction of the viewport, the pan's "minimap" box extends on each axis. */
export const MINIMAP_FRACTION = 1 / 3

/** How long a click's zoom takes to arrive. */
export const ZOOM_EASE_MS = 200

/**
 * The scale `elapsed` ms into a click's zoom from `from` to `to`: ease-out
 * (cubic), so the image moves fast at first and settles. Pure, for the
 * component's frame loop — the loop only reads the clock and writes the
 * answer. At and past `ZOOM_EASE_MS` this is exactly `to`.
 */
export function zoomAt(from: number, to: number, elapsed: number): number {
  const t = Math.max(0, Math.min(1, elapsed / ZOOM_EASE_MS))
  const eased = 1 - (1 - t) ** 3
  return t >= 1 ? to : from + (to - from) * eased
}

const LINE_HEIGHT_PX = 16
const PAGE_HEIGHT_PX = 800

/** The scale that fits `natural` into `viewport` without cropping. Never upscales. */
export function fitScale(natural: Size, viewport: Size): number {
  return Math.min(viewport.width / natural.width, viewport.height / natural.height, 1)
}

/**
 * The scale that covers `viewport` with `natural` — the larger of the two
 * axis ratios, so the image overflows along the other axis only. Unlike
 * `fitScale` this upscales freely: covering a viewport bigger than the image
 * is the ask, not a defect (design D7's risk "cover upscales a small image").
 */
export function coverScale(natural: Size, viewport: Size): number {
  return Math.max(viewport.width / natural.width, viewport.height / natural.height)
}

/**
 * The scale a click zooms to: the cover, or — when the image's aspect matches
 * the viewport's exactly, so the cover coincides with the fit — twice the
 * fit, so the click always visibly zooms.
 */
export function clickTarget(natural: Size, viewport: Size): number {
  const fit = fitScale(natural, viewport)
  const cover = coverScale(natural, viewport)
  return cover > fit ? cover : 2 * fit
}

/**
 * `scale` multiplied by `factor` and clamped to `[fit, ZOOM_MAX * fit]`. A
 * factor that would land below the fit lands on the fit instead.
 */
export function zoomBy(scale: number, factor: number, fit: number): number {
  return Math.max(fit, Math.min(ZOOM_MAX * fit, scale * factor))
}

/** The minimal shape `wheelZoomFactor` reads off a wheel event. */
export interface WheelZoomEvent {
  deltaY: number
  deltaMode: number
  ctrlKey: boolean
}

/**
 * A wheel (or WebView2 ctrl-wheel pinch) event as a multiplicative zoom
 * factor. `deltaY` is normalised to pixels first — `deltaMode` 1 is lines
 * (× 16), 2 is pages (× 800) — then run through `exp(-pixels *
 * sensitivity)`, so a mouse notch (~100px) is ×≈1.22 and a trackpad's
 * few-pixel events are a few percent each. `ctrlKey` marks Chromium's pinch
 * gesture (WebView2 on Windows), whose deltas are small enough to need the
 * steeper `PINCH_SENSITIVITY`.
 */
export function wheelZoomFactor(event: WheelZoomEvent): number {
  const pixels = event.deltaMode === 1
    ? event.deltaY * LINE_HEIGHT_PX
    : event.deltaMode === 2
      ? event.deltaY * PAGE_HEIGHT_PX
      : event.deltaY
  const sensitivity = event.ctrlKey ? PINCH_SENSITIVITY : WHEEL_SENSITIVITY
  return Math.exp(-pixels * sensitivity)
}

/**
 * The image's translation off its centred position, so the pointer's
 * position across the viewport chooses what part of the image is shown —
 * the "minimap" the owner described. The image is drawn centred in the
 * viewport by the surrounding layout; this is the additional shift from that
 * centre, per axis: the pointer's fraction across a box centred on the
 * viewport, `MINIMAP_FRACTION` of its size (clamped to `[0, 1]`, so a pointer
 * outside the box rests at the nearer edge), against the overflow between
 * `content` and `viewport` — a fraction of 0 uncovers the near edge, 1 the
 * far edge, and 0.5 — or any fraction once `content` no longer overflows
 * `viewport` — is the centre unmoved. Shrinking the box to a third of the
 * viewport (rather than reading the whole of it) is what lets a small
 * movement of the pointer reach the far end of the image.
 */
export function panOffset(pointer: Point, viewport: Size, content: Size): Point {
  return {
    x: axisOffset(pointer.x, viewport.width, content.width),
    y: axisOffset(pointer.y, viewport.height, content.height),
  }
}

function axisOffset(pointer: number, viewportSize: number, contentSize: number): number {
  const overflow = Math.max(contentSize - viewportSize, 0)
  const boxSize = viewportSize * MINIMAP_FRACTION
  const boxStart = (viewportSize - boxSize) / 2
  const fraction = boxSize <= 0
    ? 0.5
    : Math.max(0, Math.min(1, (pointer - boxStart) / boxSize))
  const shift = (0.5 - fraction) * overflow
  // A fraction above 0.5 with no overflow gives `-0`, the same centred
  // position under a different sign. `shift === 0` is true for both zeroes and
  // false for a NaN, which `|| 0` would have swallowed instead of surfacing.
  return shift === 0 ? 0 : shift
}
