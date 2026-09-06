// @vitest-environment jsdom
// The wiring between the two search inputs and the `search` command. The query
// language itself is covered by `$lib/domain/tag-parser.test.ts`; what is
// asserted here is that what the user typed arrives as the right SearchRequest.

import type { SearchRequest, SearchResult } from '@boorubox/shared'
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks'
import { img } from '$lib/domain/image-fixture'
import { afterEach, expect, it, vi } from 'vitest'
import { PAGE_SIZE, SearchResults } from './search.svelte'

afterEach(() => {
  clearMocks()
})

const empty: SearchResult = { images: [], total: 0 }

/** Records the requests that reached the IPC boundary and answers with `reply`. */
function spyIPC(reply: SearchResult | ((offset: number) => SearchResult)) {
  const requests = vi.fn()
  mockIPC((cmd, args) => {
    if (cmd !== 'search') throw `unexpected command ${cmd}`
    const req = (args as { req: { offset: number } }).req
    requests(req)
    return typeof reply === 'function' ? reply(req.offset) : reply
  })
  return requests
}

/** The one request the spy saw, for asserting on a single search. */
function onlyRequest(requests: ReturnType<typeof spyIPC>): SearchRequest {
  expect(requests).toHaveBeenCalledTimes(1)
  return requests.mock.calls[0][0] as SearchRequest
}

it('sends the first page of a bare query', async () => {
  const requests = spyIPC(empty)
  await new SearchResults().run({ tagQuery: '', text: '' })

  expect(onlyRequest(requests)).toMatchObject({
    text: '',
    includeDeleted: false,
    limit: PAGE_SIZE,
    offset: 0,
  })
})

it('sends `cat -dog` as an include and an exclude', async () => {
  const requests = spyIPC(empty)
  await new SearchResults().run({ tagQuery: 'cat -dog', text: '' })

  expect(onlyRequest(requests).query).toMatchObject({
    includeTags: ['cat'],
    excludeTags: ['dog'],
    orGroups: [],
  })
})

it('sends `cat or dog` as one OR group and no include tags', async () => {
  const requests = spyIPC(empty)
  await new SearchResults().run({ tagQuery: 'cat or dog', text: '' })

  expect(onlyRequest(requests).query).toMatchObject({
    includeTags: [],
    excludeTags: [],
    orGroups: [['cat', 'dog']],
  })
})

it('sends `rating:s,q` as two ratings and no tags', async () => {
  const requests = spyIPC(empty)
  await new SearchResults().run({ tagQuery: 'rating:s,q', text: '' })

  expect(onlyRequest(requests).query).toMatchObject({
    includeTags: [],
    ratings: ['s', 'q'],
  })
})

it('sends free text unparsed, beside an empty tag query (design D14)', async () => {
  const requests = spyIPC(empty)
  await new SearchResults().run({ tagQuery: '', text: '  cat or dog  ' })

  const req = onlyRequest(requests)
  expect(req.text).toBe('cat or dog')
  expect(req.query.includeTags).toEqual([])
  expect(req.query.orGroups).toEqual([])
})

it('reports no matches without an error', async () => {
  spyIPC(empty)
  const results = new SearchResults()
  await results.run({ tagQuery: 'nothing-matches-this', text: '' })

  expect(results.total).toBe(0)
  expect(results.loading).toBe(false)
  expect(results.error).toBeNull()
  expect(results.at(0)).toBeUndefined()
})

it('a rejected search is reported, not thrown at the caller', async () => {
  mockIPC(() => {
    throw 'no library is open'
  })
  const results = new SearchResults()
  await results.run({ tagQuery: '', text: '' })

  expect(results.error).toBe('no library is open')
  expect(results.loading).toBe(false)
})

/** A library of `total` rows whose ids are their absolute index. */
function pagedLibrary(total: number) {
  return (offset: number): SearchResult => ({
    total,
    images: Array.from(
      { length: Math.min(PAGE_SIZE, total - offset) },
      (_, i) => img({ id: `image-${offset + i}` }),
    ),
  })
}

it('keeps records at their absolute row, and asks only for the pages a window touches', async () => {
  const requests = spyIPC(pagedLibrary(10_000))
  const results = new SearchResults()
  await results.run({ tagQuery: '', text: '' })

  expect(results.total).toBe(10_000)
  expect(results.at(0)?.id).toBe('image-0')
  // The window never touched them, so no request was made for them.
  expect(results.at(PAGE_SIZE)).toBeUndefined()

  results.ensureRange(PAGE_SIZE * 3 + 10, PAGE_SIZE * 4 + 10)
  await vi.waitFor(() => expect(results.at(PAGE_SIZE * 4 + 9)).toBeDefined())

  expect(results.at(PAGE_SIZE * 3 + 10)?.id).toBe(`image-${PAGE_SIZE * 3 + 10}`)
  expect(requests.mock.calls.map(([req]) => req.offset))
    .toEqual([0, PAGE_SIZE * 3, PAGE_SIZE * 4])
})

it('does not ask again for a page it already holds', async () => {
  const requests = spyIPC(pagedLibrary(1000))
  const results = new SearchResults()
  await results.run({ tagQuery: '', text: '' })

  results.ensureRange(0, 10)
  results.ensureRange(100, 150)
  await vi.waitFor(() => expect(requests).toHaveBeenCalledTimes(1))
})

it('discards the answer to a query that was replaced while it was in flight', async () => {
  const replies: Array<(result: SearchResult) => void> = []
  mockIPC(() => new Promise<SearchResult>((resolve) => replies.push(resolve)))

  const results = new SearchResults()
  const stale = results.run({ tagQuery: 'cat', text: '' })
  const current = results.run({ tagQuery: 'dog', text: '' })

  replies[0]({ images: [img({ id: 'cat' })], total: 1 })
  replies[1]({ images: [img({ id: 'dog' })], total: 42 })
  await Promise.all([stale, current])

  expect(results.total).toBe(42)
  expect(results.at(0)?.id).toBe('dog')
  expect(results.inputs.tagQuery).toBe('dog')
})
