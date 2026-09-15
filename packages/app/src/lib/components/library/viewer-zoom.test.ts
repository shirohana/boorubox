import { describe, expect, it } from 'vitest'
import { fitScale, panOffset, zoomStep, zoomTarget, ZOOM_MAX } from './viewer-zoom'

describe('fitScale', () => {
  it('never upscales', () => {
    expect(fitScale({ width: 400, height: 300 }, { width: 1500, height: 900 })).toBe(1)
  })

  it('scales down to the tighter axis', () => {
    expect(fitScale({ width: 4000, height: 3000 }, { width: 1500, height: 900 })).toBeCloseTo(0.3)
  })
})

describe('zoomTarget', () => {
  it('zooms a large image to its natural size', () => {
    const natural = { width: 4000, height: 3000 }
    const viewport = { width: 1500, height: 900 }
    expect(zoomTarget(natural, viewport)).toBe(1)
  })

  it('zooms a small image to twice its fit', () => {
    const natural = { width: 400, height: 300 }
    const viewport = { width: 1500, height: 900 }
    // fitScale is 1 (never upscaled), so the target is 2.
    expect(zoomTarget(natural, viewport)).toBe(2)
  })
})

describe('zoomStep', () => {
  it('steps up and down by the zoom factor', () => {
    expect(zoomStep(1, 1, 0.5)).toBeCloseTo(1.25)
    expect(zoomStep(1.25, -1, 0.5)).toBeCloseTo(1)
  })

  it('clamps at the fit', () => {
    expect(zoomStep(0.5, -1, 0.5)).toBe(0.5)
  })

  it('clamps at the ceiling', () => {
    const fit = 0.5
    expect(zoomStep(ZOOM_MAX * fit, 1, fit)).toBe(ZOOM_MAX * fit)
  })
})

describe('panOffset', () => {
  const viewport = { width: 100, height: 100 }
  // Overflows on the x axis only (200px of overflow), so the y assertions
  // below all read 0: nothing to pan on that axis.
  const content = { width: 300, height: 100 }

  it('at fraction 0 uncovers the left edge', () => {
    expect(panOffset({ x: 0, y: 50 }, viewport, content)).toEqual({ x: 100, y: 0 })
  })

  it('at fraction 0.5 stays centred', () => {
    expect(panOffset({ x: 50, y: 50 }, viewport, content)).toEqual({ x: 0, y: 0 })
  })

  it('at fraction 1 uncovers the right edge', () => {
    expect(panOffset({ x: 100, y: 50 }, viewport, content)).toEqual({ x: -100, y: 0 })
  })

  it('a content smaller than the viewport centres, whatever the pointer', () => {
    const small = { width: 50, height: 50 }
    expect(panOffset({ x: 0, y: 0 }, viewport, small)).toEqual({ x: 0, y: 0 })
    expect(panOffset({ x: 100, y: 100 }, viewport, small)).toEqual({ x: 0, y: 0 })
  })

  it('a pointer outside the viewport clamps to the near edge', () => {
    expect(panOffset({ x: -50, y: 50 }, viewport, content)).toEqual({ x: 100, y: 0 })
    expect(panOffset({ x: 150, y: 50 }, viewport, content)).toEqual({ x: -100, y: 0 })
  })
})
