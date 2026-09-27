// The selection model on its own: no grid, no IPC. The resolver is a spy, so
// what is asserted is when a range is turned into ids and with what — the round
// trip design D3 exists to make rare — as well as the gesture rules themselves.

import { expect, it, vi } from 'vitest'
import { type LoadedRows, relocate, Selection, shiftPast } from './selection.svelte'

/** Rows of a result: row `n` is image `i<n>`, as `search_ids` would answer. */
const idAt = (index: number) => `i${index}`

/**
 * `order` is the result as the database answers it now: a test that stores a
 * capture edits it in place, and `resolve` and `position` read the new order
 * from then on, the way `search_ids` and `search_position` do once the
 * capture's transaction has committed (`selection-by-id` design D3).
 */
function selection(total = 100) {
  const order = Array.from({ length: total }, (_, index) => idAt(index))
  const resolve = vi.fn((offset: number, limit: number) =>
    Promise.resolve(order.slice(offset, offset + limit)),
  )
  // Every test that cares what it answers builds its own; the default keeps
  // whatever it was asked about, which is a no-op prune.
  const matches = vi.fn((ids: string[]) => Promise.resolve(ids))
  const position = vi.fn((id: string) => {
    const index = order.indexOf(id)
    return Promise.resolve(index < 0 ? null : index)
  })
  return { selection: new Selection(resolve, matches, position), resolve, matches, position, order }
}

/** The rows a screen holds: `undefined` is a row whose page has not loaded. */
function rows(ids: (string | undefined)[]): LoadedRows {
  return {
    total: ids.length,
    at: (index) => {
      const id = ids[index]
      return id === undefined ? undefined : { id }
    },
  }
}

/** Only the first `loaded` rows of `order` loaded, as after a refresh's page 0. */
function firstRows(order: string[], loaded: number): LoadedRows {
  return rows(order.map((id, index) => (index < loaded ? id : undefined)))
}

it('focuses and clears on a plain click', async () => {
  const { selection: sel, resolve } = selection()
  sel.selectAll(100)

  await sel.click(7, idAt(7))

  expect(sel.count).toBe(0)
  expect(sel.focus).toBe(7)
  expect(sel.anchor).toBe(7)
  expect(resolve).not.toHaveBeenCalled()
})

it('toggles one image on a multi-select-click and leaves the rest', async () => {
  const { selection: sel } = selection()

  await sel.click(3, idAt(3), { multi: true })
  await sel.click(9, idAt(9), { multi: true })
  expect(sel.count).toBe(2)
  expect(sel.has(3, idAt(3))).toBe(true)
  expect(sel.has(9, idAt(9))).toBe(true)

  await sel.click(3, idAt(3), { multi: true })
  expect(sel.count).toBe(1)
  expect(sel.has(3, idAt(3))).toBe(false)
  expect(sel.has(9, idAt(9))).toBe(true)
})

// Design D2: a modifier-click over an empty selection also picks up the card
// the user was standing on, the way shift-click already includes both ends.
it('a multi-select-click over nothing selected also picks up the anchored card', async () => {
  const { selection: sel, resolve } = selection()

  await sel.click(10, idAt(10))
  sel.focusEntered(13)
  await sel.click(13, idAt(13), { multi: true })

  expect(sel.count).toBe(2)
  expect(sel.has(10, idAt(10))).toBe(true)
  expect(sel.has(13, idAt(13))).toBe(true)
  expect(resolve).toHaveBeenCalledExactlyOnceWith(10, 1)
})

it('a multi-select-click on the anchored card itself picks up nothing extra', async () => {
  const { selection: sel } = selection()

  await sel.click(10, idAt(10))
  await sel.click(10, idAt(10), { multi: true })

  expect(sel.count).toBe(1)
  expect(sel.has(10, idAt(10))).toBe(true)
})

it('the checkbox toggles only its own image after a plain click elsewhere', async () => {
  const { selection: sel } = selection()

  await sel.click(10, idAt(10))
  await sel.toggle(13, idAt(13))

  expect(sel.count).toBe(1)
  expect(sel.has(13, idAt(13))).toBe(true)
  expect(sel.has(10, idAt(10))).toBe(false)
})

it('takes a shift-click range on either side of the anchor', async () => {
  const { selection: sel } = selection()

  await sel.click(10, idAt(10))
  await sel.click(13, idAt(13), { range: true })
  expect(sel.count).toBe(4)
  expect(sel.has(10, undefined)).toBe(true)
  expect(sel.has(13, undefined)).toBe(true)
  expect(sel.has(14, idAt(14))).toBe(false)

  // The anchor did not move, so the other direction is measured from it too.
  await sel.click(7, idAt(7), { range: true })
  expect(sel.count).toBe(4)
  expect(sel.has(7, undefined)).toBe(true)
  expect(sel.has(10, undefined)).toBe(true)
  expect(sel.has(11, idAt(11))).toBe(false)
  expect(sel.focus).toBe(7)
})

it('grows and shrinks a shift-arrow range from a fixed anchor', () => {
  const { selection: sel } = selection()
  sel.focusAt(5)

  sel.extendTo(6)
  sel.extendTo(7)
  expect(sel.count).toBe(3)
  expect(sel.focus).toBe(7)

  sel.extendTo(6)
  expect(sel.count).toBe(2)
  expect(sel.has(7, idAt(7))).toBe(false)
  expect(sel.anchor).toBe(5)
})

// The grid focuses the card every gesture moves to, and the DOM answers with a
// `focusin` the tile reports back (`ImageCard`'s `onfocus`). These two go
// through `focusEntered` the way `LibraryGrid` does: without it the anchor
// followed the focus, and a shift-arrow selected only the last two cards.
it('keeps the anchor pinned while shift-arrows move the focus under it', () => {
  const { selection: sel } = selection()
  sel.focusAt(3)
  sel.focusEntered(3)

  sel.extendTo(4)
  sel.focusEntered(4)
  sel.extendTo(5)
  sel.focusEntered(5)
  expect(sel.count).toBe(3)
  expect(sel.has(3, undefined)).toBe(true)
  expect(sel.has(5, undefined)).toBe(true)
  expect(sel.anchor).toBe(3)

  sel.extendTo(4)
  sel.focusEntered(4)
  expect(sel.count).toBe(2)
  expect(sel.has(5, idAt(5))).toBe(false)
  expect(sel.anchor).toBe(3)

  // A plain arrow moves both again, so the next range starts where it landed.
  sel.focusAt(6)
  sel.focusEntered(6)
  expect(sel.anchor).toBe(6)
  sel.extendTo(7)
  expect(sel.count).toBe(2)
  expect(sel.has(3, idAt(3))).toBe(false)
})

it('anchors a shift-click where the focus was, not where the press landed', async () => {
  const { selection: sel } = selection()
  await sel.click(10, idAt(10))
  sel.focusEntered(10)

  // The press focuses the tile before the click can say shift was held.
  sel.focusEntered(13)
  await sel.click(13, idAt(13), { range: true })

  expect(sel.count).toBe(4)
  expect(sel.anchor).toBe(10)
  expect(sel.has(10, undefined)).toBe(true)
})

it('drops one image out of a range and keeps the rest', async () => {
  const { selection: sel, resolve } = selection()
  sel.focusAt(0)
  sel.extendTo(3)

  await sel.remove(idAt(2))

  expect(resolve).toHaveBeenCalledExactlyOnceWith(0, 4)
  expect(sel.count).toBe(3)
  expect(sel.has(2, idAt(2))).toBe(false)
  expect(sel.has(0, idAt(0))).toBe(true)
  // The strip removes a thumbnail; the card it belonged to stays the current one.
  expect(sel.focus).toBe(3)
})

it('a clear during an edit\'s resolve wins over the edit', async () => {
  const { selection: sel } = selection()
  sel.selectAll(100)

  const removing = sel.remove(idAt(4))
  sel.clear()
  await removing

  expect(sel.count).toBe(0)
})

it('keepMatching leaves only the ids the search still matches', async () => {
  const { selection: sel, matches } = selection()
  matches.mockImplementation((ids: string[]) => Promise.resolve(ids.filter((id) => id === idAt(1))))
  await sel.click(0, idAt(0), { multi: true })
  await sel.click(1, idAt(1), { multi: true })
  await sel.click(2, idAt(2), { multi: true })

  await sel.keepMatching()

  expect(sel.count).toBe(1)
  expect(sel.has(1, idAt(1))).toBe(true)
  expect(sel.has(0, idAt(0))).toBe(false)
  expect(sel.has(2, idAt(2))).toBe(false)
})

it('keepMatching resolves a live range before asking what still matches', async () => {
  const { selection: sel, resolve, matches } = selection()
  sel.focusAt(0)
  sel.extendTo(2)

  await sel.keepMatching()

  expect(resolve).toHaveBeenCalledExactlyOnceWith(0, 3)
  expect(matches).toHaveBeenCalledExactlyOnceWith([idAt(0), idAt(1), idAt(2)])
})

it('keepMatching makes no call over an empty selection', async () => {
  const { selection: sel, resolve, matches } = selection()

  await sel.keepMatching()

  expect(resolve).not.toHaveBeenCalled()
  expect(matches).not.toHaveBeenCalled()
})

it('a gesture during keepMatching\'s round trip wins over the prune', async () => {
  const { selection: sel, matches } = selection()
  await sel.click(0, idAt(0), { multi: true })
  await sel.click(1, idAt(1), { multi: true })
  let settle: (ids: string[]) => void = () => {}
  matches.mockReturnValue(new Promise((resolve) => (settle = resolve)))

  const pruning = sel.keepMatching()
  sel.clear()
  settle([idAt(1)])
  await pruning

  expect(sel.count).toBe(0)
})

it('counts a select-all without resolving anything', () => {
  const { selection: sel, resolve } = selection(10_000)
  sel.focusAt(42)

  sel.selectAll(10_000)

  expect(sel.count).toBe(10_000)
  expect(sel.has(9_999, undefined)).toBe(true)
  expect(resolve).not.toHaveBeenCalled()
})

it('resolves a range once, with its offset and limit, and stays in id mode', async () => {
  const { selection: sel, resolve } = selection()
  sel.focusAt(20)
  sel.extendTo(24)

  expect(await sel.ids()).toEqual([20, 21, 22, 23, 24].map(idAt))
  expect(resolve).toHaveBeenCalledExactlyOnceWith(20, 5)

  expect(await sel.ids()).toHaveLength(5)
  expect(resolve).toHaveBeenCalledTimes(1)
  expect(sel.count).toBe(5)
})

it('peekIds resolves a range without promoting it to ids', async () => {
  const { selection: sel, resolve } = selection()
  sel.focusAt(20)
  sel.extendTo(24)

  expect(await sel.peekIds()).toEqual([20, 21, 22, 23, 24].map(idAt))
  expect(resolve).toHaveBeenCalledExactlyOnceWith(20, 5)
  expect(sel.count).toBe(5)

  // Still a range, not promoted to ids: a second peek resolves again rather
  // than reading a cached id set the way a second `ids()` call would.
  expect(await sel.peekIds()).toEqual([20, 21, 22, 23, 24].map(idAt))
  expect(resolve).toHaveBeenCalledTimes(2)
})

it('peekIds reads a resolved selection straight, like ids does', async () => {
  const { selection: sel, resolve } = selection()
  await sel.click(3, idAt(3), { multi: true })
  await sel.click(9, idAt(9), { multi: true })

  const ids = await sel.peekIds()

  expect(new Set(ids)).toEqual(new Set([idAt(3), idAt(9)]))
  expect(resolve).not.toHaveBeenCalled()
})

it('resolves a live range before a multi-select-click toggles out of it', async () => {
  const { selection: sel, resolve } = selection()
  sel.focusAt(0)
  sel.extendTo(3)

  await sel.click(2, idAt(2), { multi: true })

  expect(resolve).toHaveBeenCalledExactlyOnceWith(0, 4)
  expect(sel.count).toBe(3)
  expect(sel.has(2, idAt(2))).toBe(false)
  expect(sel.has(0, idAt(0))).toBe(true)
})

it('leaves the focus alone when the selection is cleared', () => {
  const { selection: sel } = selection()
  sel.focusAt(12)
  sel.selectAll(100)

  sel.clear()

  expect(sel.count).toBe(0)
  expect(sel.focus).toBe(12)
})

it('shiftPast moves an index past every inserted row at or before it', () => {
  expect(shiftPast(5, [2, 9])).toBe(6)
  expect(shiftPast(5, [5])).toBe(6)
  expect(shiftPast(0, [0, 1])).toBe(2)
  expect(shiftPast(5, [9])).toBe(5)
})

it('relocate finds a loaded row without asking for its position', async () => {
  const position = vi.fn(() => Promise.resolve(40))

  const row = await relocate({ index: 0, id: 'b' }, rows(['a', 'b']), position)

  expect(row).toBe(1)
  expect(position).not.toHaveBeenCalled()
})

it('relocate asks for the position of a row that is not loaded', async () => {
  const position = vi.fn(() => Promise.resolve(40))

  const row = await relocate({ index: 3, id: 'z' }, rows(['a', 'b']), position)

  expect(row).toBe(40)
  expect(position).toHaveBeenCalledExactlyOnceWith('z')
})

it('relocate keeps the index of an image that left the result', async () => {
  const row = await relocate({ index: 3, id: 'z' }, rows(['a']), () => Promise.resolve(null))

  expect(row).toBe(3)
})

it('relocate shifts a pinned index that had no id', async () => {
  const position = vi.fn(() => Promise.resolve(40))

  const row = await relocate({ index: 3, id: undefined }, rows([]), position, [0, 7])

  expect(row).toBe(4)
  expect(position).not.toHaveBeenCalled()
})

it('pin turns a loaded range into its ids without a round trip', () => {
  const { selection: sel, resolve, order } = selection(7)
  sel.focusAt(0)
  sel.extendTo(2)

  sel.pin(rows(order))

  // Id mode answers nothing for a row without its id; a range would.
  expect(sel.has(0, undefined)).toBe(false)
  expect(sel.has(0, 'i0')).toBe(true)
  expect(sel.count).toBe(3)
  expect(resolve).not.toHaveBeenCalled()
})

it('pin leaves a range that reaches past the loaded rows a range', () => {
  const { selection: sel, resolve, order } = selection(7)
  sel.focusAt(0)
  sel.extendTo(4)

  sel.pin(firstRows(order, 3))

  expect(sel.has(4, undefined)).toBe(true)
  expect(sel.count).toBe(5)
  expect(resolve).not.toHaveBeenCalled()
})

it('the owner\'s repro: a capture above a loaded range keeps the same three images', async () => {
  const { selection: sel, order } = selection(7)
  sel.focusAt(0)
  sel.extendTo(2)

  const before = [...order]
  order.unshift('new')
  const pin = sel.pin(rows(before))
  await sel.repin(pin, rows(order), ['new'])

  expect(sel.has(1, 'i0')).toBe(true)
  expect(sel.has(3, 'i2')).toBe(true)
  expect(sel.has(0, 'new')).toBe(false)
  expect(sel.count).toBe(3)
  expect(sel.focus).toBe(3)
  expect(sel.anchor).toBe(1)
})

it('repin puts the focus and the anchor back on their images', async () => {
  const { selection: sel, position, order } = selection(7)
  sel.focusAt(1)
  sel.focusEntered(4)

  const pin = sel.pin(rows(order))
  order.reverse()
  await sel.repin(pin, rows(order))

  expect(sel.focus).toBe(2)
  expect(sel.anchor).toBe(5)
  expect(position).not.toHaveBeenCalled()
})

it('repin asks for the position of a focus off the loaded rows', async () => {
  const { selection: sel, position, order } = selection(100)
  sel.focusAt(50)

  const pin = sel.pin(rows(order))
  order.unshift('new')
  await sel.repin(pin, firstRows(order, 10))

  expect(position).toHaveBeenCalledWith('i50')
  expect(sel.focus).toBe(51)
})

it('a gesture during the re-read keeps its focus', async () => {
  const { selection: sel, order } = selection(7)
  sel.focusAt(2)

  const pin = sel.pin(rows(order))
  order.unshift('new')
  sel.focusAt(5)
  await sel.repin(pin, rows(order), ['new'])

  expect(sel.focus).toBe(5)
  expect(sel.anchor).toBe(5)
})

it('select-all stays a range across a capture above it', async () => {
  const { selection: sel, resolve, order } = selection(100)
  sel.selectAll(100)

  const pin = sel.pin(firstRows(order, 10))
  order.unshift('new')
  await sel.repin(pin, firstRows(order, 10), ['new'])

  expect(sel.count).toBe(100)
  expect(sel.has(0, 'new')).toBe(false)
  expect(sel.has(1, undefined)).toBe(true)
  expect(sel.has(100, undefined)).toBe(true)
  expect(resolve).not.toHaveBeenCalled()
})

it('a capture below a range leaves it alone', async () => {
  const { selection: sel, resolve, order } = selection(20)
  sel.focusAt(0)
  sel.extendTo(4)

  const pin = sel.pin(firstRows(order, 0))
  order.splice(10, 0, 'new')
  await sel.repin(pin, firstRows(order, 0), ['new'])

  expect(sel.count).toBe(5)
  expect(sel.has(0, undefined)).toBe(true)
  expect(sel.has(4, undefined)).toBe(true)
  expect(sel.has(5, undefined)).toBe(false)
  expect(resolve).not.toHaveBeenCalled()
})

it('a capture inside a range is resolved out of it', async () => {
  const { selection: sel, resolve, order } = selection(20)
  sel.focusAt(2)
  sel.extendTo(5)

  const pin = sel.pin(firstRows(order, 0))
  order.splice(4, 0, 'new')
  await sel.repin(pin, firstRows(order, 0), ['new'])

  expect(resolve).toHaveBeenCalledExactlyOnceWith(2, 5)
  expect(sel.has(4, 'new')).toBe(false)
  expect(sel.has(2, 'i2')).toBe(true)
  expect(sel.has(6, 'i5')).toBe(true)
  expect(sel.count).toBe(4)
})

it('a capture at a range\'s first row shifts the range without resolving it', async () => {
  const { selection: sel, resolve, order } = selection(20)
  sel.focusAt(2)
  sel.extendTo(5)

  const pin = sel.pin(firstRows(order, 0))
  order.splice(2, 0, 'new')
  await sel.repin(pin, firstRows(order, 0), ['new'])

  expect(resolve).not.toHaveBeenCalled()
  expect(sel.count).toBe(4)
  expect(sel.has(2, undefined)).toBe(false)
  expect(sel.has(3, undefined)).toBe(true)
  expect(sel.has(6, undefined)).toBe(true)
})

it('a capture at a range\'s last row is resolved out of it', async () => {
  const { selection: sel, resolve, order } = selection(20)
  sel.focusAt(2)
  sel.extendTo(5)

  const pin = sel.pin(firstRows(order, 0))
  order.splice(5, 0, 'new')
  await sel.repin(pin, firstRows(order, 0), ['new'])

  expect(resolve).toHaveBeenCalledExactlyOnceWith(2, 5)
  expect(sel.has(5, 'new')).toBe(false)
  expect(sel.has(6, 'i5')).toBe(true)
  expect(sel.count).toBe(4)
})

it('a multi-select-click over an unloaded range keeps its edit when a capture shifts the range', async () => {
  const { selection: sel, resolve, order } = selection(20)
  sel.focusAt(2)
  sel.extendTo(5)
  let answer!: () => void
  resolve.mockImplementationOnce((offset, limit) => {
    const ids = order.slice(offset, offset + limit)
    return new Promise((done) => {
      answer = () => done(ids)
    })
  })

  const click = sel.click(9, 'i9', { multi: true })
  const pin = sel.pin(firstRows(order, 0))
  order.unshift('new')
  await sel.repin(pin, firstRows(order, 0), ['new'])
  answer()
  await click

  expect(sel.count).toBe(5)
  expect(sel.has(10, 'i9')).toBe(true)
  expect(sel.has(3, 'i2')).toBe(true)
  expect(sel.has(6, 'i5')).toBe(true)
  expect(sel.has(0, 'new')).toBe(false)
})

it('an unloaded focus follows the capture\'s row', async () => {
  const { selection: sel, position, order } = selection(20)
  sel.focusAt(5)

  const pin = sel.pin(firstRows(order, 0))
  order.unshift('new')
  await sel.repin(pin, firstRows(order, 0), ['new'])

  expect(sel.focus).toBe(6)
  expect(sel.anchor).toBe(6)
  expect(position).toHaveBeenCalledExactlyOnceWith('new')
})

it('the strip shows the same images after a capture', async () => {
  const { selection: sel, order } = selection(7)
  sel.focusAt(0)
  sel.extendTo(2)
  const before = sel.previewIds(10, (index) => order[index])

  const pin = sel.pin(rows(order))
  order.unshift('new')
  await sel.repin(pin, rows(order), ['new'])

  expect(before).toEqual(['i0', 'i1', 'i2'])
  expect(sel.previewIds(10, (index) => order[index])).toEqual(before)
})

it('repin returns the inserted rows\' positions, sorted', async () => {
  const { selection: sel, order } = selection(7)

  const pin = sel.pin(rows(order))
  order.unshift('a')
  order.splice(3, 0, 'b')
  const positions = await sel.repin(pin, rows(order), ['b', 'elsewhere', 'a'])

  expect(positions).toEqual([0, 3])
})
