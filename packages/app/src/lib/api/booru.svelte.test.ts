// @vitest-environment jsdom

import type { BooruSite } from '@boorubox/shared'
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks'
import { afterEach, expect, it, vi } from 'vitest'
import { BooruSites } from './booru.svelte'

afterEach(() => {
  clearMocks()
})

const danbooru: BooruSite = {
  id: 'danbooru-donmai-us',
  name: 'Danbooru',
  baseUrl: 'https://danbooru.donmai.us',
  username: 'alice',
  createdAt: 0,
  updatedAt: 0,
}

it('reads the list once per library and answers by id', async () => {
  const calls = vi.fn()
  mockIPC((cmd) => {
    calls(cmd)
    return [danbooru]
  })

  const store = new BooruSites()
  await store.load('/libraries/one')
  await store.load('/libraries/one')

  expect(calls).toHaveBeenCalledExactlyOnceWith('booru_site_list')
  expect(store.sites).toEqual([danbooru])
  expect(store.site('danbooru-donmai-us')).toEqual(danbooru)
  expect(store.site('nothing')).toBeUndefined()
})

it('reads again for another library, and forgets the last one’s refusals', async () => {
  mockIPC(() => [danbooru])
  const store = new BooruSites()
  await store.load('/libraries/one')
  store.refused(danbooru.id, 'keychain is locked')

  await store.load('/libraries/two')

  expect(store.refusal(danbooru.id)).toBeNull()
})

it('keeps a refusal until the store answers for that site again', async () => {
  mockIPC(() => [danbooru])
  const store = new BooruSites()
  await store.load('/libraries/one')

  store.refused(danbooru.id, 'keychain is locked')
  expect(store.refusal(danbooru.id)).toBe('keychain is locked')

  // A reload is the list, not the credentials: a site found unusable stays so.
  await store.reload()
  expect(store.refusal(danbooru.id)).toBe('keychain is locked')

  store.usable(danbooru.id)
  expect(store.refusal(danbooru.id)).toBeNull()
})

it('remembers a connection test per site, and forgets a removed site', async () => {
  mockIPC(() => [danbooru])
  const store = new BooruSites()
  await store.load('/libraries/one')

  store.refused(danbooru.id, 'keychain is locked')
  store.tested(danbooru.id, { status: 'connected' })

  // The answer is the store answering, so the refusal it replaces is over.
  expect(store.test(danbooru.id)).toEqual({ status: 'connected' })
  expect(store.refusal(danbooru.id)).toBeNull()

  store.forget(danbooru.id)
  expect(store.test(danbooru.id)).toBeNull()
})

it('drops a connection test when the credential is then refused', async () => {
  mockIPC(() => [danbooru])
  const store = new BooruSites()
  await store.load('/libraries/one')

  store.tested(danbooru.id, { status: 'connected' })
  store.refused(danbooru.id, 'keychain is locked')

  expect(store.test(danbooru.id)).toBeNull()
  expect(store.refusal(danbooru.id)).toBe('keychain is locked')
})

it('forgets the last library’s test outcomes on a switch', async () => {
  mockIPC(() => [danbooru])
  const store = new BooruSites()
  await store.load('/libraries/one')
  store.tested(danbooru.id, { status: 'connected' })

  await store.load('/libraries/two')

  expect(store.test(danbooru.id)).toBeNull()
})

it('reads again after a failed read, rather than claiming the library has no sites', async () => {
  mockIPC(() => {
    throw new Error('no library is open')
  })
  const store = new BooruSites()
  await store.load('/libraries/one')

  expect(store.error).toBe('no library is open')

  mockIPC(() => [danbooru])
  await store.load('/libraries/one')

  expect(store.error).toBeNull()
  expect(store.sites).toEqual([danbooru])
})

it('reports a list that could not be read, and clears the reason on the next one', async () => {
  mockIPC(() => {
    throw new Error('no library is open')
  })
  const store = new BooruSites()
  await store.load('/libraries/one')

  expect(store.error).toBe('no library is open')
  expect(store.sites).toEqual([])

  mockIPC(() => [danbooru])
  await store.reload()

  expect(store.error).toBeNull()
  expect(store.sites).toEqual([danbooru])
})
