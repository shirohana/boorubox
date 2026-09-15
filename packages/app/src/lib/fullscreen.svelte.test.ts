// @vitest-environment jsdom

import { clearMocks, mockIPC, mockWindows } from '@tauri-apps/api/mocks'
import { afterEach, beforeEach, expect, it } from 'vitest'
import { fullscreen } from './fullscreen.svelte'

beforeEach(() => {
  mockWindows('main')
})

afterEach(() => {
  clearMocks()
})

it('toggle asks for the opposite of active and stores what isFullscreen answers', async () => {
  fullscreen.active = false
  let requested: boolean | undefined

  mockIPC((cmd, args) => {
    if (cmd === 'plugin:window|set_fullscreen') {
      requested = (args as { value: boolean }).value
      return null
    }
    if (cmd === 'plugin:window|is_fullscreen') return true
    throw new Error(`unexpected command ${cmd}`)
  })

  await fullscreen.toggle()

  expect(requested).toBe(true)
  expect(fullscreen.active).toBe(true)
})

it('re-reads instead of assuming: a refused request leaves active as the OS answers', async () => {
  fullscreen.active = false

  mockIPC((cmd) => {
    if (cmd === 'plugin:window|set_fullscreen') return null
    // The OS refused, so `isFullscreen` still answers false.
    if (cmd === 'plugin:window|is_fullscreen') return false
    throw new Error(`unexpected command ${cmd}`)
  })

  await fullscreen.toggle()

  expect(fullscreen.active).toBe(false)
})

it('refresh re-reads the current state without toggling it', async () => {
  fullscreen.active = false

  mockIPC((cmd) => {
    if (cmd === 'plugin:window|is_fullscreen') return true
    throw new Error(`unexpected command ${cmd}`)
  })

  await fullscreen.refresh()

  expect(fullscreen.active).toBe(true)
})
