// Where an arrow key moves the grid's focus. Pure, and separate from the
// component, so the column arithmetic can be asserted at every column count
// instead of being eyeballed against one window size.

import type { GroupSlice } from '@boorubox/shared'
import { offsetIndexClamped } from '$lib/domain/navigation-math'
import { KEY_DOWN, KEY_END, KEY_HOME, KEY_LEFT, KEY_RIGHT, KEY_UP } from '$lib/keyboard'

/**
 * The index `key` moves focus to, or `null` when the key moves nothing — which
 * is also the signal not to consume the event.
 *
 * Movement is CLAMPED, not bounded: pressing up in the first row keeps the
 * focus where it is rather than wrapping or losing it (`navigation-math` holds
 * that difference from the lightbox and says why).
 *
 * `groups` is the layout the grid is drawing (design D7). A row belongs to one
 * group, so up and down move inside it and step into the neighbouring group at
 * its edges; without the slices the vertical step would cross a short last row
 * and land on a card in a different column than the one it left.
 */
export function moveFocus(
  current: number,
  key: string,
  columns: number,
  total: number,
  groups: GroupSlice[] = [],
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
      return current < 0 ? 0 : vertical(current, -1, stride, total, groups)
    case KEY_DOWN:
      return current < 0 ? 0 : vertical(current, 1, stride, total, groups)
    default:
      return null
  }
}

/** Nothing focused yet: the first arrow press takes the first card, in any direction. */
function step(current: number, offset: number, total: number): number {
  return current < 0 ? 0 : offsetIndexClamped(current, offset, total)
}

interface Slice {
  first: number
  count: number
}

/** The groups as index ranges, or the whole result as one range when ungrouped. */
function slicesOf(groups: GroupSlice[], total: number): Slice[] {
  if (groups.length === 0) return [{ first: 0, count: total }]
  let first = 0
  return groups.map((group) => {
    const slice = { first, count: group.count }
    first += group.count
    return slice
  })
}

function vertical(
  current: number,
  direction: 1 | -1,
  columns: number,
  total: number,
  groups: GroupSlice[],
): number {
  const slices = slicesOf(groups, total)
  const at = slices.findIndex(
    (slice) => current >= slice.first && current < slice.first + slice.count,
  )
  if (at === -1) return offsetIndexClamped(current, direction * columns, total)

  const slice = slices[at]
  const local = current - slice.first
  const moved = local + direction * columns
  if (moved >= 0 && moved < slice.count) return slice.first + moved

  // No neighbouring group: the edge of the whole list, where the move clamps
  // onto the nearest card rather than doing nothing.
  const next = slices[at + direction]
  if (!next) return offsetIndexClamped(current, direction * columns, total)

  // Into the neighbouring group at the same column, or its last card when that
  // group's edge row is shorter than the column being left.
  const column = local % columns
  const target = direction === 1
    ? column
    : Math.floor((next.count - 1) / columns) * columns + column
  return next.first + Math.min(target, next.count - 1)
}
