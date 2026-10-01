import { describe, expect, it } from 'vitest'
import { dropIndex, moveItem } from './reorder'

describe('moveItem', () => {
  const list = ['a', 'b', 'c', 'd']

  it('moves an item forward', () => {
    expect(moveItem(list, 0, 2)).toEqual(['b', 'c', 'a', 'd'])
  })

  it('moves an item backward', () => {
    expect(moveItem(list, 3, 1)).toEqual(['a', 'd', 'b', 'c'])
  })

  it('does not change the input', () => {
    moveItem(list, 0, 3)
    expect(list).toEqual(['a', 'b', 'c', 'd'])
  })

  it('returns the same list for the same index', () => {
    expect(moveItem(list, 2, 2)).toBe(list)
  })

  it('returns the same list when an index is out of range', () => {
    expect(moveItem(list, -1, 2)).toBe(list)
    expect(moveItem(list, 1, 4)).toBe(list)
    expect(moveItem(list, 9, 0)).toBe(list)
    expect(moveItem(list, 0.5, 2)).toBe(list)
  })

  it('returns a new array for a real move', () => {
    expect(moveItem(list, 0, 1)).not.toBe(list)
  })
})

describe('dropIndex', () => {
  it('lands before the row dragged onto from below', () => {
    expect(dropIndex(3, 1, false)).toBe(1)
  })

  it('lands after the row dragged onto from below', () => {
    expect(dropIndex(3, 1, true)).toBe(2)
  })

  it('lands before the row dragged onto from above', () => {
    expect(dropIndex(0, 2, false)).toBe(1)
  })

  it('lands after the row dragged onto from above', () => {
    expect(dropIndex(0, 2, true)).toBe(2)
  })

  it('reads the slots beside the dragged row as no move', () => {
    expect(dropIndex(2, 2, false)).toBe(2)
    expect(dropIndex(2, 2, true)).toBe(2)
    expect(dropIndex(2, 1, true)).toBe(2)
    expect(dropIndex(2, 3, false)).toBe(2)
  })
})
