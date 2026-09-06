// @vitest-environment jsdom
// mockIPC installs its handler on `window`, so these run in a DOM environment.

import type { AppSettings, SearchRequest, TagCounts } from '@boorubox/shared'
import { GRID_TILE_DEFAULT, GRID_TILE_MAX } from '@boorubox/shared'
import { img } from '$lib/domain/image-fixture'
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks'
import { afterEach, expect, it, vi } from 'vitest'
import {
  appSettings,
  closeLibrary,
  dropImageRecord,
  forgetRecent,
  imageCounts,
  importPaths,
  libraryStatus,
  openLibrary,
  pickLibrary,
  recentLibraries,
  revealLibrary,
  search,
  setGridTileSize,
  setListenerPort,
  setRating,
  setTheme,
  tagCounts,
  tagSuggestions,
  thumbnailPath,
  updateTags,
} from './commands'
import { status } from './status-fixture'

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

it('pick_library takes no arguments and returns the status', async () => {
  const calls = spyIPC(status({ libraryPath: '/picked' }))
  await expect(pickLibrary()).resolves.toEqual(status({ libraryPath: '/picked' }))
  expect(calls).toHaveBeenCalledWith('pick_library', {})
})

it('open_library passes the path', async () => {
  const calls = spyIPC(status())
  await openLibrary('/library')
  expect(calls).toHaveBeenCalledWith('open_library', { path: '/library' })
})

it('library_status decodes a missing library', async () => {
  spyIPC(status({ opened: false, libraryPath: null, missingPath: '/gone' }))
  const result = await libraryStatus()
  expect(result.opened).toBe(false)
  expect(result.missingPath).toBe('/gone')
})

it('close_library takes no arguments and returns the closed status', async () => {
  const closed = status({ opened: false, libraryPath: null })
  const calls = spyIPC(closed)
  await expect(closeLibrary()).resolves.toEqual(closed)
  expect(calls).toHaveBeenCalledWith('close_library', {})
})

it('recent_libraries takes no arguments', async () => {
  const recent = [{ path: '/library', name: 'library', available: true }]
  const calls = spyIPC(recent)
  await expect(recentLibraries()).resolves.toEqual(recent)
  expect(calls).toHaveBeenCalledWith('recent_libraries', {})
})

it('forget_recent passes the path and returns what is left', async () => {
  const calls = spyIPC([])
  await expect(forgetRecent('/gone')).resolves.toEqual([])
  expect(calls).toHaveBeenCalledWith('forget_recent', { path: '/gone' })
})

it('reveal_library takes no arguments', async () => {
  const calls = spyIPC(null)
  await revealLibrary()
  expect(calls).toHaveBeenCalledWith('reveal_library', {})
})

it('app_settings takes no arguments', async () => {
  const settings: AppSettings = { theme: 'system', gridTileSize: GRID_TILE_DEFAULT }
  const calls = spyIPC(settings)
  await expect(appSettings()).resolves.toEqual(settings)
  expect(calls).toHaveBeenCalledWith('app_settings', {})
})

it('set_theme passes the theme and returns the settings', async () => {
  const settings: AppSettings = { theme: 'dark', gridTileSize: GRID_TILE_DEFAULT }
  const calls = spyIPC(settings)
  await expect(setTheme('dark')).resolves.toEqual(settings)
  expect(calls).toHaveBeenCalledWith('set_theme', { theme: 'dark' })
})

it('set_grid_tile_size passes the size', async () => {
  const calls = spyIPC({ theme: 'system', gridTileSize: GRID_TILE_MAX })
  await setGridTileSize(10_000)
  expect(calls).toHaveBeenCalledWith('set_grid_tile_size', { size: 10_000 })
})

it('set_listener_port passes the port and returns the listener state', async () => {
  const listener = { running: false, port: 1234, error: 'address in use' }
  const calls = spyIPC(listener)
  await expect(setListenerPort(1234)).resolves.toEqual(listener)
  expect(calls).toHaveBeenCalledWith('set_listener_port', { port: 1234 })
})

const request: SearchRequest = {
  query: {
    includeTags: ['cat'],
    excludeTags: [],
    orGroups: [],
    ratings: [],
    fileTypes: [],
    tagCount: null,
    includeUnrated: false,
    accounts: [],
    excludeAccounts: [],
  },
  text: '',
  includeDeleted: false,
  sort: { field: 'captured', direction: 'desc' },
  group: 'none',
  limit: 100,
  offset: 0,
}

it('search wraps the request in a `req` argument', async () => {
  const answer = { images: [], total: 0, groups: [] }
  const calls = spyIPC(answer)
  await expect(search(request)).resolves.toEqual(answer)
  expect(calls).toHaveBeenCalledWith('search', { req: request })
})

it('image_counts takes no arguments', async () => {
  const calls = spyIPC({ total: 3, extension: 2, local: 1, legacyBundle: 0 })
  await expect(imageCounts()).resolves.toEqual({
    total: 3,
    extension: 2,
    local: 1,
    legacyBundle: 0,
  })
  expect(calls).toHaveBeenCalledWith('image_counts', {})
})

it('drop_image_record passes the id', async () => {
  const calls = spyIPC(status({ imageCount: 9 }))
  await expect(dropImageRecord('abc')).resolves.toEqual(status({ imageCount: 9 }))
  expect(calls).toHaveBeenCalledWith('drop_image_record', { id: 'abc' })
})

it('thumbnail_path passes the id and returns the absolute path', async () => {
  const calls = spyIPC('/library/.thumbs/abc.jpg')
  await expect(thumbnailPath('abc')).resolves.toBe('/library/.thumbs/abc.jpg')
  expect(calls).toHaveBeenCalledWith('thumbnail_path', { id: 'abc' })
})

it('import_paths passes the path array', async () => {
  const report = { imported: 1, skipped: 0, failed: 0, items: [] }
  const calls = spyIPC(report)
  await expect(importPaths(['/a', '/b'])).resolves.toEqual(report)
  expect(calls).toHaveBeenCalledWith('import_paths', { paths: ['/a', '/b'] })
})

it('update_tags passes the id and the whole tag set, and returns the row', async () => {
  const record = img({ id: 'abc', tags: ['cat', 'dog'] })
  const calls = spyIPC(record)
  await expect(updateTags('abc', ['cat', 'dog'])).resolves.toEqual(record)
  expect(calls).toHaveBeenCalledWith('update_tags', { id: 'abc', tags: ['cat', 'dog'] })
})

it('set_rating passes the rating, and null to clear it', async () => {
  const calls = spyIPC(img({ id: 'abc', rating: 'e' }))
  await setRating('abc', 'e')
  expect(calls).toHaveBeenCalledWith('set_rating', { id: 'abc', rating: 'e' })

  await setRating('abc', null)
  expect(calls).toHaveBeenCalledWith('set_rating', { id: 'abc', rating: null })
})

it('tag_suggestions passes the prefix and the limit', async () => {
  const suggestions = [{ name: 'cathedral', count: 4 }]
  const calls = spyIPC(suggestions)
  await expect(tagSuggestions('cat', 8)).resolves.toEqual(suggestions)
  expect(calls).toHaveBeenCalledWith('tag_suggestions', { prefix: 'cat', limit: 8 })
})

it('tag_counts wraps the same request as search in a `req` argument', async () => {
  const counts: TagCounts = {
    tags: [{ name: 'cat', count: 3 }],
    ratings: { g: 0, s: 3, q: 0, e: 1, unrated: 2 },
  }
  const calls = spyIPC(counts)
  await expect(tagCounts(request)).resolves.toEqual(counts)
  expect(calls).toHaveBeenCalledWith('tag_counts', { req: request })
})

it('a rejected command reaches the caller', async () => {
  mockIPC(() => {
    throw 'no library is open'
  })
  await expect(libraryStatus()).rejects.toBe('no library is open')
})
