// A paged view over the `search` command (design D12). The grid asks for the
// page its scroll window lands in, never for the whole library, so ten thousand
// images cost one page of records at a time.
//
// It sits beside the command wrappers because this is the one place the two
// search inputs of design D14 become a `SearchRequest`: the tag box goes through
// `parseTagSearch` (the only definition of the query language, D3) and the free
// text goes to Rust unparsed, for FTS5 to match. The order, the grouping and the
// sidebar's counts join it here for the same reason — they are inputs to and
// answers about the same request.

import type {
  GroupBy,
  GroupSlice,
  ImageRecord,
  Rating,
  SearchRequest,
  Sort,
  TagCounts,
} from '@boorubox/shared'
import { parseTagSearch } from '$lib/domain/tag-utils'
import { search, setRating, tagCounts, updateTags } from './commands'
import { errorText } from './errors'

/** Records per `search` call. */
export const PAGE_SIZE = 200

/** Newest capture first, ungrouped (spec `sort-and-group`). */
export const DEFAULT_SORT: Sort = { field: 'captured', direction: 'desc' }
/**
 * The trash opens on what was trashed last (`trash` design D16): a slip with
 * Backspace is then the first tile, one Restore away, without the user having
 * to remember which image it was.
 */
export const TRASH_DEFAULT_SORT: Sort = { field: 'trashed', direction: 'desc' }
export const DEFAULT_GROUP: GroupBy = 'none'

export interface SearchInputs {
  /** Danbooru-style tag query, parsed in the webview. */
  tagQuery: string
  /** Free text over page title and URLs, matched by Rust. */
  text: string
}

/**
 * How the result set is ordered and divided, and which set it is drawn from:
 * session state, never a setting. `view` is fixed for the life of the screen
 * that reads it (`trash` design D1) — a `SearchResults` searches either the
 * library or the trash and never switches between the two.
 */
export interface SearchView {
  sort: Sort
  group: GroupBy
  view: 'library' | 'trash'
}

export function buildSearchRequest(
  inputs: SearchInputs,
  view: SearchView,
  offset: number,
  limit: number = PAGE_SIZE,
): SearchRequest {
  return {
    query: parseTagSearch(inputs.tagQuery),
    text: inputs.text.trim(),
    view: view.view,
    sort: view.sort,
    group: view.group,
    limit,
    offset,
  }
}

export function pageOf(index: number): number {
  return Math.floor(index / PAGE_SIZE)
}

/**
 * The current result set: `total` rows, of which only the pages that have been
 * asked for hold records. `at()` answers `undefined` for a row whose page has
 * not arrived, which the grid renders as a placeholder.
 */
export class SearchResults {
  /**
   * Which set of images this instance searches, fixed for its lifetime
   * (`trash` design D1): `LibraryScreen` makes one `SearchResults` per route,
   * so the library and the trash never share one and never need to switch.
   */
  readonly view: 'library' | 'trash'

  inputs = $state<SearchInputs>({ tagQuery: '', text: '' })
  /** Part of every request; changing either is a new list (design D6, D7). */
  sort = $state<Sort>({ ...DEFAULT_SORT })
  group = $state<GroupBy>(DEFAULT_GROUP)
  /** Matches for the current query, from the newest answer. */
  total = $state(0)
  /** Every group of the whole result set; empty while ungrouped (design D7). */
  groups = $state<GroupSlice[]>([])
  /** What the sidebar draws; `null` until the current query has answered (D8). */
  counts = $state<TagCounts | null>(null)
  /** True until the first page of the current query has answered. */
  loading = $state(true)
  error = $state<string | null>(null)
  /** Bumped by every load from scratch; a consumer re-asks for its window. */
  generation = $state(0)
  /**
   * Bumped only when the list itself changed — a new query, a new order, a new
   * grouping. A refresh after a command is the same list the user was looking
   * at, so scroll position survives it; a different list does not.
   */
  queryGeneration = $state(0)

  #images = $state<(ImageRecord | undefined)[]>([])
  /* eslint-disable svelte/prefer-svelte-reactivity --
     Plain Sets on purpose. A reactive one would make the effect that asks for
     pages depend on the pages arriving, so every answer would re-run it. */
  #loaded = new Set<number>()
  #pending = new Set<number>()
  /* eslint-enable svelte/prefer-svelte-reactivity */

  constructor(view: 'library' | 'trash' = 'library') {
    this.view = view
    if (view === 'trash') this.sort = { ...TRASH_DEFAULT_SORT }
  }

  at(index: number): ImageRecord | undefined {
    return this.#images[index]
  }

  /** Runs `inputs` from the top, discarding what the previous query loaded. */
  run(inputs: SearchInputs): Promise<void> {
    this.queryGeneration++
    // Design D8: counts describe a query, so they go the moment it does. A
    // refresh keeps the old ones on screen until the new ones land, because it
    // is the same query.
    this.counts = null
    return this.#start(inputs)
  }

  /** Re-runs the current query, for after a command changed the library. */
  refresh(): Promise<void> {
    return this.#start(this.inputs)
  }

  /** A different order is a different list: the rows moved (design D6). */
  setSort(sort: Sort): Promise<void> {
    this.sort = sort
    this.queryGeneration++
    return this.#start(this.inputs)
  }

  /** A grouping also decides membership, not only order (design D7). */
  setGroup(group: GroupBy): Promise<void> {
    this.group = group
    this.queryGeneration++
    return this.#start(this.inputs)
  }

  /**
   * Swaps one edited record for the row it already occupies and refetches the
   * counts. The search is deliberately not re-run (design D10): an image that
   * no longer matches stays on screen until the next search, rather than
   * vanishing from under the hands of whoever is tagging it.
   */
  replace(record: ImageRecord): void {
    const index = this.#images.findIndex((image) => image?.id === record.id)
    if (index !== -1) this.#images[index] = record
    void this.#loadCounts(this.generation)
  }

  /** Writes the whole tag set of one image and redraws it (design D2, D10). */
  async saveTags(id: string, tags: string[]): Promise<ImageRecord> {
    const record = await updateTags(id, tags)
    this.replace(record)
    return record
  }

  /** Writes or clears one image's rating and redraws it (design D11). */
  async saveRating(id: string, rating: Rating | null): Promise<ImageRecord> {
    const record = await setRating(id, rating)
    this.replace(record)
    return record
  }

  async #start(inputs: SearchInputs): Promise<void> {
    this.inputs = inputs
    this.#images = []
    this.#loaded.clear()
    this.#pending.clear()
    this.error = null
    this.loading = true
    const generation = ++this.generation

    // Both halves of one run: the sidebar and the grid describe the same
    // request, so they are asked for together and superseded together.
    await Promise.all([this.#loadPage(0, generation), this.#loadCounts(generation)])
    if (generation === this.generation) this.loading = false
  }

  /** Loads every page the half-open row range `[start, end)` touches. */
  ensureRange(start: number, end: number): void {
    // Read before the early return: a caller inside a reactive effect must come
    // to depend on the generation, or a new query with the same row range would
    // never re-ask for the pages under that range.
    const generation = this.generation
    if (end <= start) return
    for (let page = pageOf(start); page <= pageOf(end - 1); page++) {
      void this.#loadPage(page, generation)
    }
  }

  get #view(): SearchView {
    return { sort: this.sort, group: this.group, view: this.view }
  }

  async #loadPage(page: number, generation: number): Promise<void> {
    if (this.#loaded.has(page) || this.#pending.has(page)) return
    this.#pending.add(page)
    try {
      const result = await search(
        buildSearchRequest(this.inputs, this.#view, page * PAGE_SIZE),
      )
      // A query that started while this one was in flight owns the state now.
      if (generation !== this.generation) return
      this.total = result.total
      this.groups = result.groups
      if (this.#images.length !== result.total) this.#images.length = result.total
      result.images.forEach((image, i) => {
        this.#images[page * PAGE_SIZE + i] = image
      })
      this.#loaded.add(page)
    } catch (error) {
      if (generation === this.generation) this.error = errorText(error)
    } finally {
      this.#pending.delete(page)
    }
  }

  /**
   * `limit` and `offset` are ignored by the command (design D8): the counts
   * describe the whole result set, not a page of it.
   *
   * A failure leaves the sidebar with nothing rather than a message of its own.
   * The count query compiles the same filter the search does, so what breaks it
   * breaks the search too, and that failure is on screen.
   */
  async #loadCounts(generation: number): Promise<void> {
    try {
      const counts = await tagCounts(buildSearchRequest(this.inputs, this.#view, 0))
      if (generation !== this.generation) return
      this.counts = counts
    } catch {
      if (generation === this.generation) this.counts = null
    }
  }
}
