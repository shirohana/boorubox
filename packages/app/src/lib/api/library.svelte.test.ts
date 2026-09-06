// @vitest-environment jsdom

import { clearMocks, mockIPC } from '@tauri-apps/api/mocks'
import { afterEach, expect, it, vi } from 'vitest'
import { library } from './library.svelte'
import { status } from './status-fixture'

afterEach(() => {
  clearMocks()
})

it('asks Rust once and keeps the answer', async () => {
  const calls = vi.fn()
  mockIPC((cmd) => {
    calls(cmd)
    return status({ libraryPath: '/library' })
  })

  await library.load()
  await library.load()

  expect(calls).toHaveBeenCalledTimes(1)
  expect(library.status?.libraryPath).toBe('/library')
})

it('takes the status a command returned without asking again', () => {
  library.set(status({ imageCount: 7 }))
  expect(library.status?.imageCount).toBe(7)
  expect(library.error).toBeNull()
})
