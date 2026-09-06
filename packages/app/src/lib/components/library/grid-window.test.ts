import { expect, it } from 'vitest'
import { gridWindow, ROW_HEIGHT } from './grid-window'

/** A 1400 x 900 window, roughly the default 1200 x 800 app window maximised. */
const viewport = { width: 1400, height: 900 }

it('drops a column rather than shrinking a card below its minimum', () => {
  expect(gridWindow({ total: 100, scrollTop: 0, width: 400, height: 900 }).columns).toBe(2)
  expect(gridWindow({ total: 100, scrollTop: 0, width: 200, height: 900 }).columns).toBe(1)
  // Even narrower than one card: never zero columns, or the row math divides by 0.
  expect(gridWindow({ total: 100, scrollTop: 0, width: 40, height: 900 }).columns).toBe(1)
})

it('renders the same number of cards for ten thousand images as for one hundred', () => {
  const hundred = gridWindow({ total: 100, scrollTop: 0, ...viewport })
  const tenThousand = gridWindow({ total: 10_000, scrollTop: 0, ...viewport })

  expect(tenThousand.endIndex - tenThousand.firstIndex)
    .toBe(hundred.endIndex - hundred.firstIndex)
})

it('keeps the mounted card count bounded at every scroll position of a 10k library', () => {
  const total = 10_000
  const { contentHeight, columns } = gridWindow({ total, scrollTop: 0, ...viewport })
  // What the viewport shows, plus the overscan rows above and below it.
  const ceiling = columns * (Math.ceil(viewport.height / ROW_HEIGHT) + 5)

  for (let scrollTop = 0; scrollTop <= contentHeight; scrollTop += ROW_HEIGHT / 2) {
    const window = gridWindow({ total, scrollTop, ...viewport })
    expect(window.endIndex - window.firstIndex).toBeLessThanOrEqual(ceiling)
    expect(window.endIndex).toBeLessThanOrEqual(total)
    expect(window.firstIndex).toBeLessThanOrEqual(window.endIndex)
  }
})

it('sizes the scroll content to every row, not to the rendered ones', () => {
  const { contentHeight, columns } = gridWindow({ total: 10_000, scrollTop: 0, ...viewport })
  expect(contentHeight).toBe(Math.ceil(10_000 / columns) * ROW_HEIGHT)
})

it('places the rendered block where the rows it holds belong', () => {
  const scrollTop = 40 * ROW_HEIGHT
  const { offsetTop, firstIndex, columns } = gridWindow({ total: 10_000, scrollTop, ...viewport })

  expect(offsetTop).toBe(firstIndex / columns * ROW_HEIGHT)
  expect(offsetTop).toBeLessThanOrEqual(scrollTop)
})

it('renders nothing for an empty result', () => {
  const window = gridWindow({ total: 0, scrollTop: 0, ...viewport })
  expect(window.endIndex - window.firstIndex).toBe(0)
  expect(window.contentHeight).toBe(0)
})

it('stops at the last row when scrolled to the bottom', () => {
  const total = 10_000
  const { contentHeight } = gridWindow({ total, scrollTop: 0, ...viewport })
  const window = gridWindow({ total, scrollTop: contentHeight - viewport.height, ...viewport })

  expect(window.endIndex).toBe(total)
})
