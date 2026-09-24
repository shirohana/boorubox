// @vitest-environment jsdom

import type { TagEntry } from '@boorubox/shared'
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks'
import { afterEach, expect, it, vi } from 'vitest'
import { Vocabulary } from './vocabulary.svelte'

afterEach(() => {
  clearMocks()
})

const kantoku: TagEntry = { name: 'kantoku', category: 'artist', pinnedGroup: null }
const tagme: TagEntry = { name: 'tagme', category: 'general', pinnedGroup: 1 }
const zebra: TagEntry = { name: 'zebra', category: 'general', pinnedGroup: 1 }

it('reads the exceptions list', async () => {
  const calls = vi.fn()
  mockIPC((cmd) => {
    calls(cmd)
    return [kantoku, tagme]
  })

  const store = new Vocabulary()
  await store.refresh()

  expect(calls).toHaveBeenCalledExactlyOnceWith('tag_vocabulary')
  expect(store.entries).toEqual([kantoku, tagme])
})

it('re-reads on every refresh', async () => {
  mockIPC(() => [kantoku])
  const store = new Vocabulary()
  await store.refresh()

  mockIPC(() => [kantoku, tagme])
  await store.refresh()

  expect(store.entries).toEqual([kantoku, tagme])
})

it('categoryOf defaults to general for a tag outside the exceptions', async () => {
  mockIPC(() => [kantoku])
  const store = new Vocabulary()
  await store.refresh()

  expect(store.categoryOf('kantoku')).toBe('artist')
  expect(store.categoryOf('bench')).toBe('general')
})

it('isPinned answers false for a tag outside the exceptions', async () => {
  mockIPC(() => [tagme])
  const store = new Vocabulary()
  await store.refresh()

  expect(store.isPinned('tagme')).toBe(true)
  expect(store.isPinned('bench')).toBe(false)
})

it('pinned is the groups flattened', async () => {
  mockIPC(() => [zebra, kantoku, tagme])
  const store = new Vocabulary()
  await store.refresh()

  expect(store.pinned).toEqual(store.pinnedGroups.flat())
  expect(store.pinned).toEqual(['tagme', 'zebra'])
})

it('pinnedGroups orders groups then category then name', async () => {
  const tagmeMeta: TagEntry = { name: 'tagme', category: 'meta', pinnedGroup: 1 }
  const kantokuArtist: TagEntry = { name: 'kantoku', category: 'artist', pinnedGroup: 1 }
  const girl: TagEntry = { name: '1girl', category: 'general', pinnedGroup: 2 }
  const azurLane: TagEntry = { name: 'azur_lane', category: 'copyright', pinnedGroup: 1 }
  mockIPC(() => [tagmeMeta, kantokuArtist, girl, azurLane])

  const store = new Vocabulary()
  await store.refresh()

  expect(store.pinnedGroups).toEqual([['kantoku', 'azur_lane', 'tagme'], ['1girl']])
  expect(store.groupCount).toBe(2)
})

it('groupOf reads the group and null for unpinned', async () => {
  mockIPC(() => [kantoku, tagme])
  const store = new Vocabulary()
  await store.refresh()

  expect(store.groupOf('tagme')).toBe(1)
  expect(store.groupOf('kantoku')).toBeNull()
  expect(store.groupOf('ghost')).toBeNull()
})

it('reports a list that could not be read, and clears the reason on the next one', async () => {
  mockIPC(() => {
    throw new Error('no library is open')
  })
  const store = new Vocabulary()
  await store.refresh()

  expect(store.error).toBe('no library is open')
  expect(store.entries).toEqual([])

  mockIPC(() => [kantoku])
  await store.refresh()

  expect(store.error).toBeNull()
  expect(store.entries).toEqual([kantoku])
})

it('setCategory calls the command and replaces the list with its answer', async () => {
  const calls = vi.fn()
  const next = [kantoku]
  mockIPC((cmd, args) => {
    calls(cmd, args)
    return next
  })

  const store = new Vocabulary()
  await store.setCategory('kantoku', 'artist')

  expect(calls).toHaveBeenCalledWith('set_tag_category', { name: 'kantoku', category: 'artist' })
  expect(store.entries).toEqual(next)
})

it('place calls the command and replaces the list with its answer', async () => {
  const calls = vi.fn()
  const next = [tagme]
  mockIPC((cmd, args) => {
    calls(cmd, args)
    return next
  })

  const store = new Vocabulary()
  await store.place('tagme', { group: 1 })

  expect(calls).toHaveBeenCalledWith('set_tag_pinned_group', { name: 'tagme', target: { group: 1 } })
  expect(store.entries).toEqual(next)
})

it('setCategory reports a refusal instead of throwing, and leaves entries as they were', async () => {
  mockIPC(() => [kantoku])
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
  mockIPC(() => [kantoku])
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
  mockIPC(() => [kantoku, tagme])
  const store = new Vocabulary()
  await store.refresh()

  // `editorText(tags, vocabulary.categoryOf)` is how the inspector calls it.
  const { categoryOf, isPinned } = store
  expect(categoryOf('kantoku')).toBe('artist')
  expect(categoryOf('nothing')).toBe('general')
  expect(isPinned('tagme')).toBe(true)
})
