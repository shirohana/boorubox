// @vitest-environment jsdom
import { describe, expect, it } from 'vitest'
import { clampHeight, roomAbove } from './section-resizer'

describe('clampHeight', () => {
  it('grows the section when the pointer moves up', () => {
    expect(clampHeight(100, 20, 40, 400)).toBe(120)
  })

  it('shrinks the section when the pointer moves down', () => {
    expect(clampHeight(100, -20, 40, 400)).toBe(80)
  })

  it('clamps at the floor', () => {
    expect(clampHeight(100, -1000, 40, 400)).toBe(40)
  })

  it('clamps at the ceiling', () => {
    expect(clampHeight(100, 1000, 40, 400)).toBe(400)
  })

  it('reads a NaN start as the floor, regardless of delta', () => {
    expect(clampHeight(NaN, 20, 40, 400)).toBe(40)
    expect(clampHeight(NaN, -20, 40, 400)).toBe(40)
  })
})

describe('roomAbove', () => {
  it('is the current height minus the computed min-height', () => {
    const list = document.createElement('div')
    list.style.minHeight = '128px'
    Object.defineProperty(list, 'clientHeight', { value: 300 })
    expect(roomAbove(list)).toBe(172)
  })

  it('is the full clientHeight when there is no min-height', () => {
    const list = document.createElement('div')
    Object.defineProperty(list, 'clientHeight', { value: 300 })
    expect(roomAbove(list)).toBe(300)
  })

  it('is 0 for a null list', () => {
    expect(roomAbove(null)).toBe(0)
  })
})
