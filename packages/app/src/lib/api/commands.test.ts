// @vitest-environment jsdom
// mockIPC installs its handler on `window`, so these run in a DOM environment.

import type {
  AppSettings,
  BundlePlan,
  DeleteReport,
  ExportReport,
  Note,
  RebuildReport,
  RuleInput,
  RuleListEntry,
  RulesImportReport,
  RulesRunReport,
  SearchRequest,
  Stamp,
  StampInput,
  TagCounts,
  TagEditSpec,
  TagEntry,
} from '@boorubox/shared'
import {
  CLICK_ZOOM_CEILING_DEFAULT,
  CLICK_ZOOM_CEILING_MAX,
  GRID_TILE_DEFAULT,
  GRID_TILE_MAX,
} from '@boorubox/shared'
import { img } from '$lib/domain/image-fixture'
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks'
import { afterEach, expect, it, vi } from 'vitest'
import {
  applyEdit,
  appSettings,
  bulkSetRating,
  bundlePlan,
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
  matchingIds,
  noteGet,
  noteSet,
  openLibrary,
  pickLibrary,
  recentLibraries,
  rebuildLibrary,
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
  setClickZoomCeilingPercent,
  setCollectionsCollapsed,
  setGridTileSize,
  setListenerPort,
  setNotesCollapsed,
  setOpenLastOnLaunch,
  setRating,
  setShowTileTags,
  setTagCategory,
  setTagCategoryHidden,
  setTagPinned,
  setTheme,
  stampsDelete,
  stampsList,
  stampsUpsert,
  tagCounts,
  tagSuggestions,
  tagVocabulary,
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

it('rebuild_library passes the path and returns the report', async () => {
  const report: RebuildReport = {
    images: 25_000,
    failed: 1,
    failures: [{ file: 'images/ab/cd/abcd.json', reason: 'truncated' }],
    keptAs: 'library.sqlite.corrupt-1700000000000',
    rules: 2,
    sites: 1,
    collections: 0,
  }
  const calls = spyIPC(report)
  await expect(rebuildLibrary('/library')).resolves.toEqual(report)
  expect(calls).toHaveBeenCalledWith('rebuild_library', { path: '/library' })
})

it('app_settings takes no arguments', async () => {
  const settings: AppSettings = {
    theme: 'system',
    gridTileSize: GRID_TILE_DEFAULT,
    showTileTags: false,
    clickZoomCeilingPercent: CLICK_ZOOM_CEILING_DEFAULT,
    notesCollapsed: false,
    collectionsCollapsed: false,
    openLastOnLaunch: true,
    hiddenTagCategories: [],
  }
  const calls = spyIPC(settings)
  await expect(appSettings()).resolves.toEqual(settings)
  expect(calls).toHaveBeenCalledWith('app_settings', {})
})

it('set_theme passes the theme and returns the settings', async () => {
  const settings: AppSettings = {
    theme: 'dark',
    gridTileSize: GRID_TILE_DEFAULT,
    showTileTags: false,
    clickZoomCeilingPercent: CLICK_ZOOM_CEILING_DEFAULT,
    notesCollapsed: false,
    collectionsCollapsed: false,
    openLastOnLaunch: true,
    hiddenTagCategories: [],
  }
  const calls = spyIPC(settings)
  await expect(setTheme('dark')).resolves.toEqual(settings)
  expect(calls).toHaveBeenCalledWith('set_theme', { theme: 'dark' })
})

it('set_grid_tile_size passes the size', async () => {
  const calls = spyIPC({
    theme: 'system',
    gridTileSize: GRID_TILE_MAX,
    showTileTags: false,
    clickZoomCeilingPercent: CLICK_ZOOM_CEILING_DEFAULT,
    notesCollapsed: false,
    collectionsCollapsed: false,
    openLastOnLaunch: true,
    hiddenTagCategories: [],
  })
  await setGridTileSize(10_000)
  expect(calls).toHaveBeenCalledWith('set_grid_tile_size', { size: 10_000 })
})

it('set_show_tile_tags passes the flag and returns the settings', async () => {
  const settings: AppSettings = {
    theme: 'system',
    gridTileSize: GRID_TILE_DEFAULT,
    showTileTags: true,
    clickZoomCeilingPercent: CLICK_ZOOM_CEILING_DEFAULT,
    notesCollapsed: false,
    collectionsCollapsed: false,
    openLastOnLaunch: true,
    hiddenTagCategories: [],
  }
  const calls = spyIPC(settings)
  await expect(setShowTileTags(true)).resolves.toEqual(settings)
  expect(calls).toHaveBeenCalledWith('set_show_tile_tags', { value: true })
})

it('set_click_zoom_ceiling_percent passes the percent', async () => {
  const calls = spyIPC({
    theme: 'system',
    gridTileSize: GRID_TILE_DEFAULT,
    showTileTags: false,
    clickZoomCeilingPercent: CLICK_ZOOM_CEILING_MAX,
    notesCollapsed: false,
    collectionsCollapsed: false,
    openLastOnLaunch: true,
    hiddenTagCategories: [],
  })
  await setClickZoomCeilingPercent(10_000)
  expect(calls).toHaveBeenCalledWith('set_click_zoom_ceiling_percent', { percent: 10_000 })
})

it('set_notes_collapsed passes the flag and returns the settings', async () => {
  const settings: AppSettings = {
    theme: 'system',
    gridTileSize: GRID_TILE_DEFAULT,
    showTileTags: false,
    clickZoomCeilingPercent: CLICK_ZOOM_CEILING_DEFAULT,
    notesCollapsed: true,
    collectionsCollapsed: false,
    openLastOnLaunch: true,
    hiddenTagCategories: [],
  }
  const calls = spyIPC(settings)
  await expect(setNotesCollapsed(true)).resolves.toEqual(settings)
  expect(calls).toHaveBeenCalledWith('set_notes_collapsed', { collapsed: true })
})

it('set_collections_collapsed passes the flag and returns the settings', async () => {
  const settings: AppSettings = {
    theme: 'system',
    gridTileSize: GRID_TILE_DEFAULT,
    showTileTags: false,
    clickZoomCeilingPercent: CLICK_ZOOM_CEILING_DEFAULT,
    notesCollapsed: false,
    collectionsCollapsed: true,
    openLastOnLaunch: true,
    hiddenTagCategories: [],
  }
  const calls = spyIPC(settings)
  await expect(setCollectionsCollapsed(true)).resolves.toEqual(settings)
  expect(calls).toHaveBeenCalledWith('set_collections_collapsed', { collapsed: true })
})

it('set_tag_category_hidden passes the category and the flag and returns the settings', async () => {
  const settings: AppSettings = {
    theme: 'system',
    gridTileSize: GRID_TILE_DEFAULT,
    showTileTags: false,
    clickZoomCeilingPercent: CLICK_ZOOM_CEILING_DEFAULT,
    notesCollapsed: false,
    collectionsCollapsed: false,
    openLastOnLaunch: true,
    hiddenTagCategories: ['artist'],
  }
  const calls = spyIPC(settings)
  await expect(setTagCategoryHidden('artist', true)).resolves.toEqual(settings)
  expect(calls).toHaveBeenCalledWith('set_tag_category_hidden', { category: 'artist', hidden: true })
})

it('set_open_last_on_launch passes the flag and returns the settings', async () => {
  const settings: AppSettings = {
    theme: 'system',
    gridTileSize: GRID_TILE_DEFAULT,
    showTileTags: false,
    clickZoomCeilingPercent: CLICK_ZOOM_CEILING_DEFAULT,
    notesCollapsed: false,
    collectionsCollapsed: false,
    openLastOnLaunch: false,
    hiddenTagCategories: [],
  }
  const calls = spyIPC(settings)
  await expect(setOpenLastOnLaunch(false)).resolves.toEqual(settings)
  expect(calls).toHaveBeenCalledWith('set_open_last_on_launch', { value: false })
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
    tagCountTerms: [],
    includeUnrated: false,
    accounts: [],
    excludeAccounts: [],
    collections: [],
    excludeCollections: [],
    anyCollection: false,
    noCollection: false,
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

it('bundle_plan passes the file array and returns the plan', async () => {
  const plan: BundlePlan = {
    parts: [{ path: '/a.db', rows: 12, error: null }],
    total: 12,
  }
  const calls = spyIPC(plan)
  await expect(bundlePlan(['/a.db'])).resolves.toEqual(plan)
  expect(calls).toHaveBeenCalledWith('bundle_plan', { files: ['/a.db'] })
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
    collections: [],
    accounts: [],
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

it('matching_ids wraps the request and the candidate ids and returns the subset matched', async () => {
  const calls = spyIPC(['a'])
  await expect(matchingIds(request, ['a', 'b'])).resolves.toEqual(['a'])
  expect(calls).toHaveBeenCalledWith('matching_ids', { req: request, ids: ['a', 'b'] })
})

it('apply_edit passes the ids and the edit, and returns the written rows', async () => {
  const edit: TagEditSpec = {
    add: ['cat'],
    remove: ['dog'],
    addCollections: ['cute'],
    removeCollections: [],
    rating: 'g',
  }
  const records = [img({ id: 'a', tags: ['cat'] })]
  const calls = spyIPC(records)
  await expect(applyEdit(['a', 'b'], edit)).resolves.toEqual(records)
  expect(calls).toHaveBeenCalledWith('apply_edit', { ids: ['a', 'b'], edit })
})

it('bulk_set_rating passes the ids and the rating, and null to clear it', async () => {
  const calls = spyIPC(null)
  await bulkSetRating(['a', 'b'], 'e')
  expect(calls).toHaveBeenCalledWith('bulk_set_rating', { ids: ['a', 'b'], rating: 'e' })

  await bulkSetRating(['a', 'b'], null)
  expect(calls).toHaveBeenCalledWith('bulk_set_rating', { ids: ['a', 'b'], rating: null })
})

it('selection_tag_counts passes the ids and the limit, names omitted', async () => {
  const counts = [{ name: 'cat', count: 2 }]
  const calls = spyIPC(counts)
  await expect(selectionTagCounts(['a', 'b'], 10)).resolves.toEqual(counts)
  expect(calls).toHaveBeenCalledWith('selection_tag_counts', { ids: ['a', 'b'], limit: 10 })
})

it('selection_tag_counts passes the optional names filter when given', async () => {
  const counts = [{ name: 'tagme', count: 4 }]
  const calls = spyIPC(counts)
  await expect(selectionTagCounts(['a', 'b'], 50, ['tagme'])).resolves.toEqual(counts)
  expect(calls).toHaveBeenCalledWith('selection_tag_counts', {
    ids: ['a', 'b'],
    limit: 50,
    names: ['tagme'],
  })
})

it('tag_vocabulary takes no arguments and returns the exceptions list', async () => {
  const vocabulary: TagEntry[] = [{ name: 'kantoku', category: 'artist', pinned: false }]
  const calls = spyIPC(vocabulary)
  await expect(tagVocabulary()).resolves.toEqual(vocabulary)
  expect(calls).toHaveBeenCalledWith('tag_vocabulary', {})
})

it('set_tag_category passes the name and the category and returns the vocabulary', async () => {
  const vocabulary: TagEntry[] = [{ name: 'cat', category: 'artist', pinned: false }]
  const calls = spyIPC(vocabulary)
  await expect(setTagCategory('cat', 'artist')).resolves.toEqual(vocabulary)
  expect(calls).toHaveBeenCalledWith('set_tag_category', { name: 'cat', category: 'artist' })
})

it('set_tag_pinned passes the name and the flag and returns the vocabulary', async () => {
  const vocabulary: TagEntry[] = [{ name: 'tagme', category: 'general', pinned: true }]
  const calls = spyIPC(vocabulary)
  await expect(setTagPinned('tagme', true)).resolves.toEqual(vocabulary)
  expect(calls).toHaveBeenCalledWith('set_tag_pinned', { name: 'tagme', pinned: true })
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

const stamp: Stamp = {
  id: 's-1',
  name: 'Cat',
  text: 'cat animal',
  createdAt: 1_700_000_000_000,
  updatedAt: 1_700_000_000_000,
}

it('stamps_list takes no arguments and returns the list', async () => {
  const calls = spyIPC([stamp])
  await expect(stampsList()).resolves.toEqual([stamp])
  expect(calls).toHaveBeenCalledWith('stamps_list', {})
})

it('stamps_upsert passes the input and returns the row', async () => {
  const input: StampInput = { name: 'Cat', text: 'cat animal' }
  const calls = spyIPC(stamp)
  await expect(stampsUpsert(input)).resolves.toEqual(stamp)
  expect(calls).toHaveBeenCalledWith('stamps_upsert', { input })
})

it('stamps_delete passes the id', async () => {
  const calls = spyIPC(null)
  await stampsDelete('s-1')
  expect(calls).toHaveBeenCalledWith('stamps_delete', { id: 's-1' })
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
