// @vitest-environment jsdom

import { clearMocks, mockConvertFileSrc, mockIPC } from '@tauri-apps/api/mocks'
import { afterEach, expect, it } from 'vitest'
import { cachedThumbnail, cacheEpoch, forgetAll, thumbnail } from './thumbnail-cache.svelte'

afterEach(() => {
  clearMocks()
})

it('asks Rust at most once per id, and answers the cached url after that', async () => {
  let calls = 0
  mockConvertFileSrc('macos')
  mockIPC((cmd, args) => {
    if (cmd === 'thumbnail_path') {
      calls++
      return { path: `/library/.thumbs/${(args as { id: string }).id}.jpg`, version: 1 }
    }
    return null
  })

  const url = await thumbnail('asked-once')
  expect(cachedThumbnail('asked-once')).toBe(url)

  await thumbnail('asked-once')
  expect(calls).toBe(1)
})

it('forgetAll bumps the epoch and drops the cache, so a second call reaches Rust again', async () => {
  let calls = 0
  mockConvertFileSrc('macos')
  mockIPC((cmd, args) => {
    if (cmd === 'thumbnail_path') {
      calls++
      return { path: `/library/.thumbs/${(args as { id: string }).id}.jpg`, version: calls }
    }
    return null
  })

  await thumbnail('after-forget')
  expect(calls).toBe(1)
  expect(cachedThumbnail('after-forget')).not.toBeNull()

  const before = cacheEpoch()
  forgetAll()

  expect(cacheEpoch()).toBe(before + 1)
  expect(cachedThumbnail('after-forget')).toBeNull()

  await thumbnail('after-forget')
  expect(calls).toBe(2)
})
