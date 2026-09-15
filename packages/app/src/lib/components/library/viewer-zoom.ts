// The viewer's zoom and pan arithmetic (design D3). Pure and tested without a
// DOM: the component only measures the natural image, the viewport box and
// the pointer, and draws the result — the fit, the double-click target, the
// wheel step and the pan mapping are decided here.

export interface Size {
  width: number
  height: number
}

export interface Point {
  x: number
  y: number
}

/** Multiplier per wheel notch. */
export const ZOOM_FACTOR = 1.25
/** The zoomed ceiling, as a multiple of the fit. */
export const ZOOM_MAX = 8

/** The scale that fits `natural` into `viewport` without cropping. Never upscales. */
export function fitScale(natural: Size, viewport: Size): number {
  return Math.min(viewport.width / natural.width, viewport.height / natural.height, 1)
}

/**
 * The scale a double click zooms to: natural pixels, or — when the image is
 * already at or below its fitted size — twice the fit, so a small image still
 * visibly zooms.
 */
export function zoomTarget(natural: Size, viewport: Size): number {
  const fit = fitScale(natural, viewport)
  return 1 > fit ? 1 : 2 * fit
}

/**
 * The scale after one wheel notch in `direction`, clamped to `[fit, ZOOM_MAX *
 * fit]`. A step that would land below the fit lands on the fit instead.
 */
export function zoomStep(scale: number, direction: 1 | -1, fit: number): number {
  const stepped = direction === 1 ? scale * ZOOM_FACTOR : scale / ZOOM_FACTOR
  return Math.max(fit, Math.min(ZOOM_MAX * fit, stepped))
}

/**
 * The image's translation off its centred position, so the pointer's
 * position across the viewport chooses what part of the image is shown —
 * the "minimap" the owner described. The image is drawn centred in the
 * viewport by the surrounding layout; this is the additional shift from that
 * centre, per axis: the pointer's fraction across the viewport (clamped to
 * `[0, 1]`) against the overflow between `content` and `viewport`, so a
 * fraction of 0 uncovers the near edge, 1 the far edge, and 0.5 — or any
 * fraction once `content` no longer overflows `viewport` — is the centre
 * unmoved.
 */
export function panOffset(pointer: Point, viewport: Size, content: Size): Point {
  return {
    x: axisOffset(pointer.x, viewport.width, content.width),
    y: axisOffset(pointer.y, viewport.height, content.height),
  }
}

function axisOffset(pointer: number, viewportSize: number, contentSize: number): number {
  const overflow = Math.max(contentSize - viewportSize, 0)
  const fraction = viewportSize <= 0
    ? 0.5
    : Math.max(0, Math.min(1, pointer / viewportSize))
  const shift = (0.5 - fraction) * overflow
  // A fraction above 0.5 with no overflow gives `-0`, the same centred
  // position under a different sign. `shift === 0` is true for both zeroes and
  // false for a NaN, which `|| 0` would have swallowed instead of surfacing.
  return shift === 0 ? 0 : shift
}
