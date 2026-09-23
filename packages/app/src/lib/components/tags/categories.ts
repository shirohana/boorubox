// Colour and search-marking for a tag's category (`tag-vocabulary` design
// D6, amended by `tag-panel-polish` D2 and D3): the inspector's tags, the
// pinned chips, the sidebar's tag list and the editor's suggestion popover
// all read `CATEGORY_TEXT_CLASS` and `SEARCH_MARK_CLASS` rather than each
// carrying its own idea of what an artist tag or an included one looks
// like. The order and the label are a domain rule, not styling, and live in
// `domain/tag-categories.ts`; re-exported here so a UI caller that wants
// both the order and the colour can still reach them from one module.

import type { TagCategory } from '@boorubox/shared'
import type { Component } from 'svelte'
import CopyrightIcon from '@lucide/svelte/icons/copyright'
import InfoIcon from '@lucide/svelte/icons/info'
import PaletteIcon from '@lucide/svelte/icons/palette'
import TagIcon from '@lucide/svelte/icons/tag'
import UserIcon from '@lucide/svelte/icons/user'

export { CATEGORY_ORDER } from '$lib/domain/tag-categories'

/**
 * Danbooru's five hues, general included (design D2): general is the
 * majority of every panel, and the owner's taste call for this pass is that
 * colouring the majority is the point, not a problem to avoid — general was
 * left uncoloured only because blue was the account chip's colour, and the
 * account entry has its own facts row now (design D4). Tailwind's own
 * steps, not the hex-precise Danbooru palette: the rest of the app is drawn
 * in Tailwind's, and the owner asked for a design for this app, not a copy
 * of Danbooru's.
 */
export const CATEGORY_TEXT_CLASS: Record<TagCategory, string> = {
  artist: 'text-red-600 dark:text-red-400',
  copyright: 'text-violet-600 dark:text-violet-400',
  character: 'text-green-600 dark:text-green-400',
  general: 'text-blue-600 dark:text-blue-400',
  meta: 'text-amber-600 dark:text-amber-300',
}

/**
 * One glyph per category (`tag-category-visibility` design D4), the second
 * cue beside the colour for a reader who cannot tell them apart: the sidebar's
 * toggle row draws each icon in `CATEGORY_TEXT_CLASS[category]`. Styling, like
 * the colours above, so it lives here rather than in `domain/tag-categories.ts`.
 */
export const CATEGORY_ICON: Record<TagCategory, Component> = {
  artist: PaletteIcon,
  copyright: CopyrightIcon,
  character: UserIcon,
  general: TagIcon,
  meta: InfoIcon,
}

/** Whether a name is in the current search, ruled out by it, or neither. */
export type SearchMark = 'included' | 'excluded' | 'none'

/**
 * One marking for "in the search", read wherever a tag or an account is
 * drawn as text (design D3, amended `tag-panel-polish` 2026-09-23 from the
 * running app): background only, on the whole hoverable box, not the text
 * itself — an underline makes English hard to read, and a coloured word
 * struck through is one colour too many once every tag carries its
 * category's colour. `included` also carries `font-medium`, the one weight
 * change left: the tint alone did not read as "this is the one that's
 * active" next to its neighbours. Each state carries its own hover shade
 * (`hover:bg-emerald-500/25`, `hover:bg-destructive/20`) — amended again,
 * 2026-09-23: a caller's neutral `hover:bg-accent` on the same element as
 * the tint outranks it at equal specificity, so hovering an active tag lost
 * its marking. Read through `searchMarkClass` below rather than indexed
 * directly, so a neutral hover is never placed on the same element as a
 * tint's own.
 */
export const SEARCH_MARK_CLASS: Record<SearchMark, string> = {
  included: 'bg-emerald-500/15 font-medium hover:bg-emerald-500/25',
  excluded: 'bg-destructive/10 hover:bg-destructive/20',
  none: '',
}

/**
 * `name`'s mark against the two sides of an `activeTerms` result — `terms.included`
 * / `terms.excluded` for a tag, `terms.accounts` / `terms.excludedAccounts` for an
 * account — so the sidebar, the inspector's tags and the inspector's account row
 * all answer "is this in the search" the same way.
 */
export function searchMark(name: string, included: Set<string>, excluded: Set<string>): SearchMark {
  if (included.has(name)) return 'included'
  if (excluded.has(name)) return 'excluded'
  return 'none'
}

/**
 * `mark`'s class, with `neutralHover` standing in only for `'none'` (design
 * D3, amended 2026-09-23): `included` and `excluded` already carry their own
 * hover shade, so a caller that concatenated its neutral hover onto them too
 * put two hover backgrounds on one element, and the neutral one — same
 * specificity, later in the class list — always won. Every caller (the
 * inspector's tag buttons and Account row, the sidebar's row, the account
 * rail's row, the collections rows) passes its own idle-state hover here
 * instead of writing it beside `SEARCH_MARK_CLASS` itself.
 */
export function searchMarkClass(mark: SearchMark, neutralHover: string): string {
  return mark === 'none' ? neutralHover : SEARCH_MARK_CLASS[mark]
}
