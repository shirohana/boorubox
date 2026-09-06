// Which rows the grid renders, given how far it is scrolled.
//
// This is the whole answer to "stays responsive with ten thousand images": the
// number of cards in the DOM depends on the viewport, never on the library. It
// is a separate module from the component so that claim can be asserted in a
// test rather than eyeballed in a running app.
//
// Grouping is layout, not membership: the group slices arrive with the search
// (design D7), so every heading, its count and the row it sits on are
// arithmetic over those counts — right before the page under them has loaded.

import type { GroupSlice } from '@boorubox/shared'
import { GRID_TILE_MAX, GRID_TILE_MIN } from '@boorubox/shared'

export const GAP = 12
/** Padding between the scroll container and the cards. */
export const EDGE = 12
/** Rows kept mounted beyond the viewport, so a scroll does not show holes. */
export const OVERSCAN_ROWS = 2
/** Height of a group heading row, gap included. */
export const HEADING_HEIGHT = 28

function clampTile(tile: number): number {
  // The slider clamps too, but a stored setting from a hand-edited file or an
  // older build reaches here unfiltered: an unclamped 0 gives zero columns and
  // the row math divides by it.
  if (!Number.isFinite(tile)) return GRID_TILE_MIN
  return Math.max(GRID_TILE_MIN, Math.min(GRID_TILE_MAX, tile))
}

export interface Viewport {
  total: number
  /** Empty while ungrouped: one section holding every row, with no heading. */
  groups: GroupSlice[]
  scrollTop: number
  /** `clientWidth` of the scroll container, padding included. */
  width: number
  height: number
  /** Target edge length of one tile in px (design D11). */
  tile: number
}

/** One group's place in the list, or the whole list when ungrouped. */
export interface Section {
  /** `null` means no heading: the ungrouped result is one nameless section. */
  key: string | null
  count: number
  /** Absolute index of this section's first image in the result. */
  first: number
  /** Top of the heading, or of the first tile row when there is none. */
  top: number
  /** Top of the first tile row. */
  tilesTop: number
  /** Just past the last tile row. */
  bottom: number
}

export type GridRow
  = | { kind: 'heading', key: string, count: number, top: number }
    | { kind: 'tiles', first: number, count: number, top: number }

export interface GridWindow {
  columns: number
  /**
   * Height of one row: a tile is square, so this is the width a `1fr` column
   * actually gets, plus the gap. Deriving it from `tile` instead would let the
   * rows the CSS grid lays out drift from the ones this module positions.
   */
  rowHeight: number
  headingHeight: number
  /** Height of the full list, so the scrollbar matches the row count. */
  contentHeight: number
  /** Every section, so the grid can scroll one image into view. */
  sections: Section[]
  /** The rows to render, each carrying its own offset in the list. */
  rows: GridRow[]
  /** Images `[firstIndex, endIndex)` are the ones the rendered rows show. */
  firstIndex: number
  endIndex: number
}

export function columnsFor(width: number, tile: number): number {
  const contentWidth = Math.max(0, width - EDGE * 2)
  return Math.max(1, Math.floor((contentWidth + GAP) / (clampTile(tile) + GAP)))
}

/** Where every group starts and ends, top to bottom. */
export function sectionsOf(
  groups: GroupSlice[],
  total: number,
  columns: number,
  rowHeight: number,
): Section[] {
  const slices = groups.length > 0
    ? groups.map((group) => ({ key: group.key as string | null, count: group.count }))
    : [{ key: null, count: total }]

  const sections: Section[] = []
  let first = 0
  let top = 0
  for (const slice of slices) {
    const tilesTop = top + (slice.key === null ? 0 : HEADING_HEIGHT)
    const bottom = tilesTop + Math.ceil(slice.count / columns) * rowHeight
    sections.push({ key: slice.key, count: slice.count, first, top, tilesTop, bottom })
    first += slice.count
    top = bottom
  }
  return sections
}

/** The y a card sits at, which is what scrolling one into view needs. */
export function imageTop(sections: Section[], index: number, columns: number, row: number): number {
  const section = sections.find((s) => index >= s.first && index < s.first + s.count)
  if (!section) return 0
  return section.tilesTop + Math.floor((index - section.first) / columns) * row
}

export function gridWindow({
  total,
  groups,
  scrollTop,
  width,
  height,
  tile,
}: Viewport): GridWindow {
  const edge = clampTile(tile)
  const contentWidth = Math.max(0, width - EDGE * 2)
  const columns = columnsFor(width, tile)
  // `tile` is the size a column may not go below, so the leftover width is
  // shared out and a tile is a little larger than asked for; before the
  // container is measured there is no width to share and `tile` stands in.
  const columnWidth = contentWidth > 0
    ? (contentWidth - (columns - 1) * GAP) / columns
    : edge
  const row = columnWidth + GAP
  const sections = sectionsOf(groups, total, columns, row)

  const top = scrollTop - OVERSCAN_ROWS * row
  const bottom = scrollTop + height + OVERSCAN_ROWS * row
  const rows: GridRow[] = []
  let firstIndex = -1
  let endIndex = 0

  for (const section of sections) {
    if (section.key !== null && section.tilesTop > top && section.top < bottom) {
      rows.push({ kind: 'heading', key: section.key, count: section.count, top: section.top })
    }

    const rowCount = Math.ceil(section.count / columns)
    const from = Math.min(rowCount, Math.max(0, Math.floor((top - section.tilesTop) / row)))
    const to = Math.min(rowCount, Math.max(0, Math.ceil((bottom - section.tilesTop) / row)))
    for (let index = from; index < to; index++) {
      const first = section.first + index * columns
      const count = Math.min(columns, section.first + section.count - first)
      rows.push({ kind: 'tiles', first, count, top: section.tilesTop + index * row })
      if (firstIndex < 0) firstIndex = first
      endIndex = first + count
    }
  }

  return {
    columns,
    rowHeight: row,
    headingHeight: HEADING_HEIGHT,
    contentHeight: sections.at(-1)?.bottom ?? 0,
    sections,
    rows,
    firstIndex: firstIndex < 0 ? 0 : firstIndex,
    endIndex: firstIndex < 0 ? 0 : endIndex,
  }
}
