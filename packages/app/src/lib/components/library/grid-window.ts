// Which rows the grid renders, given how far it is scrolled.
//
// This is the whole answer to "stays responsive with ten thousand images": the
// number of cards in the DOM depends on the viewport, never on the library. It
// is a separate module from the component so that claim can be asserted in a
// test rather than eyeballed in a running app.

import { GRID_TILE_MAX, GRID_TILE_MIN } from '@boorubox/shared'

export const GAP = 12
/** Padding between the scroll container and the cards. */
export const EDGE = 12
/** Rows kept mounted beyond the viewport, so a scroll does not show holes. */
export const OVERSCAN_ROWS = 2

function clampTile(tile: number): number {
  // The slider clamps too, but a stored setting from a hand-edited file or an
  // older build reaches here unfiltered: an unclamped 0 gives zero columns and
  // the row math divides by it.
  if (!Number.isFinite(tile)) return GRID_TILE_MIN
  return Math.max(GRID_TILE_MIN, Math.min(GRID_TILE_MAX, tile))
}

export interface Viewport {
  total: number
  scrollTop: number
  /** `clientWidth` of the scroll container, padding included. */
  width: number
  height: number
  /** Target edge length of one tile in px (design D11). */
  tile: number
}

export interface GridWindow {
  columns: number
  /**
   * Height of one row: a tile is square, so this is the width a `1fr` column
   * actually gets, plus the gap. Deriving it from `tile` instead would let the
   * rows the CSS grid lays out drift from the ones this module positions.
   */
  rowHeight: number
  /** Height of the full list, so the scrollbar matches the row count. */
  contentHeight: number
  /** Offset of the first rendered row inside the list. */
  offsetTop: number
  /** Rows `[firstIndex, endIndex)` are the ones to render. */
  firstIndex: number
  endIndex: number
}

export function gridWindow({ total, scrollTop, width, height, tile }: Viewport): GridWindow {
  const edge = clampTile(tile)
  const contentWidth = Math.max(0, width - EDGE * 2)
  const columns = Math.max(1, Math.floor((contentWidth + GAP) / (edge + GAP)))
  // `tile` is the size a column may not go below, so the leftover width is
  // shared out and a tile is a little larger than asked for; before the
  // container is measured there is no width to share and `tile` stands in.
  const columnWidth = contentWidth > 0
    ? (contentWidth - (columns - 1) * GAP) / columns
    : edge
  const row = columnWidth + GAP
  const rowCount = Math.ceil(total / columns)
  const firstRow = Math.max(0, Math.floor(scrollTop / row) - OVERSCAN_ROWS)
  const lastRow = Math.min(rowCount, Math.ceil((scrollTop + height) / row) + OVERSCAN_ROWS)

  return {
    columns,
    rowHeight: row,
    contentHeight: rowCount * row,
    offsetTop: firstRow * row,
    firstIndex: firstRow * columns,
    endIndex: Math.max(firstRow * columns, Math.min(total, lastRow * columns)),
  }
}
