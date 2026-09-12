// @vitest-environment jsdom
// mockIPC installs its handler on `window`, so these run in a DOM environment.

import type {
  AppSettings,
  DeleteReport,
  ExportReport,
  Note,
  RuleInput,
  RuleListEntry,
  RulesImportReport,
  RulesRunReport,
  SearchRequest,
  TagCounts,
} from '@boorubox/shared'
import { GRID_TILE_DEFAULT, GRID_TILE_MAX } from '@boorubox/shared'
import { img } from '$lib/domain/image-fixture'
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks'
import { afterEach, expect, it, vi } from 'vitest'
import {
  appSettings,
  bulkSetRating,
  bulkUpdateTags,
  closeLibrary,
  deleteForever,
  emptyTrash,
  exportZip,
  forgetRecent,
  imageCounts,
  importBundle,
  importCancel,
  importPaths,
  importPause,
  importResume,
  libraryStatus,
  noteGet,
  noteSet,
  openLibrary,
  pickLibrary,
  recentLibraries,
  restoreImages,
  revealLibrary,
  rulesDelete,
  rulesExport,
  rulesImport,
  rulesList,
  rulesRun,
  rulesUpsert,
  search,
  searchIds,
  selectionTagCounts,
  setGridTileSize,
  setListenerPort,
  setNotesCollapsed,
  setRating,
  setTheme,
  tagCounts,
  tagSuggestions,
  thumbnailPath,
  trashCount,
  trashImages,
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
  const settings: AppSettings = {
    theme: 'system',
    gridTileSize: GRID_TILE_DEFAULT,
    notesCollapsed: false,
  }
  const calls = spyIPC(settings)
  await expect(appSettings()).resolves.toEqual(settings)
  expect(calls).toHaveBeenCalledWith('app_settings', {})
})

it('set_theme passes the theme and returns the settings', async () => {
  const settings: AppSettings = {
    theme: 'dark',
    gridTileSize: GRID_TILE_DEFAULT,
    notesCollapsed: false,
  }
  const calls = spyIPC(settings)
  await expect(setTheme('dark')).resolves.toEqual(settings)
  expect(calls).toHaveBeenCalledWith('set_theme', { theme: 'dark' })
})

it('set_grid_tile_size passes the size', async () => {
  const calls = spyIPC({ theme: 'system', gridTileSize: GRID_TILE_MAX, notesCollapsed: false })
  await setGridTileSize(10_000)
  expect(calls).toHaveBeenCalledWith('set_grid_tile_size', { size: 10_000 })
})

it('set_notes_collapsed passes the flag and returns the settings', async () => {
  const settings: AppSettings = {
    theme: 'system',
    gridTileSize: GRID_TILE_DEFAULT,
    notesCollapsed: true,
  }
  const calls = spyIPC(settings)
  await expect(setNotesCollapsed(true)).resolves.toEqual(settings)
  expect(calls).toHaveBeenCalledWith('set_notes_collapsed', { collapsed: true })
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
  view: 'library',
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

it('thumbnail_path passes the id and returns the absolute path', async () => {
  const calls = spyIPC('/library/.thumbs/abc.jpg')
  await expect(thumbnailPath('abc')).resolves.toBe('/library/.thumbs/abc.jpg')
  expect(calls).toHaveBeenCalledWith('thumbnail_path', { id: 'abc' })
})

it('import_paths passes the path array', async () => {
  const report = { imported: 1, skipped: 0, failed: 0, cancelled: false, items: [] }
  const calls = spyIPC(report)
  await expect(importPaths(['/a', '/b'])).resolves.toEqual(report)
  expect(calls).toHaveBeenCalledWith('import_paths', { paths: ['/a', '/b'] })
})

it('import_bundle passes the file array', async () => {
  const report = { imported: 1, skipped: 0, failed: 0, cancelled: false, items: [] }
  const calls = spyIPC(report)
  await expect(importBundle(['/a.db', '/b.db'])).resolves.toEqual(report)
  expect(calls).toHaveBeenCalledWith('import_bundle', { files: ['/a.db', '/b.db'] })
})

it('import_pause takes no arguments', async () => {
  const calls = spyIPC(null)
  await importPause()
  expect(calls).toHaveBeenCalledWith('import_pause', {})
})

it('import_resume takes no arguments', async () => {
  const calls = spyIPC(null)
  await importResume()
  expect(calls).toHaveBeenCalledWith('import_resume', {})
})

it('import_cancel takes no arguments', async () => {
  const calls = spyIPC(null)
  await importCancel()
  expect(calls).toHaveBeenCalledWith('import_cancel', {})
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

it('search_ids wraps the request in a `req` argument and returns just the ids', async () => {
  const calls = spyIPC(['a', 'b'])
  await expect(searchIds(request)).resolves.toEqual(['a', 'b'])
  expect(calls).toHaveBeenCalledWith('search_ids', { req: request })
})

it('bulk_update_tags passes the ids, the tags to add and the tags to remove', async () => {
  const calls = spyIPC(null)
  await bulkUpdateTags(['a', 'b'], ['cat'], ['dog'])
  expect(calls).toHaveBeenCalledWith('bulk_update_tags', {
    ids: ['a', 'b'],
    add: ['cat'],
    remove: ['dog'],
  })
})

it('bulk_set_rating passes the ids and the rating, and null to clear it', async () => {
  const calls = spyIPC(null)
  await bulkSetRating(['a', 'b'], 'e')
  expect(calls).toHaveBeenCalledWith('bulk_set_rating', { ids: ['a', 'b'], rating: 'e' })

  await bulkSetRating(['a', 'b'], null)
  expect(calls).toHaveBeenCalledWith('bulk_set_rating', { ids: ['a', 'b'], rating: null })
})

it('selection_tag_counts passes the ids and the limit', async () => {
  const counts = [{ name: 'cat', count: 2 }]
  const calls = spyIPC(counts)
  await expect(selectionTagCounts(['a', 'b'], 10)).resolves.toEqual(counts)
  expect(calls).toHaveBeenCalledWith('selection_tag_counts', { ids: ['a', 'b'], limit: 10 })
})

it('export_zip passes the ids, the path and this zone\'s offset, and returns the report', async () => {
  const report: ExportReport = { path: '/out.zip', written: 2, missing: [] }
  const calls = spyIPC(report)
  await expect(exportZip(['a', 'b'], '/out.zip')).resolves.toEqual(report)
  expect(calls).toHaveBeenCalledWith('export_zip', {
    ids: ['a', 'b'],
    path: '/out.zip',
    utcOffsetMinutes: -new Date().getTimezoneOffset(),
  })
})

it('trash_images passes the ids', async () => {
  const calls = spyIPC(null)
  await trashImages(['a', 'b'])
  expect(calls).toHaveBeenCalledWith('trash_images', { ids: ['a', 'b'] })
})

it('restore_images passes the ids', async () => {
  const calls = spyIPC(null)
  await restoreImages(['a', 'b'])
  expect(calls).toHaveBeenCalledWith('restore_images', { ids: ['a', 'b'] })
})

it('delete_forever passes the ids and returns the report', async () => {
  const report: DeleteReport = { deleted: 2, filesLeft: [] }
  const calls = spyIPC(report)
  await expect(deleteForever(['a', 'b'])).resolves.toEqual(report)
  expect(calls).toHaveBeenCalledWith('delete_forever', { ids: ['a', 'b'] })
})

it('empty_trash takes no arguments and returns the report', async () => {
  const report: DeleteReport = { deleted: 3, filesLeft: ['/library/images/a.png'] }
  const calls = spyIPC(report)
  await expect(emptyTrash()).resolves.toEqual(report)
  expect(calls).toHaveBeenCalledWith('empty_trash', {})
})

it('trash_count takes no arguments and returns the count', async () => {
  const calls = spyIPC(4)
  await expect(trashCount()).resolves.toBe(4)
  expect(calls).toHaveBeenCalledWith('trash_count', {})
})

const ruleEntry: RuleListEntry = {
  rule: {
    id: 'r-1',
    name: 'pixiv',
    pattern: 'pixiv',
    isRegex: false,
    tags: ['pixiv'],
    enabled: true,
    createdAt: 1_700_000_000_000,
    updatedAt: 1_700_000_000_000,
  },
  patternError: null,
}

it('rules_list takes no arguments and returns the list', async () => {
  const calls = spyIPC([ruleEntry])
  await expect(rulesList()).resolves.toEqual([ruleEntry])
  expect(calls).toHaveBeenCalledWith('rules_list', {})
})

it('rules_upsert passes the rule and returns the row', async () => {
  const input: RuleInput = {
    name: 'pixiv',
    pattern: 'pixiv',
    isRegex: false,
    tags: ['pixiv'],
    enabled: true,
  }
  const calls = spyIPC(ruleEntry.rule)
  await expect(rulesUpsert(input)).resolves.toEqual(ruleEntry.rule)
  expect(calls).toHaveBeenCalledWith('rules_upsert', { rule: input })
})

it('rules_delete passes the id', async () => {
  const calls = spyIPC(null)
  await rulesDelete('r-1')
  expect(calls).toHaveBeenCalledWith('rules_delete', { id: 'r-1' })
})

it('rules_run takes no arguments and returns the report', async () => {
  const report: RulesRunReport = { examined: 400, changed: 12, rules: [], invalid: [] }
  const calls = spyIPC(report)
  await expect(rulesRun()).resolves.toEqual(report)
  expect(calls).toHaveBeenCalledWith('rules_run', {})
})

it('rules_export passes the path', async () => {
  const calls = spyIPC(null)
  await rulesExport('/library/rules.json')
  expect(calls).toHaveBeenCalledWith('rules_export', { path: '/library/rules.json' })
})

it('rules_import passes the path and returns the report', async () => {
  const report: RulesImportReport = { imported: 2, skipped: 1 }
  const calls = spyIPC(report)
  await expect(rulesImport('/library/rules.json')).resolves.toEqual(report)
  expect(calls).toHaveBeenCalledWith('rules_import', { path: '/library/rules.json' })
})

it('note_get takes no arguments and returns the note', async () => {
  const note: Note = { content: 'still to sort', updatedAt: 1_700_000_000_000 }
  const calls = spyIPC(note)
  await expect(noteGet()).resolves.toEqual(note)
  expect(calls).toHaveBeenCalledWith('note_get', {})
})

it('note_set passes the content and returns the note as stored', async () => {
  const note: Note = { content: 'still to sort', updatedAt: 1_700_000_000_000 }
  const calls = spyIPC(note)
  await expect(noteSet('still to sort')).resolves.toEqual(note)
  expect(calls).toHaveBeenCalledWith('note_set', { content: 'still to sort' })
})

it('a rejected command reaches the caller', async () => {
  mockIPC(() => {
    throw 'no library is open'
  })
  await expect(libraryStatus()).rejects.toBe('no library is open')
})
