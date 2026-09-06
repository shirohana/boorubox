import { GRID_TILE_DEFAULT, GRID_TILE_MAX, GRID_TILE_MIN } from '@boorubox/shared'
import { describe, expect, it } from 'vitest'
import { EDGE, GAP, gridWindow } from './grid-window'

/** A 1400 x 900 window, roughly the default 1200 x 800 app window maximised. */
const viewport = { width: 1400, height: 900, tile: GRID_TILE_DEFAULT }

/** Every tile size the slider can produce, so no assertion is tied to the default. */
const tiles = [GRID_TILE_MIN, GRID_TILE_DEFAULT, GRID_TILE_MAX]

describe('columns', () => {
  it('fits as many tiles of the asked-for size as the content width holds', () => {
    const contentWidth = viewport.width - EDGE * 2
    for (const tile of tiles) {
      expect(gridWindow({ total: 100, scrollTop: 0, ...viewport, tile }).columns)
        .toBe(Math.floor((contentWidth + GAP) / (tile + GAP)))
    }
  })

  it('drops a column rather than shrinking a tile below the asked-for size', () => {
    const narrow = { total: 100, scrollTop: 0, height: 900, tile: GRID_TILE_DEFAULT }
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

  it('places the rendered block where the rows it holds belong', () => {
    for (const tile of tiles) {
      const scrollTop = 40 * (tile + GAP)
      const { offsetTop, firstIndex, columns, rowHeight } = gridWindow({
        total: 10_000,
        scrollTop,
        ...viewport,
        tile,
      })

      expect(offsetTop).toBe(firstIndex / columns * rowHeight)
      expect(offsetTop).toBeLessThanOrEqual(scrollTop)
    }
  })

  it('renders nothing for an empty result', () => {
    const window = gridWindow({ total: 0, scrollTop: 0, ...viewport })
    expect(window.endIndex - window.firstIndex).toBe(0)
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
