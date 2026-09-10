// The selection model on its own: no grid, no IPC. The resolver is a spy, so
// what is asserted is when a range is turned into ids and with what — the round
// trip design D3 exists to make rare — as well as the gesture rules themselves.

import { expect, it, vi } from 'vitest'
import { Selection } from './selection.svelte'

/** Rows of a result: row `n` is image `i<n>`, as `search_ids` would answer. */
const idAt = (index: number) => `i${index}`

function selection(total = 100) {
  const resolve = vi.fn((offset: number, limit: number) =>
    Promise.resolve(
      Array.from({ length: Math.min(limit, total - offset) }, (_, i) => idAt(offset + i)),
    ),
  )
  return { selection: new Selection(resolve), resolve }
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

it('drops the written ids and keeps the rest of the selection', async () => {
  const { selection: sel } = selection()
  sel.focusAt(0)
  sel.extendTo(4)

  await sel.removeMany([idAt(1), idAt(3), 'never-selected'])

  expect(sel.count).toBe(3)
  expect(sel.has(1, idAt(1))).toBe(false)
  expect(sel.has(4, idAt(4))).toBe(true)
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
