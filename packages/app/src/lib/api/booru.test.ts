// @vitest-environment jsdom
// mockIPC installs its handler on `window`, so these run in a DOM environment.

import type { BooruConnectionTest, BooruSite, BooruUploadForm, BooruUploadOutcome } from '@boorubox/shared'
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks'
import { afterEach, expect, it, vi } from 'vitest'
import { booruSiteDelete, booruSiteList, booruSiteSave, booruSiteTest, booruUpload } from './booru'

afterEach(() => {
  clearMocks()
})

/** Records what reached the IPC boundary and answers with `reply`. */
function spyIPC(reply: unknown) {
  const calls = vi.fn()
  mockIPC((cmd, args) => {
    calls(cmd, args)
    return reply
  })
  return calls
}

const site: BooruSite = {
  id: 'danbooru-donmai-us',
  name: 'Danbooru',
  baseUrl: 'https://danbooru.donmai.us',
  username: 'alice',
  createdAt: 1_700_000_000_000,
  updatedAt: 1_700_000_000_000,
}

it('booru_site_list takes no arguments', async () => {
  const calls = spyIPC([site])
  await expect(booruSiteList()).resolves.toEqual([site])
  expect(calls).toHaveBeenCalledWith('booru_site_list', {})
})

it('booru_site_save passes id and apiKey through as given, null included', async () => {
  const calls = spyIPC(site)

  await booruSiteSave(null, 'Danbooru', 'https://danbooru.donmai.us', 'alice', 'a-key')
  expect(calls).toHaveBeenCalledWith('booru_site_save', {
    id: null,
    name: 'Danbooru',
    baseUrl: 'https://danbooru.donmai.us',
    username: 'alice',
    apiKey: 'a-key',
  })

  await booruSiteSave('danbooru-donmai-us', 'Danbooru', 'https://danbooru.donmai.us', 'alice', null)
  expect(calls).toHaveBeenCalledWith('booru_site_save', {
    id: 'danbooru-donmai-us',
    name: 'Danbooru',
    baseUrl: 'https://danbooru.donmai.us',
    username: 'alice',
    apiKey: null,
  })
})

it('booru_site_delete passes the id', async () => {
  const calls = spyIPC(undefined)
  await booruSiteDelete('danbooru-donmai-us')
  expect(calls).toHaveBeenCalledWith('booru_site_delete', { id: 'danbooru-donmai-us' })
})

it('booru_site_test passes the id and returns the tagged outcome', async () => {
  const outcome: BooruConnectionTest = { status: 'credentialRejected' }
  const calls = spyIPC(outcome)

  await expect(booruSiteTest('danbooru-donmai-us')).resolves.toEqual(outcome)
  expect(calls).toHaveBeenCalledWith('booru_site_test', { id: 'danbooru-donmai-us' })
})

it('booru_upload passes imageId, siteId and the form, and returns the tagged outcome', async () => {
  const form: BooruUploadForm = {
    tags: ['1girl', 'blue_sky'],
    rating: 's',
    source: 'https://example.test/p',
    artist: 'pixiv_user_1',
    commentaryTitle: 'a title',
    commentaryBody: '',
  }
  const outcome: BooruUploadOutcome = {
    outcome: 'posted',
    post: { site: 'danbooru-donmai-us', remoteId: '555', postedAt: 1_700_000_000_000 },
    commentary: { status: 'applied' },
  }
  const calls = spyIPC(outcome)

  await expect(booruUpload('img-1', 'danbooru-donmai-us', form)).resolves.toEqual(outcome)
  expect(calls).toHaveBeenCalledWith('booru_upload', {
    imageId: 'img-1',
    siteId: 'danbooru-donmai-us',
    form,
  })
})
