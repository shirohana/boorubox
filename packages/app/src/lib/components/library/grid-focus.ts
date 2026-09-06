// Where an arrow key moves the grid's focus. Pure, and separate from the
// component, so the column arithmetic can be asserted at every column count
// instead of being eyeballed against one window size.

import { offsetIndexClamped } from '$lib/domain/navigation-math'
import { KEY_DOWN, KEY_END, KEY_HOME, KEY_LEFT, KEY_RIGHT, KEY_UP } from '$lib/keyboard'

/**
 * The index `key` moves focus to, or `null` when the key moves nothing — which
 * is also the signal not to consume the event.
 *
 * Movement is CLAMPED, not bounded: pressing up in the first row keeps the
 * focus where it is rather than wrapping or losing it (`navigation-math` holds
 * that difference from the lightbox and says why).
 */
export function moveFocus(
  current: number,
  key: string,
  columns: number,
  total: number,
): number | null {
  if (total <= 0) return null
  const stride = Math.max(1, columns)

  switch (key) {
    case KEY_HOME:
      return 0
    case KEY_END:
      return total - 1
    case KEY_LEFT:
      return step(current, -1, total)
    case KEY_RIGHT:
      return step(current, 1, total)
    case KEY_UP:
      return step(current, -stride, total)
    case KEY_DOWN:
      return step(current, stride, total)
    default:
      return null
  }
}

/** Nothing focused yet: the first arrow press takes the first card, in any direction. */
function step(current: number, offset: number, total: number): number {
  return current < 0 ? 0 : offsetIndexClamped(current, offset, total)
}
