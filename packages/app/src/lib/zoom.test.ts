import { expect, it } from 'vitest'
import { ZOOM_MAX, ZOOM_MIN, zoomStep } from './zoom'

it('steps up and down by a tenth', () => {
  expect(zoomStep(1, 1)).toBe(1.1)
  expect(zoomStep(1, -1)).toBe(0.9)
})

it('stops at the ends instead of overshooting', () => {
  expect(zoomStep(ZOOM_MAX, 1)).toBe(ZOOM_MAX)
  expect(zoomStep(ZOOM_MIN, -1)).toBe(ZOOM_MIN)
})

it('resets to one from anywhere', () => {
  expect(zoomStep(1.7, 0)).toBe(1)
  expect(zoomStep(0.6, 0)).toBe(1)
})

it('does not accumulate floating-point drift', () => {
  let factor = 1
  for (let i = 0; i < 3; i++) factor = zoomStep(factor, 1)
  expect(factor).toBe(1.3)
})
