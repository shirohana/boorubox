// What a click on a tile means (design D7). It is a decision about a press,
// not about a DOM node, so it lives here with its test rather than inside the
// component, where the two ways it went wrong in the running app — a stale
// current-ness and a press that travelled — could only be found by hand.

/**
 * How far a press may travel and still be a click. Beyond it the user pressed
 * on the picture, changed their mind and moved away: the release is the end of
 * a drag, and a drag never opens the viewer. Four pixels is above the jitter a
 * deliberate trackpad click produces and far below a deliberate move.
 */
export const TILE_DRAG_SLOP_PX = 4

/** Where a press started, and what the tile was when it did. */
export interface TilePress {
  x: number
  y: number
  /**
   * Whether this tile was the grid's current card as the press began. Read at
   * `pointerdown`, never at `click`: the context-menu trigger wrapping the tile
   * carries `tabindex="-1"`, and WebKit focuses a tabbable element on mousedown,
   * so by the time the click arrives the tile has already made itself current.
   */
  wasCurrent: boolean
}

/** Where the press ended, and what it was held down with. */
export interface TileRelease {
  x: number
  y: number
  /** The platform's multi-select modifier. */
  multi: boolean
  /** Shift. */
  range: boolean
}

/**
 * A click opens the tile only when it is the second click on the card the
 * inspector is already describing (D7), the pointer stayed put, and no modifier
 * turned the gesture into a selection. A click with no press behind it — a
 * synthesised one — is not that gesture; the keyboard opens through the grid's
 * own `Enter` and `Space`.
 */
export function shouldActivate(press: TilePress | null, release: TileRelease): boolean {
  if (!press || !press.wasCurrent) return false
  if (release.multi || release.range) return false
  return !travelled(press, release)
}

/** Whether the pointer moved far enough between the two points to be a drag. */
export function travelled(
  press: { x: number, y: number },
  release: { x: number, y: number },
): boolean {
  return Math.hypot(release.x - press.x, release.y - press.y) > TILE_DRAG_SLOP_PX
}
