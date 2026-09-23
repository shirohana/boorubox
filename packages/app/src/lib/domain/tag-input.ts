// Every rule the tag autocomplete has, as functions over `(value, caret)`
// (design D12). `TagInput.svelte` wires events and holds no rule of its own:
// this repo has no component-test harness, so anything left inside the
// component is untested by construction.

import type { TagCategory } from '@boorubox/shared'
import { parseTagSearch } from './tag-utils'
import { groupByCategory } from './tag-categories'

/** Rows the popover offers at once — the legacy viewer showed eight. */
export const SUGGESTION_LIMIT = 8

/**
 * Rows to ask the library for. More than the popover shows, because the tags
 * already in the input are dropped after the query comes back — SQL matched the
 * prefix, and only the webview can read a query string (design D12).
 */
export const SUGGESTION_FETCH = SUGGESTION_LIMIT * 3

/** A token whose first character is `-` is an exclusion (`-cat`). */
const EXCLUSION = '-'

/**
 * The nine category prefixes (`tag-vocabulary` design D4, D6): long form
 * first, Danbooru's own short forms beside it, copied from `tags.rs`'s
 * `CATEGORY_PREFIXES` rather than re-derived — nothing checks the two lists
 * against each other, so a token added to one and not the other would drift
 * silently; the tests below are what would catch it.
 */
const CATEGORY_PREFIXES = [
  'artist', 'art', 'copyright', 'copy', 'character', 'char', 'meta', 'general', 'gen',
]

/**
 * The metatags of the query language (`tag-utils`'s parser), plus the tag
 * editor's own category prefixes above. A token that has begun one is not a
 * tag being typed, so the list stays shut behind it exactly as behind a
 * `rating:`.
 */
const METATAG = new RegExp(
  `^(rating|is|tagcount|account|collection|${CATEGORY_PREFIXES.join('|')}):`,
  'i',
)

/** Value and caret after a rule rewrote the input. */
export interface TagInputText {
  value: string
  caret: number
}

/**
 * The token the caret sits in: `[start, end)` of `value`, and `typed`, the part
 * of it before the caret. A suggestion matches `typed` but replaces the whole
 * token, so a caret parked in the middle of a word still completes that word.
 */
export interface CurrentToken {
  start: number
  end: number
  typed: string
}

export function currentToken(value: string, caret: number): CurrentToken {
  const start = caret <= 0 ? 0 : value.lastIndexOf(' ', caret - 1) + 1
  const next = value.indexOf(' ', caret)
  return { start, end: next === -1 ? value.length : next, typed: value.slice(start, caret) }
}

/**
 * What to ask the library for, or `null` while the token is a metatag or the
 * `or` operator. `o` counts as the operator: it is the half of `or` that would
 * otherwise fill the list with every tag beginning in `o` between two
 * keystrokes.
 */
export function suggestionPrefix(value: string, caret: number): string | null {
  const { typed } = currentToken(value, caret)
  const lower = typed.toLowerCase()
  if (METATAG.test(typed) || lower === 'or' || lower === 'o') return null
  return typed.startsWith(EXCLUSION) ? typed.slice(1) : typed
}

/**
 * Whether a field that has just gained focus should ask for suggestions at
 * all (`TagInput`'s `onfocus`). An empty prefix lists the vocabulary's
 * most-used tags (`initialHighlight`'s own comment) — a real feature while
 * typing, through `oninput`, but not what a fresh focus asked for: the
 * inspector's `focusEnd` (design D5) lands the caret on the empty token
 * `editorText`'s trailing space leaves, and opening the editor is not a
 * request to see the library's whole vocabulary. Gating here, before
 * `suggest` is ever called, rather than dropping a completed answer after
 * the fact, is what keeps a failed or slow request from racing a later one
 * — there is no in-flight request to race in the first place. A caret
 * parked in the middle of a partly typed word still suggests: only an
 * empty prefix is gated, not `onfocus` itself.
 */
export function suggestsOnFocus(value: string, caret: number): boolean {
  const prefix = suggestionPrefix(value, caret)
  return prefix !== null && prefix.length > 0
}

/**
 * Drops the tags the input already names — including the one being typed, so an
 * exact match never offers itself — and caps what is left. SQL matched the
 * prefix; the parser is the only thing that can read a query string (design
 * D3), and it is here.
 */
export function filterSuggestions(value: string, candidates: string[]): string[] {
  const parsed = parseTagSearch(value)
  const taken = new Set(
    [...parsed.includeTags, ...parsed.excludeTags, ...parsed.orGroups.flat()]
      .map((tag) => tag.toLowerCase()),
  )
  return candidates.filter((tag) => !taken.has(tag.toLowerCase())).slice(0, SUGGESTION_LIMIT)
}

/**
 * Which row is highlighted when the list opens. Highlighting the first row is
 * what makes confirming accept a suggestion (design D13), so it happens only
 * once something has been typed — an empty prefix lists the vocabulary, and
 * confirming there must not insert whatever happens to be first.
 */
export function initialHighlight(prefix: string, count: number): number {
  if (count === 0) return -1
  return prefix.length > 0 ? 0 : -1
}

/** Arrow keys. `-1` is "nothing highlighted", which is a position, not an absence. */
export function moveHighlight(index: number, delta: number, count: number): number {
  if (count === 0) return -1
  return Math.max(-1, Math.min(index + delta, count - 1))
}

/**
 * Replaces the token the caret is in with `tag`, keeping an exclusion prefix the
 * token already had, then finishes it exactly as `completeToken` would — a
 * space after it, stepping over one already there — with the caret after that
 * space.
 *
 * The space is deliberate: an accepted tag used to leave the caret glued to the
 * word (`apple ball|`), and no suggestion list can open until the caret is past
 * a word boundary, so nothing happened again until the owner typed a space by
 * hand (design D13). A suggestion is chosen with the intent that it is a whole
 * tag, so accepting it and finishing it are the same event.
 */
export function applySuggestion(value: string, caret: number, tag: string): TagInputText {
  const { start, end, typed } = currentToken(value, caret)
  const inserted = typed.startsWith(EXCLUSION) ? `${EXCLUSION}${tag}` : tag
  const head = value.slice(0, start)
  return completeToken(head + inserted + value.slice(end), head.length + inserted.length)
}

/** True while the caret sits at the end of a word nobody has finished. */
export function isTokenIncomplete(value: string, caret: number): boolean {
  const before = value.slice(0, caret)
  if (before.length === 0 || before.endsWith(' ')) return false
  return currentToken(value, caret).typed.trim().length > 0
}

/**
 * Finishes the token under the caret with a space. A space already there is
 * stepped over rather than doubled, so accepting a suggestion in the middle of a
 * query and then confirming does not put text in the input nobody typed.
 */
export function completeToken(value: string, caret: number): TagInputText {
  if (value[caret] === ' ') return { value, caret: caret + 1 }
  return { value: `${value.slice(0, caret)} ${value.slice(caret)}`, caret: caret + 1 }
}

/**
 * The text an image's tag editor opens with (`tag-vocabulary` design D6, D7):
 * one line per category that has any tag (`groupByCategory`, `tag-panel-polish`
 * design D1/D7), and a space after the very last tag so the caret a click
 * puts at the end is already on a new token. Without it the first thing
 * typed glued itself to the last tag and the owner had to type the space by
 * hand on every edit — and a click into the editor means a tag is about to
 * be added. A save trims, so the space never counts as a change, and a line
 * break counts as one too (`tagList` already splits on any whitespace).
 * Nothing for an image with no tags: the caret is on a new token already.
 * The lines are presentation only — `categoryOf` is read once, here, and the
 * stored set stays unordered.
 */
export function editorText(tags: string[], categoryOf: (name: string) => TagCategory): string {
  if (tags.length === 0) return ''
  const lines = groupByCategory(tags, (tag) => tag, categoryOf).map((group) => group.items.join(' '))
  return `${lines.join('\n')} `
}

/** What one press of the confirm key means, in the order of design D13. */
export type Confirmation = 'accept' | 'complete' | 'submit'

export function confirmAction(value: string, caret: number, highlighted: boolean): Confirmation {
  if (highlighted) return 'accept'
  if (isTokenIncomplete(value, caret)) return 'complete'
  return 'submit'
}
