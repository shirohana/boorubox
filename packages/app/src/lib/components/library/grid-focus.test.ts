import { describe, expect, it } from 'vitest'
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
