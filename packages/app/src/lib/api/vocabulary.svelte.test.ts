// @vitest-environment jsdom

import type { PinnedGroup, TagEntry, Vocabulary as VocabularyAnswer } from '@boorubox/shared'
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks'
import { afterEach, expect, it, vi } from 'vitest'
import { Vocabulary } from './vocabulary.svelte'

afterEach(() => {
  clearMocks()
})

const kantoku: TagEntry = { name: 'kantoku', category: 'artist', pinnedGroup: null, note: null }
const tagme: TagEntry = { name: 'tagme', category: 'general', pinnedGroup: 1, note: null }
const zebra: TagEntry = { name: 'zebra', category: 'general', pinnedGroup: 1, note: null }

/** The answer a command gives: `tags` plus one unnamed group per number they name. */
function answer(tags: TagEntry[], groups?: PinnedGroup[]): VocabularyAnswer {
  const count = Math.max(0, ...tags.map((tag) => tag.pinnedGroup ?? 0))
  return { tags, groups: groups ?? Array.from({ length: count }, () => ({ name: '', collapsed: false })) }
}

it('reads the exceptions list', async () => {
  const calls = vi.fn()
  mockIPC((cmd) => {
    calls(cmd)
    return answer([kantoku, tagme])
  })

  const store = new Vocabulary()
  await store.refresh()

  expect(calls).toHaveBeenCalledExactlyOnceWith('tag_vocabulary')
  expect(store.entries).toEqual([kantoku, tagme])
})

it('re-reads on every refresh', async () => {
  mockIPC(() => answer([kantoku]))
  const store = new Vocabulary()
  await store.refresh()

  mockIPC(() => answer([kantoku, tagme]))
  await store.refresh()

  expect(store.entries).toEqual([kantoku, tagme])
})

it('categoryOf defaults to general for a tag outside the exceptions', async () => {
  mockIPC(() => answer([kantoku]))
  const store = new Vocabulary()
  await store.refresh()

  expect(store.categoryOf('kantoku')).toBe('artist')
  expect(store.categoryOf('bench')).toBe('general')
})

it('isPinned answers false for a tag outside the exceptions', async () => {
  mockIPC(() => answer([tagme]))
  const store = new Vocabulary()
  await store.refresh()

  expect(store.isPinned('tagme')).toBe(true)
  expect(store.isPinned('bench')).toBe(false)
})

it('pinned is the groups flattened', async () => {
  mockIPC(() => answer([zebra, kantoku, tagme]))
  const store = new Vocabulary()
  await store.refresh()

  expect(store.pinned).toEqual(store.pinnedGroups.flatMap((group) => group.tags))
  expect(store.pinned).toEqual(['tagme', 'zebra'])
})

it('pinnedGroups orders groups then category then name', async () => {
  const tagmeMeta: TagEntry = { name: 'tagme', category: 'meta', pinnedGroup: 1, note: null }
  const kantokuArtist: TagEntry = { name: 'kantoku', category: 'artist', pinnedGroup: 1, note: null }
  const girl: TagEntry = { name: '1girl', category: 'general', pinnedGroup: 2, note: null }
  const azurLane: TagEntry = { name: 'azur_lane', category: 'copyright', pinnedGroup: 1, note: null }
  mockIPC(() => answer([tagmeMeta, kantokuArtist, girl, azurLane]))

  const store = new Vocabulary()
  await store.refresh()

  expect(store.pinnedGroups).toEqual([
    { name: '', collapsed: false, tags: ['kantoku', 'azur_lane', 'tagme'] },
    { name: '', collapsed: false, tags: ['1girl'] },
  ])
  expect(store.groupCount).toBe(2)
})

it('groupOf reads the group and null for unpinned', async () => {
  mockIPC(() => answer([kantoku, tagme]))
  const store = new Vocabulary()
  await store.refresh()

  expect(store.groupOf('tagme')).toBe(1)
  expect(store.groupOf('kantoku')).toBeNull()
  expect(store.groupOf('ghost')).toBeNull()
})

it('noteOf reads the note and null for a tag without one', async () => {
  const sky: TagEntry = { name: 'sky', category: 'general', pinnedGroup: null, note: 'whole background only' }
  mockIPC(() => answer([sky, kantoku]))
  const store = new Vocabulary()
  await store.refresh()

  expect(store.noteOf('sky')).toBe('whole background only')
  expect(store.noteOf('kantoku')).toBeNull()
  expect(store.noteOf('ghost')).toBeNull()
})

it('reports a list that could not be read, and clears the reason on the next one', async () => {
  mockIPC(() => {
    throw new Error('no library is open')
  })
  const store = new Vocabulary()
  await store.refresh()

  expect(store.error).toBe('no library is open')
  expect(store.entries).toEqual([])

  mockIPC(() => answer([kantoku]))
  await store.refresh()

  expect(store.error).toBeNull()
  expect(store.entries).toEqual([kantoku])
})

it('setCategory calls the command and replaces the list with its answer', async () => {
  const calls = vi.fn()
  const next = answer([kantoku])
  mockIPC((cmd, args) => {
    calls(cmd, args)
    return next
  })

  const store = new Vocabulary()
  await store.setCategory('kantoku', 'artist')

  expect(calls).toHaveBeenCalledWith('set_tag_category', { name: 'kantoku', category: 'artist' })
  expect(store.entries).toEqual(next.tags)
})

it('place calls the command and replaces the list with its answer', async () => {
  const calls = vi.fn()
  const next = answer([tagme])
  mockIPC((cmd, args) => {
    calls(cmd, args)
    return next
  })

  const store = new Vocabulary()
  await store.place('tagme', { group: 1 })

  expect(calls).toHaveBeenCalledWith('set_tag_pinned_group', { name: 'tagme', target: { group: 1 } })
  expect(store.entries).toEqual(next.tags)
})

it('setNote calls the command and replaces the list with its answer, answering true', async () => {
  const calls = vi.fn()
  const sky: TagEntry = { name: 'sky', category: 'general', pinnedGroup: null, note: 'whole background only' }
  const next = answer([sky])
  mockIPC((cmd, args) => {
    calls(cmd, args)
    return next
  })

  const store = new Vocabulary()
  const landed = await store.setNote('sky', 'whole background only')

  expect(calls).toHaveBeenCalledWith('set_tag_note', { name: 'sky', note: 'whole background only' })
  expect(store.entries).toEqual(next.tags)
  expect(landed).toBe(true)
})

it('setNote reports a refusal, answers false, and leaves entries as they were', async () => {
  mockIPC(() => answer([kantoku]))
  const store = new Vocabulary()
  await store.refresh()

  mockIPC(() => {
    throw new Error('no such tag')
  })
  const landed = await store.setNote('ghost', 'a note')

  expect(store.error).toBe('no such tag')
  expect(store.entries).toEqual([kantoku])
  expect(landed).toBe(false)
})

it('setCategory reports a refusal instead of throwing, and leaves entries as they were', async () => {
  mockIPC(() => answer([kantoku]))
  const store = new Vocabulary()
  await store.refresh()

  mockIPC(() => {
    throw new Error('cat is a general tag and cannot become an artist tag')
  })
  await store.setCategory('cat', 'artist')

  expect(store.error).toBe('cat is a general tag and cannot become an artist tag')
  expect(store.entries).toEqual([kantoku])
})

it('place reports a refusal instead of throwing, and leaves entries as they were', async () => {
  mockIPC(() => answer([kantoku]))
  const store = new Vocabulary()
  await store.refresh()

  mockIPC(() => {
    throw new Error('no such tag')
  })
  await store.place('ghost', { group: 1 })

  expect(store.error).toBe('no such tag')
  expect(store.entries).toEqual([kantoku])
})

it('answers when the lookup is handed around detached from the store', async () => {
  mockIPC(() => answer([kantoku, tagme]))
  const store = new Vocabulary()
  await store.refresh()

  // `editorText(tags, vocabulary.categoryOf)` is how the inspector calls it.
  const { categoryOf, isPinned } = store
  expect(categoryOf('kantoku')).toBe('artist')
  expect(categoryOf('nothing')).toBe('general')
  expect(isPinned('tagme')).toBe(true)
})

it('pinnedGroups carries each group\'s name and fold, and a named empty group is a row', async () => {
  mockIPC(() => answer([tagme], [
    { name: 'Style', collapsed: true },
    { name: 'Clothes', collapsed: false },
  ]))
  const store = new Vocabulary()
  await store.refresh()

  expect(store.pinnedGroups).toEqual([
    { name: 'Style', collapsed: true, tags: ['tagme'] },
    { name: 'Clothes', collapsed: false, tags: [] },
  ])
  expect(store.groupCount).toBe(2)
  expect(store.pinned).toEqual(['tagme'])
})

it('labelOf answers the name and falls back to #n', async () => {
  mockIPC(() => answer([tagme, { ...zebra, pinnedGroup: 2 }], [
    { name: 'Style', collapsed: false },
    { name: '', collapsed: false },
  ]))
  const store = new Vocabulary()
  await store.refresh()

  const { labelOf } = store
  expect(labelOf(1)).toBe('Style')
  expect(labelOf(2)).toBe('#2')
})

it.each([
  ['placeMany', 'move_pinned_tags', (s: Vocabulary) => s.placeMany(['a', 'b'], { group: 2 }), { names: ['a', 'b'], target: { group: 2 } }],
  ['setGroupCollapsed', 'set_pinned_group_collapsed', (s: Vocabulary) => s.setGroupCollapsed(1, true), { position: 1, collapsed: true }],
  ['moveGroup', 'move_pinned_group', (s: Vocabulary) => s.moveGroup(1, 2), { from: 1, to: 2 }],
  ['deleteGroup', 'delete_pinned_group', (s: Vocabulary) => s.deleteGroup(2), { position: 2 }],
  ['renameGroup', 'rename_pinned_group', (s: Vocabulary) => s.renameGroup(1, 'Style'), { position: 1, name: 'Style' }],
  ['createGroup', 'create_pinned_group', (s: Vocabulary) => s.createGroup('Style'), { name: 'Style' }],
])('%s calls its command and replaces both arrays with the answer', async (_name, command, run, args) => {
  const calls = vi.fn()
  const next = answer([tagme], [{ name: 'Style', collapsed: true }])
  mockIPC((cmd, sent) => {
    calls(cmd, sent)
    return next
  })

  const store = new Vocabulary()
  await run(store)

  expect(calls).toHaveBeenCalledWith(command, args)
  expect(store.entries).toEqual(next.tags)
  expect(store.groups).toEqual(next.groups)
})

it('renameGroup and createGroup answer whether they landed, and keep both arrays on a refusal', async () => {
  mockIPC(() => answer([tagme]))
  const store = new Vocabulary()
  await store.refresh()

  mockIPC(() => {
    throw new Error('a group needs a name')
  })

  expect(await store.createGroup(' ')).toBe(false)
  expect(await store.renameGroup(9, 'x')).toBe(false)
  expect(store.error).toBe('a group needs a name')
  expect(store.entries).toEqual([tagme])
  expect(store.groups).toEqual([{ name: '', collapsed: false }])

  mockIPC(() => answer([tagme]))
  expect(await store.createGroup('Style')).toBe(true)
})
