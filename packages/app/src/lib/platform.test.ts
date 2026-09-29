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

describe('isWindows', () => {
  it('is true when app.html stamped windows', async () => {
    const { isWindows } = await loadWith('windows')
    expect(isWindows).toBe(true)
  })

  it('is false on macOS and when nothing was stamped', async () => {
    expect((await loadWith('macos')).isWindows).toBe(false)
    expect((await loadWith(undefined)).isWindows).toBe(false)
  })
})
