// @vitest-environment jsdom

import { clearMocks, mockConvertFileSrc, mockIPC } from '@tauri-apps/api/mocks'
import { afterEach, beforeEach, expect, it } from 'vitest'
import { imageUrl, thumbnailUrl } from './assets'

beforeEach(() => {
  mockConvertFileSrc('macos')
})

afterEach(() => {
  clearMocks()
})

it('builds the full-image URL from the library path and the record\'s file', () => {
  expect(imageUrl('/library', { file: 'images/ab/c/abc.png' }))
    .toBe(`asset://localhost/${encodeURIComponent('/library/images/ab/c/abc.png')}`)
})

it('asks Rust for the thumbnail path and converts that, with the version as a query string', async () => {
  mockIPC((cmd, args) => {
    if (cmd !== 'thumbnail_path') throw `unexpected command ${cmd}`
    return { path: `/library/.thumbs/${(args as { id: string }).id}.jpg`, version: 1_700_000_000_000 }
  })

  await expect(thumbnailUrl('abc'))
    .resolves.toBe(`asset://localhost/${encodeURIComponent('/library/.thumbs/abc.jpg')}?v=1700000000000`)
})
