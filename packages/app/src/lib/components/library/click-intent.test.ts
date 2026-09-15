import { describe, expect, it } from 'vitest'
import { clickIntent } from './click-intent'

describe('clickIntent', () => {
  it('a first click schedules the single-click action', () => {
    expect(clickIntent(1, false)).toBe('schedule')
  })

  it('a second click within the window cancels it and is the double', () => {
    expect(clickIntent(2, true)).toBe('double')
  })

  it('a detail === 2 with nothing pending is ignored', () => {
    expect(clickIntent(2, false)).toBe('ignore')
  })

  it('a detail === 1 after the window closed schedules again', () => {
    expect(clickIntent(1, false)).toBe('schedule')
  })

  it('a triple click while pending is still the double, not a third state', () => {
    expect(clickIntent(3, true)).toBe('double')
  })
})
