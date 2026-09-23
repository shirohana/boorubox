import { describe, expect, it } from 'vitest'
import { fillOf, fillState, toggledSelection, toggledTag } from './pinned-state'

describe('fillOf', () => {
  it('reads none for 0 of 12', () => {
    expect(fillOf(0, 12)).toBe('none')
  })

  it('reads some for 4 of 12', () => {
    expect(fillOf(4, 12)).toBe('some')
  })

  it('reads all for 12 of 12', () => {
    expect(fillOf(12, 12)).toBe('all')
  })

  it('reads none for 0 of 0', () => {
    expect(fillOf(0, 0)).toBe('none')
  })
})

describe('fillState', () => {
  it('reads none for a tag absent from the counts', () => {
    expect(fillState('tagme', [], 12)).toBe('none')
    expect(fillState('tagme', [{ name: 'other', count: 12 }], 12)).toBe('none')
  })

  it('reads all when every selected image carries it', () => {
    expect(fillState('tagme', [{ name: 'tagme', count: 12 }], 12)).toBe('all')
  })

  it('reads some for a count between none and all', () => {
    expect(fillState('tagme', [{ name: 'tagme', count: 4 }], 12)).toBe('some')
  })

  it('reads none for an empty selection', () => {
    expect(fillState('tagme', [], 0)).toBe('none')
  })
})

describe('toggledSelection', () => {
  it('adds unless every selected image already carries the tag', () => {
    expect(toggledSelection('none', 'tagme')).toEqual({ add: ['tagme'], remove: [] })
    expect(toggledSelection('some', 'tagme')).toEqual({ add: ['tagme'], remove: [] })
  })

  it('removes when all of them carry it', () => {
    expect(toggledSelection('all', 'tagme')).toEqual({ add: [], remove: ['tagme'] })
  })
})

describe('toggledTag', () => {
  it('adds a tag the set does not have', () => {
    expect(toggledTag(['1girl'], 'tagme')).toEqual(['1girl', 'tagme'])
  })

  it('drops a tag the set already has', () => {
    expect(toggledTag(['1girl', 'tagme'], 'tagme')).toEqual(['1girl'])
  })
})
