import { describe, expect, it } from 'vitest'
import { Selection } from '$lib/api/selection.svelte'
import { KEY_DOWN, KEY_END, KEY_HOME, KEY_LEFT, KEY_RIGHT, KEY_UP } from '$lib/keyboard'
import { moveFocus } from './grid-focus'

/** The column counts a 120 / 180 / 360 tile produces in a typical window. */
const columnCounts = [3, 4, 7, 11]

describe('moveFocus', () => {
  it('steps one card left and right whatever the column count is', () => {
    for (const columns of columnCounts) {
      expect(moveFocus(5, KEY_RIGHT, columns, 40)).toBe(6)
      expect(moveFocus(5, KEY_LEFT, columns, 40)).toBe(4)
    }
  })

  it('steps one row by the column count', () => {
    for (const columns of columnCounts) {
      expect(moveFocus(2 * columns, KEY_DOWN, columns, 40)).toBe(3 * columns)
      expect(moveFocus(2 * columns, KEY_UP, columns, 40)).toBe(columns)
    }
  })

  it('clamps at the first card instead of wrapping or losing the focus', () => {
    for (const columns of columnCounts) {
      expect(moveFocus(0, KEY_LEFT, columns, 40)).toBe(0)
      expect(moveFocus(0, KEY_UP, columns, 40)).toBe(0)
      // Part-way through the first row: up must not fall off the top.
      expect(moveFocus(1, KEY_UP, columns, 40)).toBe(0)
    }
  })

  it('clamps at the last card, including from a short final row', () => {
    for (const columns of columnCounts) {
      const total = 3 * columns + 1
      expect(moveFocus(total - 1, KEY_RIGHT, columns, total)).toBe(total - 1)
      // Down from the second-to-last row lands on the last card, not past it.
      expect(moveFocus(total - 1 - columns, KEY_DOWN, columns, total)).toBe(total - 1)
      expect(moveFocus(total - 2, KEY_DOWN, columns, total)).toBe(total - 1)
    }
  })

  it('takes Home and End to the ends regardless of where the focus is', () => {
    expect(moveFocus(17, KEY_HOME, 5, 40)).toBe(0)
    expect(moveFocus(17, KEY_END, 5, 40)).toBe(39)
    expect(moveFocus(-1, KEY_HOME, 5, 40)).toBe(0)
    expect(moveFocus(-1, KEY_END, 5, 40)).toBe(39)
  })

  it('focuses the first card from nothing focused, in any direction', () => {
    for (const key of [KEY_LEFT, KEY_RIGHT, KEY_UP, KEY_DOWN]) {
      expect(moveFocus(-1, key, 5, 40)).toBe(0)
    }
  })

  it('moves nothing in an empty result', () => {
    for (const key of [KEY_LEFT, KEY_RIGHT, KEY_UP, KEY_DOWN, KEY_HOME, KEY_END]) {
      expect(moveFocus(-1, key, 5, 0)).toBeNull()
    }
  })

  it('leaves a key it does not own alone, so the event is not consumed', () => {
    expect(moveFocus(3, 'i', 5, 40)).toBeNull()
    expect(moveFocus(3, 'Enter', 5, 40)).toBeNull()
    expect(moveFocus(3, 'PageDown', 5, 40)).toBeNull()
  })

  it('never divides by a zero column count', () => {
    expect(moveFocus(3, KEY_DOWN, 0, 40)).toBe(4)
  })
})

// Design D7: a grouped result draws a heading row between groups, so a vertical
// move that ignored the slices would cross one and land in the wrong column.
describe('moveFocus over group slices', () => {
  /** Three groups of five, at indices 0-4, 5-9 and 10-14, in three columns. */
  const groups = [
    { key: 'a', count: 5 },
    { key: 'b', count: 5 },
    { key: 'c', count: 5 },
  ]
  const total = 15

  it('moves down inside a group', () => {
    expect(moveFocus(0, KEY_DOWN, 3, total, groups)).toBe(3)
  })

  it('steps into the next group at the same column from its last row', () => {
    // Index 3 is the first card of group a's short second row.
    expect(moveFocus(3, KEY_DOWN, 3, total, groups)).toBe(5)
    expect(moveFocus(4, KEY_DOWN, 3, total, groups)).toBe(6)
  })

  it('steps back into the previous group at its last row', () => {
    expect(moveFocus(5, KEY_UP, 3, total, groups)).toBe(3)
    // Column 2 of the previous group's last row does not exist; its last card does.
    expect(moveFocus(7, KEY_UP, 3, total, groups)).toBe(4)
  })

  it('clamps at the first and the last group', () => {
    expect(moveFocus(0, KEY_UP, 3, total, groups)).toBe(0)
    expect(moveFocus(14, KEY_DOWN, 3, total, groups)).toBe(14)
  })

  it('moves left and right straight through a group boundary', () => {
    expect(moveFocus(4, KEY_RIGHT, 3, total, groups)).toBe(5)
    expect(moveFocus(5, KEY_LEFT, 3, total, groups)).toBe(4)
  })
})

/**
 * The grid hands a shifted arrow the same `moveFocus` destination a plain one
 * gets and only then extends the range (design D5). What that has to buy is
 * asserted here: a shift-arrow at an edge stops there, rather than growing the
 * selection past the last card the way a bounded step would.
 */
describe('shift-arrow selection', () => {
  const columns = 5
  const total = 40

  /** The grid's `onkeydown` in one line: move, then rewrite the range. */
  function shiftArrow(selection: Selection, key: string) {
    const destination = moveFocus(selection.focus, key, columns, total)
    expect(destination).not.toBeNull()
    selection.extendTo(destination as number)
  }

  it('stops at the edges instead of growing the selection past them', () => {
    const selection = new Selection(
      () => Promise.resolve([]),
      () => Promise.resolve([]),
    )

    selection.focusAt(total - 1)
    for (const key of [KEY_RIGHT, KEY_DOWN, KEY_END]) {
      shiftArrow(selection, key)
      expect(selection.focus).toBe(total - 1)
      expect(selection.count).toBe(1)
    }

    selection.focusAt(0)
    for (const key of [KEY_LEFT, KEY_UP, KEY_HOME]) {
      shiftArrow(selection, key)
      expect(selection.focus).toBe(0)
      expect(selection.count).toBe(1)
    }
  })

  it('grows to the edge and no further', () => {
    const selection = new Selection(
      () => Promise.resolve([]),
      () => Promise.resolve([]),
    )
    selection.focusAt(total - 3)

    shiftArrow(selection, KEY_RIGHT)
    shiftArrow(selection, KEY_RIGHT)
    expect(selection.count).toBe(3)

    shiftArrow(selection, KEY_RIGHT)
    expect(selection.count).toBe(3)
    expect(selection.focus).toBe(total - 1)
  })
})
