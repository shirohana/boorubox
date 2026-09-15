// The one definition of the query language (design D3): Rust never sees a query
// string, only the `ParsedTagSearch` this produces.

import type { ParsedTagSearch, Rating } from '@boorubox/shared'

/**
 * The `rating:` metatag in every spelling the parser accepts. Shared with
 * `toggleRatingInQuery`, which rewrites the whole metatag (design D14): a
 * second copy that drifted would silently leave a query that parses to
 * something other than what the pill shows.
 *
 * Global, so only `String.match` and `String.replace` may use it — both reset
 * `lastIndex`. An `exec` here would carry its position into the next call.
 */
const RATING_METATAG
  = /rating:([gsqe](?:,[gsqe])+|general|sensitive|questionable|explicit|[gsqe])/gi

/** `is:unrated` alone, which `toggleRatingInQuery` adds and removes by itself. */
const UNRATED_METATAG = /\bis:unrated\b/gi

/**
 * Sorts tags alphabetically (case-insensitive).
 */
export function sortTags(tags: string[]): string[] {
  return [...tags].sort((a, b) => a.toLowerCase().localeCompare(b.toLowerCase()))
}

/**
 * The tags in a space-separated field, as literal names. Not a query: every
 * token goes through as it stands, so `-foo` is a tag called `-foo` and
 * `rating:s` is the metatag the tag writer reads, never an exclusion or a
 * filter. The rewriters below read those same tokens the other way, which is
 * why the two must not be confused for one another.
 */
export function tagList(text: string): string[] {
  return text.split(/\s+/).filter((tag) => tag.length > 0)
}

/** `ParsedTagSearch` uses arrays, not Sets (D13); these members are still sets in spirit. */
function addUnique(values: string[], value: string): void {
  if (!values.includes(value)) values.push(value)
}

// Parse Danbooru-style tag search
// Supports: tags (AND), tag1 or tag2 (OR), -tag (exclude), rating:, is:, tagcount:, account:,
// collection:
export function parseTagSearch(query: string): ParsedTagSearch {
  const result: ParsedTagSearch = {
    includeTags: [],
    excludeTags: [],
    orGroups: [],
    ratings: [],
    fileTypes: [],
    tagCount: null,
    includeUnrated: false,
    accounts: [],
    excludeAccounts: [],
    collections: [],
    excludeCollections: [],
  }

  if (!query.trim()) {
    return result
  }

  let remainingQuery = query

  // 1. Extract tagcount: metatag
  // tagcount:2 (exact), tagcount:1,3 (list), tagcount:>5 (gt), tagcount:<3 (lt), tagcount:1..10
  const tagCountListRegex = /tagcount:(\d+(?:,\d+)+)/gi
  const tagCountListMatch = remainingQuery.match(tagCountListRegex)
  if (tagCountListMatch) {
    const values = tagCountListMatch[0].substring(9).split(',').map((v) => parseInt(v.trim(), 10))
    result.tagCount = { operator: 'list', values }
    remainingQuery = remainingQuery.replace(tagCountListRegex, '').trim()
  } else {
    // Design D15: every step of this parse reads and strips `remainingQuery`.
    // One `exec` gives both "did it match" and the captures, so there is no
    // second read of another string to fall out of step with the strip below.
    // `String.replace` with a /g/ regex resets `lastIndex` itself; do not add
    // another `exec` on this literal without resetting it first.
    const tagCountRegex = /tagcount:(>=|<=|>|<|)(\d+)(\.\.(\d+))?/gi
    const match = tagCountRegex.exec(remainingQuery)
    if (match) {
      const operator = match[1]
      const firstNum = parseInt(match[2], 10)
      const secondNum = match[4] ? parseInt(match[4], 10) : undefined

      if (secondNum !== undefined) {
        result.tagCount = {
          operator: 'range',
          min: Math.min(firstNum, secondNum),
          max: Math.max(firstNum, secondNum),
        }
      } else if (operator === '>') {
        result.tagCount = { operator: '>', value: firstNum }
      } else if (operator === '<') {
        result.tagCount = { operator: '<', value: firstNum }
      } else if (operator === '>=') {
        result.tagCount = { operator: '>=', value: firstNum }
      } else if (operator === '<=') {
        result.tagCount = { operator: '<=', value: firstNum }
      } else {
        result.tagCount = { operator: '=', value: firstNum }
      }
      remainingQuery = remainingQuery.replace(tagCountRegex, '').trim()
    }
  }

  // 2. Extract rating: metatags (match comma-separated list first, then single values)
  const ratingMatches = remainingQuery.match(RATING_METATAG)
  if (ratingMatches) {
    ratingMatches.forEach((match) => {
      const value = match.substring(7).toLowerCase() // Remove "rating:"
      if (value.includes(',')) {
        // First char only (g/s/q/e)
        value.split(',').forEach((r) => addUnique(result.ratings, r.trim().charAt(0)))
      } else {
        addUnique(result.ratings, value.charAt(0)) // First char only
      }
    })
    remainingQuery = remainingQuery.replace(RATING_METATAG, '').trim()
  }

  // 3. Extract is: metatags
  const isRegex = /is:(unrated|jpg|jpeg|png|webp|gif|svg)/gi
  const isMatches = remainingQuery.match(isRegex)
  if (isMatches) {
    isMatches.forEach((match) => {
      const value = match.substring(3).toLowerCase() // Remove "is:"
      if (value === 'unrated') {
        result.includeUnrated = true
      } else if (value === 'jpeg') {
        addUnique(result.fileTypes, 'image/jpeg')
      } else if (value === 'jpg') {
        addUnique(result.fileTypes, 'image/jpeg')
      } else if (value === 'png') {
        addUnique(result.fileTypes, 'image/png')
      } else if (value === 'webp') {
        addUnique(result.fileTypes, 'image/webp')
      } else if (value === 'gif') {
        addUnique(result.fileTypes, 'image/gif')
      } else if (value === 'svg') {
        addUnique(result.fileTypes, 'image/svg+xml')
      }
    })
    remainingQuery = remainingQuery.replace(isRegex, '').trim()
  }

  // 4. Extract account: metatags (support comma-separated list and exclusions)
  const accountRegex = /-?account:([a-zA-Z0-9_]+(?:,[a-zA-Z0-9_]+)*)/gi
  const accountMatches = remainingQuery.match(accountRegex)
  if (accountMatches) {
    accountMatches.forEach((match) => {
      const isExclusion = match.startsWith('-')
      const value = match.substring(isExclusion ? 9 : 8) // Remove "-account:" or "account:"
      if (value.includes(',')) {
        // Multiple accounts
        value.split(',').forEach((acc) => {
          const trimmed = acc.trim()
          if (trimmed) {
            addUnique(isExclusion ? result.excludeAccounts : result.accounts, trimmed)
          }
        })
      } else {
        // Single account
        if (value) {
          addUnique(isExclusion ? result.excludeAccounts : result.accounts, value)
        }
      }
    })
    remainingQuery = remainingQuery.replace(accountRegex, '').trim()
  }

  // 5. Extract collection: metatags (design D6 — same shape as account:, a
  // comma list on either side; the slug is lower-cased so `collection:Favorites`
  // finds `favorites`, the slug Rust computed).
  //
  // Anything but a space or a comma is part of a slug: `collections::slug`
  // only trims, lower-cases and turns whitespace into `_`, so a collection
  // named `To-upload` has the slug `to-upload` and the narrower `[a-z0-9_]`
  // this shipped as read the term the sidebar itself wrote as `collection:to`
  // — a filter that matched nothing and a row that never showed as active. A
  // space separates terms and a comma separates the list, so those two are
  // the only characters this may not take.
  const collectionRegex = /-?collection:([^\s,]+(?:,[^\s,]+)*)/gi
  const collectionMatches = remainingQuery.match(collectionRegex)
  if (collectionMatches) {
    collectionMatches.forEach((match) => {
      const isExclusion = match.startsWith('-')
      const value = match.substring(isExclusion ? 12 : 11) // Remove "-collection:" or "collection:"
      value.split(',').forEach((slug) => {
        const trimmed = slug.trim().toLowerCase()
        if (trimmed) {
          addUnique(isExclusion ? result.excludeCollections : result.collections, trimmed)
        }
      })
    })
    remainingQuery = remainingQuery.replace(collectionRegex, '').trim()
  }

  // 6. Parse tag terms (handle OR, exclusion, regular tags)
  // Split by spaces but respect "or" as operator
  const tokens = remainingQuery.split(/\s+/).filter((t) => t.length > 0)

  let i = 0
  while (i < tokens.length) {
    const token = tokens[i]

    if (token.toLowerCase() === 'or') {
      // Handle OR: take previous tag and next tag as OR group
      if (i > 0 && i < tokens.length - 1) {
        const prevTag = tokens[i - 1]
        const nextTag = tokens[i + 1]

        // Remove previous tag from includeTags if it was just added
        const prevIndex = result.includeTags.indexOf(prevTag)
        if (prevIndex !== -1) {
          result.includeTags.splice(prevIndex, 1)
        }

        // Check if previous tag is already in an OR group
        let foundGroup = false
        for (const group of result.orGroups) {
          if (group.includes(prevTag)) {
            group.push(nextTag)
            foundGroup = true
            break
          }
        }

        if (!foundGroup) {
          result.orGroups.push([prevTag, nextTag])
        }

        i += 2 // Skip 'or' and next tag
        continue
      }
    } else if (token.startsWith('-')) {
      // Exclusion
      const tag = token.substring(1)
      if (tag) {
        result.excludeTags.push(tag)
      }
    } else {
      // Regular tag (include, AND)
      result.includeTags.push(token)
    }

    i++
  }

  return result
}

/**
 * Removes a tag from search query string, cleaning up orphaned "or" operators.
 * Handles: "girl or cat" → remove "cat" → "girl"
 *          "cat or girl" → remove "cat" → "girl"
 *          "dog cat girl" → remove "cat" → "dog girl"
 */
export function removeTagFromQuery(query: string, tagToRemove: string): string {
  const tokens = query.split(/\s+/)
  const newTokens: string[] = []

  for (let i = 0; i < tokens.length; i++) {
    if (tokens[i] === tagToRemove) {
      // Skip this tag
      // Also skip "or" if it's before or after this tag
      if (i > 0 && tokens[i - 1].toLowerCase() === 'or') {
        newTokens.pop() // Remove the "or" we just added
      } else if (i < tokens.length - 1 && tokens[i + 1].toLowerCase() === 'or') {
        i++ // Skip the next "or"
      }
    } else {
      newTokens.push(tokens[i])
    }
  }

  return newTokens.join(' ').trim()
}

// Every click in the sidebar, the rating pills and the inspector rewrites the
// query string through the functions below (design D14), so the query the user
// can read is always the one that ran.

/** Collapses the runs of spaces a removal leaves behind. */
function tidy(query: string): string {
  return query.replace(/\s+/g, ' ').trim()
}

function append(query: string, term: string): string {
  const base = query.trim()
  return base ? `${base} ${term}` : term
}

function isIncluded(parsed: ParsedTagSearch, tag: string): boolean {
  return parsed.includeTags.includes(tag) || parsed.orGroups.some((group) => group.includes(tag))
}

/** `removeTagFromQuery` reads a bare token; an exclusion is a token of its own. */
function removeExclusionFromQuery(query: string, tag: string): string {
  return tidy(query.split(/\s+/).filter((token) => token !== `-${tag}`).join(' '))
}

/**
 * Adds `tag` as an included term. A tag the query excludes stops being excluded
 * rather than being asked for and refused in the same breath.
 */
export function addTagToQuery(query: string, tag: string): string {
  const parsed = parseTagSearch(query)
  if (isIncluded(parsed, tag)) return query
  const base = parsed.excludeTags.includes(tag) ? removeExclusionFromQuery(query, tag) : query
  return append(base, tag)
}

/** Adds `tag` as an exclusion, dropping the inclusion it would contradict. */
export function excludeTagFromQuery(query: string, tag: string): string {
  const parsed = parseTagSearch(query)
  if (parsed.excludeTags.includes(tag)) return query
  const base = isIncluded(parsed, tag) ? removeTagFromQuery(query, tag) : query
  return append(base, `-${tag}`)
}

/** Clicking a tag: included or excluded, it leaves; otherwise it is asked for. */
export function toggleTagInQuery(query: string, tag: string): string {
  const parsed = parseTagSearch(query)
  if (parsed.excludeTags.includes(tag)) return removeExclusionFromQuery(query, tag)
  if (isIncluded(parsed, tag)) return removeTagFromQuery(query, tag)
  return append(query, tag)
}

/**
 * Adds or removes one rating, rewriting the whole `rating:` metatag from what
 * the parser read: the metatag holds every asked-for rating in one comma list,
 * so there is nothing to edit in place. `unrated` is its own `is:` term.
 */
export function toggleRatingInQuery(query: string, rating: Rating | 'unrated'): string {
  const parsed = parseTagSearch(query)

  if (rating === 'unrated') {
    if (!parsed.includeUnrated) return append(query, 'is:unrated')
    return tidy(query.replace(UNRATED_METATAG, ''))
  }

  const kept = parsed.ratings.includes(rating)
    ? parsed.ratings.filter((value) => value !== rating)
    : [...parsed.ratings, rating]
  const withoutRatings = tidy(query.replace(RATING_METATAG, ''))
  if (kept.length === 0) return withoutRatings
  return append(withoutRatings, `rating:${[...kept].sort().join(',')}`)
}

/**
 * Rewrites one side of a comma-list metatag — `account:`/`-account:` or
 * `collection:`/`-collection:` — from `kept`, the same way
 * `toggleRatingInQuery` rewrites `rating:` (design D4, D6): the metatag holds
 * every value in one comma list, so there is nothing to edit in place.
 * `marker` is matched lowercase against whole tokens, so rewriting one list
 * leaves the other, and the other metatag, standing.
 */
function rewriteMetatagList(query: string, marker: string, kept: string[]): string {
  const base = tidy(
    query
      .split(/\s+/)
      .filter((token) => !token.toLowerCase().startsWith(marker))
      .join(' '),
  )
  if (kept.length === 0) return base
  return append(base, `${marker}${kept.join(',')}`)
}

/**
 * Adds or removes `handle` from the account list, leaving the rest of the
 * query (design D4, D6). The handle is kept exactly as given — case included —
 * because `query.rs` compares the raw path segment (design D6).
 */
export function toggleAccountInQuery(query: string, handle: string): string {
  const parsed = parseTagSearch(query)
  // A handle the query excludes stops being excluded rather than being asked
  // for and ruled out in the same breath, which matches nothing at all:
  // `addTagToQuery`'s rule for `-tag`, and what the struck-through entry in the
  // panel promises when it is clicked.
  if (parsed.excludeAccounts.includes(handle)) {
    const kept = parsed.excludeAccounts.filter((value) => value !== handle)
    return rewriteMetatagList(query, '-account:', kept)
  }
  const kept = parsed.accounts.includes(handle)
    ? parsed.accounts.filter((value) => value !== handle)
    : [...parsed.accounts, handle]
  return rewriteMetatagList(query, 'account:', kept)
}

/**
 * Adds or removes `slug` from the collection list, leaving the rest of the
 * query — `toggleAccountInQuery`'s rule for `account:` (design D4), applied
 * to `collection:` (design D6): a collection the query excludes stops being
 * excluded rather than being asked for and ruled out in the same breath.
 * `slug` is what `CollectionsSection` and the inspector pass — the id Rust
 * computed, never recomputed here (design D2) — so it is used as given, with
 * no lower-casing of its own: the parser already normalises what a typed
 * query names.
 */
export function toggleCollectionInQuery(query: string, slug: string): string {
  const parsed = parseTagSearch(query)
  if (parsed.excludeCollections.includes(slug)) {
    const kept = parsed.excludeCollections.filter((value) => value !== slug)
    return rewriteMetatagList(query, '-collection:', kept)
  }
  const kept = parsed.collections.includes(slug)
    ? parsed.collections.filter((value) => value !== slug)
    : [...parsed.collections, slug]
  return rewriteMetatagList(query, 'collection:', kept)
}

/**
 * Which tags, accounts and collections a query is currently asking for or
 * ruling out (design D3): one reader, shared by the tag sidebar, the
 * collections section and the inspector panel, so "is this term active" is
 * answered the same way everywhere it is asked.
 */
export interface ActiveTerms {
  included: Set<string>
  excluded: Set<string>
  accounts: Set<string>
  excludedAccounts: Set<string>
  collections: Set<string>
  excludedCollections: Set<string>
}

export function activeTerms(query: string): ActiveTerms {
  const parsed = parseTagSearch(query)
  return {
    included: new Set([...parsed.includeTags, ...parsed.orGroups.flat()]),
    excluded: new Set(parsed.excludeTags),
    accounts: new Set(parsed.accounts),
    excludedAccounts: new Set(parsed.excludeAccounts),
    collections: new Set(parsed.collections),
    excludedCollections: new Set(parsed.excludeCollections),
  }
}
