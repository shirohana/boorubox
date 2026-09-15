// @vitest-environment jsdom

import type { Collection } from '@boorubox/shared'
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks'
import { afterEach, expect, it, vi } from 'vitest'
import { Collections } from './collections.svelte'

afterEach(() => {
  clearMocks()
})

const favorites: Collection = {
  id: 'favorites',
  name: 'Favorites',
  slug: 'favorites',
  createdAt: 0,
  updatedAt: 0,
}

it('reads the list and answers by id', async () => {
  const calls = vi.fn()
  mockIPC((cmd) => {
    calls(cmd)
    return [favorites]
  })

  const store = new Collections()
  await store.refresh()

  expect(calls).toHaveBeenCalledExactlyOnceWith('collection_list')
  expect(store.list).toEqual([favorites])
  expect(store.byId('favorites')).toEqual(favorites)
  expect(store.byId('queue')).toBeUndefined()
})

it('re-reads on every refresh, for a create, rename or delete', async () => {
  mockIPC(() => [favorites])
  const store = new Collections()
  await store.refresh()

  const queue: Collection = { ...favorites, id: 'queue', name: 'Queue', slug: 'queue' }
  mockIPC(() => [favorites, queue])
  await store.refresh()

  expect(store.list).toEqual([favorites, queue])
})

it('reports a list that could not be read, and clears the reason on the next one', async () => {
  mockIPC(() => {
    throw new Error('no library is open')
  })
  const store = new Collections()
  await store.refresh()

  expect(store.error).toBe('no library is open')
  expect(store.list).toEqual([])

  mockIPC(() => [favorites])
  await store.refresh()

  expect(store.error).toBeNull()
  expect(store.list).toEqual([favorites])
})
