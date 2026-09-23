// The stamp grammar (`stamps` design D1): the one pure reader of a stamp's
// text, in the webview because "extraction lives in the extension, policy
// lives in the app" (CLAUDE.md) puts the language here, not in Rust — Rust
// receives a `TagEditSpec` and never a string, so there is no second reader
// to drift from this one.
//
// Not `parseTagSearch`: that is the search language, and a stamp's
// `-collection:x` means "take out", the opposite of a search's "exclude" —
// merging the two readings would make one of them silently wrong for the
// other caller (`tag-utils.ts`'s own comment on why the two must stay apart).

import type { Rating, TagEditSpec } from '@boorubox/shared'

/** The four ratings a stamp may set, in the short form the grammar accepts. */
const RATING_LETTERS: Rating[] = ['g', 's', 'q', 'e']

/**
 * Search-only metatags a stamp text refuses, named in the error (design D1):
 * none of them describes a write, so none of them belongs in an edit.
 * Matched with or without a leading `-`, since excluding a search term is
 * still a search term. The five category count metatags
 * (`category-count-search` design D6) are spelled by hand rather than
 * imported from `tag-utils`'s `COUNT_METATAGS`: the search language and the
 * stamp grammar are kept apart on purpose (design D1), so a sixth count
 * metatag added there needs a row added here too.
 */
const SEARCH_ONLY_METATAGS = [
  'is:',
  'tagcount:',
  'account:',
  'posted:',
  'gentags:',
  'arttags:',
  'chartags:',
  'copytags:',
  'metatags:',
]

export type ParsedStamp = { edit: TagEditSpec } | { error: string }

function invalid(token: string): { error: string } {
  return { error: `“${token}” is not an edit.` }
}

/** `Set`-shaped push: `TagEditSpec`'s lists are arrays (design D1: duplicates dropped). */
function addUnique(values: string[], value: string): void {
  if (!values.includes(value)) values.push(value)
}

function isRating(value: string): value is Rating {
  return (RATING_LETTERS as string[]).includes(value)
}

/**
 * Parses a stamp's text into the one edit every apply writes through
 * (`stamps` design D1, D2). Tokens split on whitespace: `-tag` removes,
 * `collection:slug`/`-collection:slug` move membership (lower-cased here, and
 * that lower-casing is load-bearing: Rust matches the token against the
 * stored slug as is, and the rest of `collections::slug`'s rule — whitespace
 * to `_` — cannot arise inside one token),
 * `rating:g|s|q|e` sets the rating and the last one wins, and everything
 * else — a category prefix like `artist:name` included — is a tag to add,
 * lower-cased (`lowercase-tags` design D3, so a stamp's chip shows what Rust
 * will actually write): Rust's own `read_metatags` is still what reads the
 * prefix, once the token reaches `add`, and canonicalises it again on its own
 * side (design D1). A search-only metatag, `-rating:`, the `or` operator, a
 * token in both `add` and `remove`, or an empty text each make the whole
 * text invalid, named in the reason.
 */
export function parseStamp(text: string): ParsedStamp {
  const tokens = text.split(/\s+/).filter((token) => token.length > 0)
  if (tokens.length === 0) return { error: 'A stamp needs at least one token.' }

  const add: string[] = []
  const remove: string[] = []
  const addCollections: string[] = []
  const removeCollections: string[] = []
  let rating: Rating | undefined

  for (const token of tokens) {
    const lower = token.toLowerCase()

    if (lower === 'or') return invalid(token)

    const withoutSign = lower.startsWith('-') ? lower.slice(1) : lower
    if (SEARCH_ONLY_METATAGS.some((metatag) => withoutSign.startsWith(metatag))) {
      return invalid(token)
    }

    if (lower.startsWith('-rating:')) return invalid(token)

    if (lower.startsWith('rating:')) {
      const value = lower.slice('rating:'.length)
      if (!isRating(value)) return invalid(token)
      rating = value
      continue
    }

    if (lower.startsWith('-collection:')) {
      const slug = lower.slice('-collection:'.length)
      if (slug.length === 0) return invalid(token)
      addUnique(removeCollections, slug)
      continue
    }

    if (lower.startsWith('collection:')) {
      const slug = lower.slice('collection:'.length)
      if (slug.length === 0) return invalid(token)
      addUnique(addCollections, slug)
      continue
    }

    if (token.startsWith('-')) {
      const tag = withoutSign
      // A prefix with nothing after it means nothing, and a token that means
      // nothing is named, not dropped: dropped, `collection:` alone would save
      // as a valid stamp that applies as a no-op.
      if (tag.length === 0) return invalid(token)
      addUnique(remove, tag)
      continue
    }

    addUnique(add, lower)
  }

  const conflict = add.find((tag) => remove.includes(tag))
  if (conflict) return { error: `“${conflict}” is both added and removed.` }

  const edit: TagEditSpec = { add, remove, addCollections, removeCollections }
  if (rating !== undefined) edit.rating = rating
  return { edit }
}
