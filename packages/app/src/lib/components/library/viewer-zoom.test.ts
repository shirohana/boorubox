import { describe, expect, it } from 'vitest'
import {
  ZOOM_EASE_MS,
  zoomAt,
  clickTarget,
  coverScale,
  fitScale,
  MINIMAP_FRACTION,
  panOffset,
  wheelZoomFactor,
  zoomBy,
  ZOOM_MAX,
} from './viewer-zoom'

describe('fitScale', () => {
  it('never upscales', () => {
    expect(fitScale({ width: 400, height: 300 }, { width: 1500, height: 900 })).toBe(1)
  })

  it('scales down to the tighter axis', () => {
    expect(fitScale({ width: 4000, height: 3000 }, { width: 1500, height: 900 })).toBeCloseTo(0.3)
  })
})

describe('coverScale', () => {
  it('covers at the width when the image is relatively tall', () => {
    const cover = coverScale({ width: 4000, height: 3000 }, { width: 1500, height: 900 })
    expect(cover).toBeCloseTo(0.375)
  })

  it('covers at the height when the image is relatively wide', () => {
    expect(coverScale({ width: 4000, height: 1000 }, { width: 1500, height: 900 })).toBeCloseTo(0.9)
  })

  it('upscales a small image to cover', () => {
    expect(coverScale({ width: 400, height: 300 }, { width: 1500, height: 900 })).toBeCloseTo(3.75)
  })
})

describe('clickTarget', () => {
  it('zooms to the cover when the cover overflows the fit', () => {
    const natural = { width: 4000, height: 3000 }
    const viewport = { width: 1500, height: 900 }
    expect(clickTarget(natural, viewport)).toBeCloseTo(coverScale(natural, viewport))
  })

  it('zooms to twice the fit when the aspect matches the viewport exactly', () => {
    // 3000x1800 is the same 5:3 aspect as the 1500x900 viewport, so the cover
    // and the fit coincide — the fallback has to fire, not the cover branch.
    const natural = { width: 3000, height: 1800 }
    const viewport = { width: 1500, height: 900 }
    const fit = fitScale(natural, viewport)
    expect(coverScale(natural, viewport)).toBeCloseTo(fit)
    expect(clickTarget(natural, viewport)).toBeCloseTo(2 * fit)
  })
})

describe('zoomBy', () => {
  it('multiplies the scale by the factor', () => {
    expect(zoomBy(1, 1.25, 0.5)).toBeCloseTo(1.25)
  })

  it('clamps at the fit', () => {
    expect(zoomBy(0.5, 0.5, 0.5)).toBe(0.5)
  })

  it('clamps at the ceiling', () => {
    const fit = 0.5
    expect(zoomBy(ZOOM_MAX * fit, 1.5, fit)).toBe(ZOOM_MAX * fit)
  })
})

describe('wheelZoomFactor', () => {
  it('a 100px notch is about ×1.22', () => {
    expect(wheelZoomFactor({ deltaY: -100, deltaMode: 0, ctrlKey: false })).toBeCloseTo(1.22, 2)
  })

  it('a 5px trackpad step is about ×1.01', () => {
    expect(wheelZoomFactor({ deltaY: -5, deltaMode: 0, ctrlKey: false })).toBeCloseTo(1.01, 2)
  })

  it('a notch the other way zooms out', () => {
    const factor = wheelZoomFactor({ deltaY: 100, deltaMode: 0, ctrlKey: false })
    expect(factor).toBeCloseTo(1 / 1.2214, 3)
  })

  it('deltaMode 1 scales lines to 16px', () => {
    const lines = wheelZoomFactor({ deltaY: -1, deltaMode: 1, ctrlKey: false })
    const pixels = wheelZoomFactor({ deltaY: -16, deltaMode: 0, ctrlKey: false })
    expect(lines).toBeCloseTo(pixels)
  })

  it('deltaMode 2 scales pages to 800px', () => {
    const pages = wheelZoomFactor({ deltaY: -1, deltaMode: 2, ctrlKey: false })
    const pixels = wheelZoomFactor({ deltaY: -800, deltaMode: 0, ctrlKey: false })
    expect(pages).toBeCloseTo(pixels)
  })

  it('a ctrl wheel uses the steeper pinch sensitivity', () => {
    const ctrl = wheelZoomFactor({ deltaY: -10, deltaMode: 0, ctrlKey: true })
    const plain = wheelZoomFactor({ deltaY: -10, deltaMode: 0, ctrlKey: false })
    expect(ctrl).toBeCloseTo(Math.exp(0.1))
    expect(ctrl).toBeGreaterThan(plain)
  })
})

describe('panOffset', () => {
  it('MINIMAP_FRACTION is a third of the viewport', () => {
    expect(MINIMAP_FRACTION).toBeCloseTo(1 / 3)
  })

  it('a content smaller than the viewport centres, whatever the pointer', () => {
    const viewport = { width: 900, height: 900 }
    const small = { width: 300, height: 300 }
    expect(panOffset({ x: 0, y: 0 }, viewport, small)).toEqual({ x: 0, y: 0 })
    expect(panOffset({ x: 900, y: 900 }, viewport, small)).toEqual({ x: 0, y: 0 })
  })

  it('reads 0 at or before the minimap box, 0.5 at its centre, 1 at or after it', () => {
    // 900 wide with MINIMAP_FRACTION 1/3 puts the box's edges at 300 and 600.
    const viewport = { width: 900, height: 900 }
    // Overflows on x only (200px), so every y assertion below reads 0.
    const content = { width: 1100, height: 900 }
    expect(panOffset({ x: 0, y: 450 }, viewport, content)).toEqual({ x: 100, y: 0 })
    expect(panOffset({ x: 300, y: 450 }, viewport, content)).toEqual({ x: 100, y: 0 })
    expect(panOffset({ x: 450, y: 450 }, viewport, content)).toEqual({ x: 0, y: 0 })
    expect(panOffset({ x: 600, y: 450 }, viewport, content)).toEqual({ x: -100, y: 0 })
    expect(panOffset({ x: 900, y: 450 }, viewport, content)).toEqual({ x: -100, y: 0 })
  })
})

describe('zoomAt', () => {
  it('starts at the start and ends exactly at the target', () => {
    expect(zoomAt(0.5, 2, 0)).toBe(0.5)
    expect(zoomAt(0.5, 2, ZOOM_EASE_MS)).toBe(2)
    expect(zoomAt(0.5, 2, ZOOM_EASE_MS * 5)).toBe(2)
  })

  it('eases out: past half the time it is past half the way', () => {
    const halfway = zoomAt(1, 3, ZOOM_EASE_MS / 2)
    expect(halfway).toBeGreaterThan(2)
    expect(halfway).toBeLessThan(3)
  })

  it('is monotone frame by frame, in both directions', () => {
    const frames = [0, 16, 33, 50, 100, 150, 199, 200]
    const up = frames.map((ms) => zoomAt(1, 4, ms))
    const down = frames.map((ms) => zoomAt(4, 1, ms))
    for (let i = 1; i < frames.length; i++) {
      expect(up[i]).toBeGreaterThanOrEqual(up[i - 1])
      expect(down[i]).toBeLessThanOrEqual(down[i - 1])
    }
  })
})
