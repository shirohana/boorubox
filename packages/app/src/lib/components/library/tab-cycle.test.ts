import { describe, expect, it } from 'vitest'
import { nextTabStop } from './tab-cycle'

describe('nextTabStop', () => {
  it('steps forward and back through the stops', () => {
    expect(nextTabStop(4, 0, false)).toBe(1)
    expect(nextTabStop(4, 2, true)).toBe(1)
  })

  it('wraps at both ends, which is what makes it a trap', () => {
    expect(nextTabStop(4, 3, false)).toBe(0)
    expect(nextTabStop(4, 0, true)).toBe(3)
  })

  it('enters the cycle from the end the direction is reaching for', () => {
    // The viewer's focus surface: focused, and not a stop.
    expect(nextTabStop(4, -1, false)).toBe(0)
    expect(nextTabStop(4, -1, true)).toBe(3)
  })

  it('treats a stop that is gone like no stop at all', () => {
    // The inspector closed while its control held the focus: the index the
    // caller found no longer names anything in the list.
    expect(nextTabStop(2, 5, false)).toBe(0)
    expect(nextTabStop(2, 5, true)).toBe(1)
  })

  it('moves nowhere when there is nothing to move to', () => {
    expect(nextTabStop(0, -1, false)).toBeNull()
    expect(nextTabStop(0, 0, true)).toBeNull()
  })

  it('stays on the only stop there is', () => {
    expect(nextTabStop(1, 0, false)).toBe(0)
    expect(nextTabStop(1, 0, true)).toBe(0)
  })
})
