// @vitest-environment jsdom

import type { Stamp } from '@boorubox/shared'
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks'
import { afterEach, expect, it, vi } from 'vitest'
import { Stamps } from './stamps.svelte'

afterEach(() => {
  clearMocks()
})

const cat: Stamp = {
  id: 's-1',
  name: 'Cat',
  text: 'cat animal',
  createdAt: 0,
  updatedAt: 0,
}

it('reads the list', async () => {
  const calls = vi.fn()
  mockIPC((cmd) => {
    calls(cmd)
    return [cat]
  })

  const store = new Stamps()
  await store.refresh()

  expect(calls).toHaveBeenCalledExactlyOnceWith('stamps_list')
  expect(store.list).toEqual([cat])
})

it('re-reads on every refresh, for a create, edit or delete', async () => {
  mockIPC(() => [cat])
  const store = new Stamps()
  await store.refresh()

  const reviewed: Stamp = { ...cat, id: 's-2', name: 'Reviewed', text: 'rating:g -tagme' }
  mockIPC(() => [cat, reviewed])
  await store.refresh()

  expect(store.list).toEqual([cat, reviewed])
})

it('reports a list that could not be read, and clears the reason on the next one', async () => {
  mockIPC(() => {
    throw new Error('no library is open')
  })
  const store = new Stamps()
  await store.refresh()

  expect(store.error).toBe('no library is open')
  expect(store.list).toEqual([])

  mockIPC(() => [cat])
  await store.refresh()

  expect(store.error).toBeNull()
  expect(store.list).toEqual([cat])
})
