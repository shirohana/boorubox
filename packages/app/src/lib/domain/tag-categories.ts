// The tag category's order and label (`tag-vocabulary` design D6): a domain
// rule — the editor's line order in `tag-input.ts`'s `editorText`, and the
// vocabulary menu's category list — kept apart from
// `components/tags/categories.ts`'s `CATEGORY_TEXT_CLASS`, which is styling,
// not a rule. Splitting them is what keeps `domain/` free of a
// `$lib/components` import: a rule about how tags group does not need to
// know what colour they are drawn in.

import type { TagCategory } from '@boorubox/shared'

/**
 * The editor's line order and the badge grouping (design D7): every
 * `TagCategory` once, fixed by design D6 rather than left to enum
 * declaration order, since the wire's own alphabetical sort would put
 * `general` first.
 */
export const CATEGORY_ORDER: TagCategory[] = ['artist', 'copyright', 'character', 'meta', 'general']

/** Every category name is already its own label; this only capitalises it. */
export function categoryLabel(category: TagCategory): string {
  return category[0].toUpperCase() + category.slice(1)
}
