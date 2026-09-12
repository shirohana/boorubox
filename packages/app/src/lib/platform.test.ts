// @vitest-environment jsdom

import { afterEach, describe, expect, it, vi } from 'vitest'

// The module reads the platform once at import, so each case loads it fresh
// after stamping (or clearing) the attribute `app.html` sets.
async function loadWith(platform: string | undefined) {
  vi.resetModules()
  if (platform === undefined) delete document.documentElement.dataset.platform
  else document.documentElement.dataset.platform = platform
  return await import('./platform')
}

afterEach(() => {
  delete document.documentElement.dataset.platform
})

describe('windowDragRegion', () => {
  it('is the bare attribute on macOS, where the title bar is overlaid', async () => {
    const { windowDragRegion } = await loadWith('macos')
    expect(windowDragRegion).toBe('')
  })

  it('is no attribute elsewhere, where the native title bar drags the window', async () => {
    const { windowDragRegion } = await loadWith(undefined)
    expect(windowDragRegion).toBeUndefined()
  })
})
