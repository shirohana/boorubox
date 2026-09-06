// This is NOT what filters the library. In Phase 1 the grid and the search box
// run in SQL inside Rust (src-tauri/src/query.rs); the webview only sends the
// parsed query over IPC. This module is lifted intact as the tested statement of
// the query semantics that SQL must reproduce — read it, and the tests beside
// it, when writing or changing query.rs.
//
// FIXME: one query language, two implementations — this one and the SQL in
// query.rs. They can drift with nothing failing. Either compare them under one
// suite, or delete this once query.rs is the only answer to "what matches?".

import type { ImageRecord, TagCountFilter } from '@boorubox/shared'
import { getXAccountFromUrl } from './grouping'
import { parseTagSearch } from './tag-utils'

export type View = 'all' | 'trash'

export interface ParsedSearch {
  terms: string
  tagCount: TagCountFilter | null
}

/** Parse a `tagcount:` metatag out of a free-text URL-search query. */
export function parseSearchQuery(query: string): ParsedSearch {
  // Try to match tagcount patterns in order of specificity
  // 1. List: tagcount:1,3,5
  const listRegex = /tagcount:(\d+(?:,\d+)+)/i
  const listMatch = query.match(listRegex)

  if (listMatch) {
    const values = listMatch[1].split(',').map((v) => parseInt(v.trim(), 10))
    const terms = query.replace(listRegex, '').trim()
    return {
      terms,
      tagCount: { operator: 'list', values },
    }
  }

  // 2. Range or comparison operators
  const tagCountRegex = /tagcount:(>=|<=|>|<|)(\d+)(\.\.(\d+))?/i
  const match = query.match(tagCountRegex)

  let tagCount: TagCountFilter | null = null

  if (match) {
    const operator = match[1]
    const firstNum = parseInt(match[2], 10)
    const secondNum = match[4] ? parseInt(match[4], 10) : undefined

    if (secondNum !== undefined) {
      // Range: tagcount:1..10
      tagCount = {
        operator: 'range',
        min: Math.min(firstNum, secondNum),
        max: Math.max(firstNum, secondNum),
      }
    } else if (operator === '>') {
      tagCount = { operator: '>', value: firstNum }
    } else if (operator === '<') {
      tagCount = { operator: '<', value: firstNum }
    } else if (operator === '>=') {
      tagCount = { operator: '>=', value: firstNum }
    } else if (operator === '<=') {
      tagCount = { operator: '<=', value: firstNum }
    } else {
      // Exact: tagcount:2
      tagCount = { operator: '=', value: firstNum }
    }
  }

  // Remove tagcount: from query
  const terms = query.replace(tagCountRegex, '').trim()

  return { terms, tagCount }
}

export interface FilterInputs {
  view: View
  /** Raw value of the URL/page-title search input. */
  urlSearch: string
  /** Raw value of the Danbooru-style tag search input. */
  tagSearch: string
}

function matchesTagCount(tagCount: number, filter: TagCountFilter): boolean {
  if (filter.operator === 'list') return filter.values!.includes(tagCount)
  if (filter.operator === 'range') return tagCount >= filter.min! && tagCount <= filter.max!
  if (filter.operator === '=') return tagCount === filter.value!
  if (filter.operator === '>') return tagCount > filter.value!
  if (filter.operator === '<') return tagCount < filter.value!
  if (filter.operator === '>=') return tagCount >= filter.value!
  if (filter.operator === '<=') return tagCount <= filter.value!
  return true
}

function filterByView(images: ImageRecord[], view: View): ImageRecord[] {
  return view === 'all'
    ? images.filter((img) => img.deletedAt == null)
    : images.filter((img) => img.deletedAt != null)
}

function includesQuery(value: string | null, query: string): boolean {
  return value != null && value.toLowerCase().includes(query)
}

function filterByUrlSearch(images: ImageRecord[], urlSearch: string): ImageRecord[] {
  if (!urlSearch) return images
  const query = urlSearch.toLowerCase()
  return images.filter((img) =>
    includesQuery(img.imageUrl, query)
    || includesQuery(img.pageUrl, query)
    || includesQuery(img.pageTitle, query),
  )
}

/**
 * Apply the Danbooru-style tag search.
 *
 * `includeRating` toggles the rating filter: the grid filter applies it, while
 * the rating-pill counter skips it (so pills can show counts across all ratings).
 * Every other clause is identical — this is the single source of truth that the
 * grid filter and the rating-pill counter previously duplicated.
 */
function filterByTagSearch(
  images: ImageRecord[],
  tagSearch: string,
  includeRating: boolean,
): ImageRecord[] {
  if (!tagSearch) return images
  const parsed = parseTagSearch(tagSearch)
  let filtered = images

  // Apply rating filters (skipped when computing rating-pill counts)
  if (includeRating && (parsed.ratings.length > 0 || parsed.includeUnrated)) {
    filtered = filtered.filter((img) => {
      if (parsed.includeUnrated && !img.rating) {
        return true
      }
      return img.rating != null && parsed.ratings.includes(img.rating)
    })
  }

  // Apply file type filters
  if (parsed.fileTypes.length > 0) {
    filtered = filtered.filter((img) => parsed.fileTypes.includes(img.mime))
  }

  // Apply tag count filter
  if (parsed.tagCount) {
    filtered = filtered.filter((img) => matchesTagCount(img.tags.length, parsed.tagCount!))
  }

  // Apply account filters (OR logic for included accounts)
  if (parsed.accounts.length > 0) {
    filtered = filtered.filter((img) => {
      const account = getXAccountFromUrl(img.pageUrl)
      return account != null && parsed.accounts.includes(account)
    })
  }

  // Apply excluded account filters
  if (parsed.excludeAccounts.length > 0) {
    filtered = filtered.filter((img) => {
      const account = getXAccountFromUrl(img.pageUrl)
      return account == null || !parsed.excludeAccounts.includes(account)
    })
  }

  // Apply include tags (AND logic)
  if (parsed.includeTags.length > 0) {
    filtered = filtered.filter((img) => parsed.includeTags.every((tag) => img.tags.includes(tag)))
  }

  // Apply OR groups (image must match at least one tag from each group)
  if (parsed.orGroups.length > 0) {
    filtered = filtered.filter((img) =>
      parsed.orGroups.every((group) => group.some((tag) => img.tags.includes(tag))),
    )
  }

  // Apply exclude tags
  if (parsed.excludeTags.length > 0) {
    filtered = filtered.filter((img) => !parsed.excludeTags.some((tag) => img.tags.includes(tag)))
  }

  return filtered
}

/** Full grid filter pipeline: view → URL search → tag search (with rating). */
export function filterImages(images: ImageRecord[], inputs: FilterInputs): ImageRecord[] {
  let filtered = filterByView(images, inputs.view)
  filtered = filterByUrlSearch(filtered, inputs.urlSearch)
  filtered = filterByTagSearch(filtered, inputs.tagSearch, true)
  return filtered
}

/**
 * Count images per rating after applying every filter EXCEPT the rating filter,
 * so the rating pills can show how many images each rating would yield.
 */
export function computeRatingCounts(
  images: ImageRecord[],
  inputs: FilterInputs,
): { g: number, s: number, q: number, e: number, unrated: number } {
  let filtered = filterByView(images, inputs.view)
  filtered = filterByUrlSearch(filtered, inputs.urlSearch)
  filtered = filterByTagSearch(filtered, inputs.tagSearch, false)

  const counts = { g: 0, s: 0, q: 0, e: 0, unrated: 0 }
  for (const img of filtered) {
    if (!img.rating) {
      counts.unrated++
    } else if (img.rating === 'g') {
      counts.g++
    } else if (img.rating === 's') {
      counts.s++
    } else if (img.rating === 'q') {
      counts.q++
    } else if (img.rating === 'e') {
      counts.e++
    }
  }

  return counts
}

/** Sort images in place by `field-direction` key (e.g. 'capturedAt-desc'). */
export function sortImages(images: ImageRecord[], sortKey: string): void {
  const [field, direction] = sortKey.split('-')
  const isAsc = direction === 'asc'

  images.sort((a, b) => {
    let comparison = 0

    switch (field) {
      case 'capturedAt':
        comparison = a.capturedAt - b.capturedAt
        break
      case 'updatedAt':
        comparison = a.updatedAt - b.updatedAt
        break
      case 'size':
        comparison = a.size - b.size
        break
      case 'dimensions':
        comparison = (a.width * a.height) - (b.width * b.height)
        break
      case 'url':
        comparison = (a.imageUrl ?? '').localeCompare(b.imageUrl ?? '')
        break
    }

    return isAsc ? comparison : -comparison
  })
}
