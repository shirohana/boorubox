import { describe, expect, it } from 'vitest'
import { shouldActivate, TILE_DRAG_SLOP_PX, travelled } from './tile-click'

const at = (x: number, y: number) => ({ x, y, multi: false, range: false })
const pressAt = (x: number, y: number, wasCurrent: boolean) => ({ x, y, wasCurrent })

describe('shouldActivate', () => {
  it('opens the tile that was already current when the press began', () => {
    expect(shouldActivate(pressAt(10, 10, true), at(10, 10))).toBe(true)
  })

  it('only makes a tile current when it was not', () => {
    expect(shouldActivate(pressAt(10, 10, false), at(10, 10))).toBe(false)
  })

  it('does not open when the pointer travelled before the release', () => {
    const far = TILE_DRAG_SLOP_PX + 1
    expect(shouldActivate(pressAt(10, 10, true), at(10 + far, 10))).toBe(false)
    expect(shouldActivate(pressAt(10, 10, true), at(10, 10 - far))).toBe(false)
  })

  it('opens through the jitter of a still hand', () => {
    expect(shouldActivate(pressAt(10, 10, true), at(11, 12))).toBe(true)
  })

  it('does not open for a selection gesture', () => {
    expect(shouldActivate(pressAt(10, 10, true), { ...at(10, 10), multi: true })).toBe(false)
    expect(shouldActivate(pressAt(10, 10, true), { ...at(10, 10), range: true })).toBe(false)
  })

  it('does not open for a click no press of its own preceded', () => {
    expect(shouldActivate(null, at(10, 10))).toBe(false)
  })
})

describe('travelled', () => {
  it('measures the diagonal, not one axis', () => {
    expect(travelled({ x: 0, y: 0 }, { x: 3, y: 3 })).toBe(true)
    expect(travelled({ x: 0, y: 0 }, { x: 3, y: 0 })).toBe(false)
  })

  it('holds the threshold itself to be a click', () => {
    expect(travelled({ x: 0, y: 0 }, { x: TILE_DRAG_SLOP_PX, y: 0 })).toBe(false)
  })
})
