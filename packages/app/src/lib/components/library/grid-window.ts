// Which rows the grid renders, given how far it is scrolled.
//
// This is the whole answer to "stays responsive with ten thousand images": the
// number of cards in the DOM depends on the viewport, never on the library. It
// is a separate module from the component so that claim can be asserted in a
// test rather than eyeballed in a running app.

/** Narrowest a card may be before a column is dropped. */
export const MIN_CARD_WIDTH = 168
export const CARD_HEIGHT = 200
export const GAP = 12
/** Padding between the scroll container and the cards. */
export const EDGE = 12
/** Rows kept mounted beyond the viewport, so a scroll does not show holes. */
export const OVERSCAN_ROWS = 2

export const ROW_HEIGHT = CARD_HEIGHT + GAP

export interface Viewport {
  total: number
  scrollTop: number
  /** `clientWidth` of the scroll container, padding included. */
  width: number
  height: number
}

export interface GridWindow {
  columns: number
  /** Height of the full list, so the scrollbar matches the row count. */
  contentHeight: number
  /** Offset of the first rendered row inside the list. */
  offsetTop: number
  /** Rows `[firstIndex, endIndex)` are the ones to render. */
  firstIndex: number
  endIndex: number
}

export function gridWindow({ total, scrollTop, width, height }: Viewport): GridWindow {
  const contentWidth = Math.max(0, width - EDGE * 2)
  const columns = Math.max(1, Math.floor((contentWidth + GAP) / (MIN_CARD_WIDTH + GAP)))
  const rowCount = Math.ceil(total / columns)
  const firstRow = Math.max(0, Math.floor(scrollTop / ROW_HEIGHT) - OVERSCAN_ROWS)
  const lastRow = Math.min(rowCount, Math.ceil((scrollTop + height) / ROW_HEIGHT) + OVERSCAN_ROWS)

  return {
    columns,
    contentHeight: rowCount * ROW_HEIGHT,
    offsetTop: firstRow * ROW_HEIGHT,
    firstIndex: firstRow * columns,
    endIndex: Math.max(firstRow * columns, Math.min(total, lastRow * columns)),
  }
}
