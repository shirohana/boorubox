// Colour for a tag's category (`tag-vocabulary` design D6): the inspector's
// badges, the pinned chips, the sidebar's tag list and the editor's
// suggestion popover all read `CATEGORY_TEXT_CLASS` rather than each
// carrying its own idea of what an artist tag looks like. The order and the
// label are a domain rule, not styling, and live in
// `domain/tag-categories.ts`; re-exported here so a UI caller that wants
// both the order and the colour can still reach them from one module.

import type { TagCategory } from '@boorubox/shared'

export { CATEGORY_ORDER } from '$lib/domain/tag-categories'

/**
 * Danbooru's five hues, minus blue for general (design D6): general is the
 * majority of every panel, and colouring the majority makes the four that
 * matter harder to find, not easier — blue is also the account chip's and
 * the link colour here. Tailwind's own steps, not the hex-precise Danbooru
 * palette: the rest of the app is drawn in Tailwind's, and the owner asked
 * for a design for this app, not a copy of Danbooru's.
 */
export const CATEGORY_TEXT_CLASS: Record<TagCategory, string> = {
  artist: 'text-red-600 dark:text-red-400',
  copyright: 'text-violet-600 dark:text-violet-400',
  character: 'text-green-600 dark:text-green-400',
  meta: 'text-amber-600 dark:text-amber-300',
  general: '',
}
