import type { GroupSlice } from '@boorubox/shared'
import { GRID_TILE_DEFAULT, GRID_TILE_MAX, GRID_TILE_MIN } from '@boorubox/shared'
import { describe, expect, it } from 'vitest'
import { EDGE, GAP, gridWindow, HEADING_HEIGHT, imageTop } from './grid-window'

/** A 1400 x 900 window, roughly the default 1200 x 800 app window maximised. */
const viewport = { width: 1400, height: 900, tile: GRID_TILE_DEFAULT, groups: [] }

/** Every tile size the slider can produce, so no assertion is tied to the default. */
const tiles = [GRID_TILE_MIN, GRID_TILE_DEFAULT, GRID_TILE_MAX]

/** Every image the window renders, in order. */
function shownImages(window: ReturnType<typeof gridWindow>): number[] {
  return window.rows.flatMap((row) =>
    row.kind === 'tiles' ? Array.from({ length: row.count }, (_, i) => row.first + i) : [],
  )
}

describe('columns', () => {
  it('fits as many tiles of the asked-for size as the content width holds', () => {
    const contentWidth = viewport.width - EDGE * 2
    for (const tile of tiles) {
      expect(gridWindow({ total: 100, scrollTop: 0, ...viewport, tile }).columns)
        .toBe(Math.floor((contentWidth + GAP) / (tile + GAP)))
    }
  })

  it('drops a column rather than shrinking a tile below the asked-for size', () => {
    const narrow = { total: 100, scrollTop: 0, height: 900, tile: GRID_TILE_DEFAULT, groups: [] }
    expect(gridWindow({ ...narrow, width: 420 }).columns).toBe(2)
    expect(gridWindow({ ...narrow, width: 230 }).columns).toBe(1)
    // Even narrower than one tile: never zero columns, or the row math divides by 0.
    expect(gridWindow({ ...narrow, width: 40 }).columns).toBe(1)
  })

  it('gives a larger tile fewer columns and a taller row', () => {
    const small = gridWindow({ total: 100, scrollTop: 0, ...viewport, tile: GRID_TILE_MIN })
    const large = gridWindow({ total: 100, scrollTop: 0, ...viewport, tile: GRID_TILE_MAX })

    expect(small.columns).toBeGreaterThan(large.columns)
    expect(small.rowHeight).toBeLessThan(large.rowHeight)
  })

  it('reports the row height the CSS grid will produce, not the asked-for tile', () => {
    // Square tiles in `1fr` columns: the row is as tall as a column is wide, so
    // a row height read off `tile` would leave the absolute offsets short.
    const contentWidth = viewport.width - EDGE * 2
    for (const tile of tiles) {
      const { columns, rowHeight } = gridWindow({ total: 100, scrollTop: 0, ...viewport, tile })
      expect(rowHeight).toBe((contentWidth - (columns - 1) * GAP) / columns + GAP)
      expect(rowHeight).toBeGreaterThanOrEqual(tile + GAP)
    }
  })

  it('clamps a tile size outside the settable range instead of losing every column', () => {
    const bad = { total: 100, scrollTop: 0, ...viewport }
    expect(gridWindow({ ...bad, tile: 0 }))
      .toEqual(gridWindow({ ...bad, tile: GRID_TILE_MIN }))
    expect(gridWindow({ ...bad, tile: 10_000 }))
      .toEqual(gridWindow({ ...bad, tile: GRID_TILE_MAX }))
    expect(gridWindow({ ...bad, tile: Number.NaN }))
      .toEqual(gridWindow({ ...bad, tile: GRID_TILE_MIN }))
  })
})

describe('virtualisation', () => {
  it('renders the same number of cards for ten thousand images as for one hundred', () => {
    for (const tile of tiles) {
      const hundred = gridWindow({ total: 100, scrollTop: 0, ...viewport, tile })
      const tenThousand = gridWindow({ total: 10_000, scrollTop: 0, ...viewport, tile })

      expect(tenThousand.endIndex - tenThousand.firstIndex)
        .toBe(hundred.endIndex - hundred.firstIndex)
    }
  })

  it('keeps the mounted card count bounded at every scroll position of a 10k library', () => {
    const total = 10_000
    for (const tile of tiles) {
      const { contentHeight, columns, rowHeight } = gridWindow({
        total,
        scrollTop: 0,
        ...viewport,
        tile,
      })
      // What the viewport shows, plus the overscan rows above and below it.
      const ceiling = columns * (Math.ceil(viewport.height / rowHeight) + 5)

      for (let scrollTop = 0; scrollTop <= contentHeight; scrollTop += rowHeight / 2) {
        const window = gridWindow({ total, scrollTop, ...viewport, tile })
        expect(window.endIndex - window.firstIndex).toBeLessThanOrEqual(ceiling)
        expect(window.endIndex).toBeLessThanOrEqual(total)
        expect(window.firstIndex).toBeLessThanOrEqual(window.endIndex)
      }
    }
  })

  it('sizes the scroll content to every row, not to the rendered ones', () => {
    for (const tile of tiles) {
      const { contentHeight, columns, rowHeight } = gridWindow({
        total: 10_000,
        scrollTop: 0,
        ...viewport,
        tile,
      })
      expect(contentHeight).toBe(Math.ceil(10_000 / columns) * rowHeight)
    }
  })

  it('places each rendered row where the images it holds belong', () => {
    for (const tile of tiles) {
      const scrollTop = 40 * (tile + GAP)
      const window = gridWindow({ total: 10_000, scrollTop, ...viewport, tile })
      const first = window.rows[0]

      expect(first.kind).toBe('tiles')
      expect(first.top).toBe(window.firstIndex / window.columns * window.rowHeight)
      expect(first.top).toBeLessThanOrEqual(scrollTop)
      for (const row of window.rows) {
        if (row.kind !== 'tiles') continue
        expect(row.top).toBe(row.first / window.columns * window.rowHeight)
      }
    }
  })

  it('renders nothing for an empty result', () => {
    const window = gridWindow({ total: 0, scrollTop: 0, ...viewport })
    expect(window.endIndex - window.firstIndex).toBe(0)
    expect(window.rows).toEqual([])
    expect(window.contentHeight).toBe(0)
  })

  it('stops at the last row when scrolled to the bottom', () => {
    const total = 10_000
    for (const tile of tiles) {
      const { contentHeight } = gridWindow({ total, scrollTop: 0, ...viewport, tile })
      const window = gridWindow({
        total,
        scrollTop: contentHeight - viewport.height,
        ...viewport,
        tile,
      })

      expect(window.endIndex).toBe(total)
    }
  })
})

// Design D7: the slices come back with the search, so the layout of a grouped
// result is arithmetic — a heading is right before its own page has loaded.
describe('grouping', () => {
  /** `count` groups of `size`, named for their position. */
  function groups(count: number, size: number): GroupSlice[] {
    return Array.from({ length: count }, (_, i) => ({ key: `group-${i}`, count: size }))
  }

  const forty = groups(40, 25)
  const total = 40 * 25
  const grouped = { ...viewport, total, groups: forty }

  it('gives every group a heading row and a content height that holds them all', () => {
    const { columns, rowHeight, contentHeight } = gridWindow({ ...grouped, scrollTop: 0 })
    const rowsPerGroup = Math.ceil(25 / columns)

    // Summed section by section rather than multiplied, so the comparison is to
    // within floating-point noise rather than bit-for-bit.
    expect(contentHeight).toBeCloseTo(40 * (HEADING_HEIGHT + rowsPerGroup * rowHeight), 6)
  })

  it('renders a heading above the tiles of its own group', () => {
    const window = gridWindow({ ...grouped, scrollTop: 0 })
    const first = window.rows[0]

    expect(first).toMatchObject({ kind: 'heading', key: 'group-0', count: 25, top: 0 })
    expect(window.rows[1]).toMatchObject({ kind: 'tiles', first: 0 })
    expect(window.firstIndex).toBe(0)
  })

  it('puts the fortieth group where the counts say, before anything above it loaded', () => {
    const zero = gridWindow({ ...grouped, scrollTop: 0 })
    const rowsPerGroup = Math.ceil(25 / zero.columns)
    const heightPerGroup = HEADING_HEIGHT + rowsPerGroup * zero.rowHeight
    const fortieth = 39 * heightPerGroup

    const window = gridWindow({ ...grouped, scrollTop: fortieth })
    const heading = window.rows.find((row) => row.kind === 'heading' && row.key === 'group-39')

    expect(heading?.count).toBe(25)
    expect(heading?.top).toBeCloseTo(fortieth, 6)
    // Its first tile is the 976th image, which no page of the result has loaded.
    expect(shownImages(window)).toContain(39 * 25)
    expect(imageTop(window.sections, 39 * 25, window.columns, window.rowHeight))
      .toBeCloseTo(fortieth + HEADING_HEIGHT, 6)
  })

  it('never renders more rows for forty groups than the viewport holds', () => {
    const ungrouped = gridWindow({ ...viewport, total, groups: [], scrollTop: 0 })
    const window = gridWindow({ ...grouped, scrollTop: 0 })

    // One heading per group is at most one extra row per group in view.
    expect(window.rows.length).toBeLessThanOrEqual(ungrouped.rows.length * 2 + 2)
    expect(shownImages(window).length).toBeLessThanOrEqual(
      ungrouped.endIndex - ungrouped.firstIndex,
    )
  })

  it('shows every image of the result exactly once, scrolling top to bottom', () => {
    const seen = new Set<number>()
    const { contentHeight, rowHeight } = gridWindow({ ...grouped, scrollTop: 0 })
    for (let scrollTop = 0; scrollTop <= contentHeight; scrollTop += rowHeight / 2) {
      for (const index of shownImages(gridWindow({ ...grouped, scrollTop }))) seen.add(index)
    }

    expect(seen.size).toBe(total)
  })

  it('leaves a short last row short rather than borrowing from the next group', () => {
    const uneven = [{ key: 'a', count: 1 }, { key: 'b', count: 2 }]
    const window = gridWindow({ ...viewport, total: 3, groups: uneven, scrollTop: 0 })
    const tileRows = window.rows.filter((row) => row.kind === 'tiles')

    expect(tileRows).toEqual([
      { kind: 'tiles', first: 0, count: 1, top: HEADING_HEIGHT },
      {
        kind: 'tiles',
        first: 1,
        count: 2,
        top: HEADING_HEIGHT * 2 + window.rowHeight,
      },
    ])
  })
})
