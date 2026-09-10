// @vitest-environment jsdom
// The wiring between the two search inputs and the `search` command. The query
// language itself is covered by `$lib/domain/tag-parser.test.ts`; what is
// asserted here is that what the user typed arrives as the right SearchRequest.

import type { SearchRequest, SearchResult, TagCounts } from '@boorubox/shared'
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks'
import { img } from '$lib/domain/image-fixture'
import { afterEach, expect, it, vi } from 'vitest'
import { DEFAULT_GROUP, DEFAULT_SORT, PAGE_SIZE, SearchResults, TRASH_DEFAULT_SORT } from './search.svelte'

afterEach(() => {
  clearMocks()
})

const empty: SearchResult = { images: [], total: 0, groups: [] }

const noCounts: TagCounts = {
  tags: [],
  ratings: { g: 0, s: 0, q: 0, e: 0, unrated: 0 },
}

/**
 * Records the `search` requests that reached the IPC boundary and answers with
 * `reply`. `tag_counts` rides every run (design D8), so it is answered too — its
 * requests go to the same spy, tagged by command, for the tests that care.
 */
function spyIPC(
  reply: SearchResult | ((offset: number) => SearchResult),
  counts: TagCounts = noCounts,
) {
  const requests = vi.fn()
  const countRequests = vi.fn()
  mockIPC((cmd, args) => {
    const req = (args as { req: { offset: number } }).req
    if (cmd === 'tag_counts') {
      countRequests(req)
      return counts
    }
    if (cmd !== 'search') throw `unexpected command ${cmd}`
    requests(req)
    return typeof reply === 'function' ? reply(req.offset) : reply
  })
  return Object.assign(requests, { counts: countRequests })
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
    view: 'library',
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
    groups: [],
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
  const replies: Array<(answer: SearchResult | TagCounts) => void> = []
  mockIPC(() => new Promise((resolve) => replies.push(resolve)))

  const results = new SearchResults()
  const stale = results.run({ tagQuery: 'cat', text: '' })
  const current = results.run({ tagQuery: 'dog', text: '' })

  // Each run asks for its page and its counts (design D8), in that order.
  replies[0]({ images: [img({ id: 'cat' })], total: 1, groups: [] })
  replies[1](noCounts)
  replies[2]({ images: [img({ id: 'dog' })], total: 42, groups: [] })
  replies[3](noCounts)
  await Promise.all([stale, current])

  expect(results.total).toBe(42)
  expect(results.at(0)?.id).toBe('dog')
  expect(results.inputs.tagQuery).toBe('dog')
})

// Task 5.1 / spec `sort-and-group`: the order and the grouping are inputs to the
// same request, not a second pass over the answer.

it('defaults to newest capture first, ungrouped', async () => {
  const requests = spyIPC(empty)
  const results = new SearchResults()
  await results.run({ tagQuery: '', text: '' })

  expect(results.sort).toEqual(DEFAULT_SORT)
  expect(results.group).toBe(DEFAULT_GROUP)
  expect(onlyRequest(requests)).toMatchObject({
    sort: { field: 'captured', direction: 'desc' },
    group: 'none',
  })
})

it('the trash defaults to the last trashing first', async () => {
  const requests = spyIPC(empty)
  const results = new SearchResults('trash')
  await results.run({ tagQuery: '', text: '' })

  expect(results.sort).toEqual(TRASH_DEFAULT_SORT)
  expect(onlyRequest(requests)).toMatchObject({
    view: 'trash',
    sort: { field: 'trashed', direction: 'desc' },
  })
})

it('re-runs from the top when the sort changes, as a new list', async () => {
  const requests = spyIPC(pagedLibrary(1000))
  const results = new SearchResults()
  await results.run({ tagQuery: 'cat', text: '' })
  const query = results.queryGeneration

  await results.setSort({ field: 'size', direction: 'asc' })

  expect(results.queryGeneration).toBe(query + 1)
  const last = requests.mock.calls.at(-1)![0] as SearchRequest
  expect(last.offset).toBe(0)
  expect(last.sort).toEqual({ field: 'size', direction: 'asc' })
  expect(last.query.includeTags).toEqual(['cat'])
})

it('re-runs from the top when the grouping changes, and exposes the slices', async () => {
  const grouped: SearchResult = {
    images: [img({ id: 'a' })],
    total: 3,
    groups: [{ key: 'alice', count: 2 }, { key: 'bob', count: 1 }],
  }
  const requests = spyIPC(grouped)
  const results = new SearchResults()
  await results.run({ tagQuery: '', text: '' })
  const query = results.queryGeneration

  await results.setGroup('x-account')

  expect(results.queryGeneration).toBe(query + 1)
  expect(results.group).toBe('x-account')
  expect((requests.mock.calls.at(-1)![0] as SearchRequest).group).toBe('x-account')
  expect(results.groups).toEqual([{ key: 'alice', count: 2 }, { key: 'bob', count: 1 }])
})

// Task 4.1 / design D8: the counts and the result answer the same request.

it('asks for the counts of the query it just ran', async () => {
  const requests = spyIPC(empty, {
    tags: [{ name: 'cat', count: 3 }],
    ratings: { g: 1, s: 2, q: 0, e: 0, unrated: 4 },
  })
  const results = new SearchResults()
  await results.run({ tagQuery: 'cat', text: 'kitten' })

  expect(results.counts?.tags).toEqual([{ name: 'cat', count: 3 }])
  expect(requests.counts).toHaveBeenCalledTimes(1)
  const asked = requests.counts.mock.calls[0][0] as SearchRequest
  const searched = onlyRequest(requests)
  expect(asked.query).toEqual(searched.query)
  expect(asked.text).toBe(searched.text)
  expect(asked.sort).toEqual(searched.sort)
  expect(asked.group).toBe(searched.group)
})

it('clears the counts when the query changes', async () => {
  spyIPC(empty, { tags: [{ name: 'cat', count: 3 }], ratings: noCounts.ratings })
  const results = new SearchResults()
  await results.run({ tagQuery: 'cat', text: '' })
  expect(results.counts).not.toBeNull()

  const running = results.run({ tagQuery: 'dog', text: '' })
  expect(results.counts).toBeNull()
  await running
})

it('discards counts from a query that was superseded while they were in flight', async () => {
  const replies: Array<(answer: SearchResult | TagCounts) => void> = []
  mockIPC(() => new Promise((resolve) => replies.push(resolve)))

  const results = new SearchResults()
  const stale = results.run({ tagQuery: 'cat', text: '' })
  const current = results.run({ tagQuery: 'dog', text: '' })

  replies[2]({ images: [], total: 0, groups: [] })
  replies[3]({ tags: [{ name: 'dog', count: 9 }], ratings: noCounts.ratings })
  replies[0]({ images: [], total: 0, groups: [] })
  replies[1]({ tags: [{ name: 'cat', count: 1 }], ratings: noCounts.ratings })
  await Promise.all([stale, current])

  expect(results.counts?.tags).toEqual([{ name: 'dog', count: 9 }])
})

// Task 3.4 / design D10: an edit replaces one row and refreshes the counts.

it('replaces an edited record at its index without re-running the search', async () => {
  const requests = spyIPC(pagedLibrary(1000))
  const results = new SearchResults()
  await results.run({ tagQuery: 'cat', text: '' })
  const searches = requests.mock.calls.length

  const edited = img({ id: 'image-7', tags: ['dog'], rating: 'e' })
  results.replace(edited)

  expect(results.at(7)).toEqual(edited)
  expect(results.at(6)?.id).toBe('image-6')
  expect(results.total).toBe(1000)
  expect(results.generation).toBe(1)
  expect(results.queryGeneration).toBe(1)
  expect(requests.mock.calls.length).toBe(searches)
  await vi.waitFor(() => expect(requests.counts).toHaveBeenCalledTimes(2))
})

it('leaves an image on screen after the tag it was found by is removed', async () => {
  mockIPC((cmd, args) => {
    if (cmd === 'tag_counts') return noCounts
    if (cmd === 'update_tags') {
      return img({ id: (args as { id: string }).id, tags: (args as { tags: string[] }).tags })
    }
    return pagedLibrary(3)(0)
  })
  const results = new SearchResults()
  await results.run({ tagQuery: 'cat', text: '' })

  await results.saveTags('image-1', ['dog'])

  expect(results.at(1)?.tags).toEqual(['dog'])
  expect(results.total).toBe(3)
})

it('replaces the record a rating change returns', async () => {
  mockIPC((cmd, args) => {
    if (cmd === 'tag_counts') return noCounts
    if (cmd === 'set_rating') {
      const { id, rating } = args as { id: string, rating: 'g' | 's' | 'q' | 'e' | null }
      return img({ id, rating })
    }
    return pagedLibrary(3)(0)
  })
  const results = new SearchResults()
  await results.run({ tagQuery: '', text: '' })

  await results.saveRating('image-2', 'q')

  expect(results.at(2)?.rating).toBe('q')
})
