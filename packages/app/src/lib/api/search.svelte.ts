// A paged view over the `search` command (design D12). The grid asks for the
// page its scroll window lands in, never for the whole library, so ten thousand
// images cost one page of records at a time.
//
// It sits beside the command wrappers because this is the one place the two
// search inputs of design D14 become a `SearchRequest`: the tag box goes through
// `parseTagSearch` (the only definition of the query language, D3) and the free
// text goes to Rust unparsed, for FTS5 to match.

import type { ImageRecord, SearchRequest } from '@boorubox/shared'
import { parseTagSearch } from '$lib/domain/tag-utils'
import { search } from './commands'
import { errorText } from './errors'

/** Records per `search` call. */
export const PAGE_SIZE = 200

export interface SearchInputs {
  /** Danbooru-style tag query, parsed in the webview. */
  tagQuery: string
  /** Free text over page title and URLs, matched by Rust. */
  text: string
}

export function buildSearchRequest(
  inputs: SearchInputs,
  offset: number,
  limit: number = PAGE_SIZE,
): SearchRequest {
  return {
    query: parseTagSearch(inputs.tagQuery),
    text: inputs.text.trim(),
    includeDeleted: false,
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
  inputs = $state<SearchInputs>({ tagQuery: '', text: '' })
  /** Matches for the current query, from the newest answer. */
  total = $state(0)
  /** True until the first page of the current query has answered. */
  loading = $state(true)
  error = $state<string | null>(null)
  /** Bumped by every load from scratch; a consumer re-asks for its window. */
  generation = $state(0)
  /**
   * Bumped only when the query itself changed. A refresh after a command is the
   * same list the user was looking at, so scroll position survives it; a new
   * query is a different list and does not.
   */
  queryGeneration = $state(0)

  #images = $state<(ImageRecord | undefined)[]>([])
  /* eslint-disable svelte/prefer-svelte-reactivity --
     Plain Sets on purpose. A reactive one would make the effect that asks for
     pages depend on the pages arriving, so every answer would re-run it. */
  #loaded = new Set<number>()
  #pending = new Set<number>()
  /* eslint-enable svelte/prefer-svelte-reactivity */

  at(index: number): ImageRecord | undefined {
    return this.#images[index]
  }

  /** Runs `inputs` from the top, discarding what the previous query loaded. */
  run(inputs: SearchInputs): Promise<void> {
    this.queryGeneration++
    return this.#start(inputs)
  }

  /** Re-runs the current query, for after a command changed the library. */
  refresh(): Promise<void> {
    return this.#start(this.inputs)
  }

  async #start(inputs: SearchInputs): Promise<void> {
    this.inputs = inputs
    this.#images = []
    this.#loaded.clear()
    this.#pending.clear()
    this.error = null
    this.loading = true
    const generation = ++this.generation

    await this.#loadPage(0, generation)
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

  async #loadPage(page: number, generation: number): Promise<void> {
    if (this.#loaded.has(page) || this.#pending.has(page)) return
    this.#pending.add(page)
    try {
      const result = await search(buildSearchRequest(this.inputs, page * PAGE_SIZE))
      // A query that started while this one was in flight owns the state now.
      if (generation !== this.generation) return
      this.total = result.total
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
}
