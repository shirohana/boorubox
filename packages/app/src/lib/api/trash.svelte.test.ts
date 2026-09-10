// @vitest-environment jsdom

import { clearMocks, mockIPC } from '@tauri-apps/api/mocks'
import { afterEach, expect, it, vi } from 'vitest'
import { Trash } from './trash.svelte'

afterEach(() => {
  clearMocks()
})

it('reads the command once and publishes the number', async () => {
  const calls = vi.fn()
  mockIPC((cmd) => {
    calls(cmd)
    return 7
  })

  const trash = new Trash()
  await trash.refresh()

  expect(calls).toHaveBeenCalledExactlyOnceWith('trash_count')
  expect(trash.count).toBe(7)
})

it('keeps the last known count when the read fails', async () => {
  mockIPC(() => 7)
  const trash = new Trash()
  await trash.refresh()

  mockIPC(() => {
    throw new Error('the library is closed')
  })
  await trash.refresh()

  expect(trash.count).toBe(7)
})
