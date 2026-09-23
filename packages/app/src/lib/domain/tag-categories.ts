// The tag category's order and label (`tag-vocabulary` design D6): a domain
// rule — the editor's line order in `tag-input.ts`'s `editorText`, and the
// vocabulary menu's category list — kept apart from
// `components/tags/categories.ts`'s `CATEGORY_TEXT_CLASS`, which is styling,
// not a rule. Splitting them is what keeps `domain/` free of a
// `$lib/components` import: a rule about how tags group does not need to
// know what colour they are drawn in.

import type { TagCategory } from '@boorubox/shared'
import { sortTags } from './tag-utils'

/**
 * The one order the sidebar's groups, the inspector's groups and the
 * editor's lines all follow (`tag-panel-polish` design D1, reversing
 * `tag-vocabulary` D6's `meta` before `general`): Danbooru's sidebar order,
 * fixed here rather than left to enum declaration order, since the wire's
 * own alphabetical sort would put `general` first.
 */
export const CATEGORY_ORDER: TagCategory[] = ['artist', 'copyright', 'character', 'general', 'meta']

/** Every category name is already its own label; this only capitalises it. */
export function categoryLabel(category: TagCategory): string {
  return category[0].toUpperCase() + category.slice(1)
}

/**
 * "Group by `CATEGORY_ORDER`, `sortTags` within" (`tag-panel-polish` design
 * D1/D7): the one grouping every reader of the vocabulary needs — the
 * editor's lines (`tag-input.ts`'s `editorText`), the inspector's tag list
 * and the sidebar's rows all grouped the same three tags into three
 * different loops before this. `nameOf` lets the caller group whatever it
 * has a name for — a plain tag string in the editor and the inspector, a
 * `TagCount` row in the sidebar — without forcing every caller through the
 * same shape first. Ordering within a group goes through `sortTags` itself
 * (never a re-typed comparator) by sorting the names and reassembling the
 * items in that order, so this and every plain tag list agree on what
 * "sorted" means by construction. Groups with nothing in them are dropped:
 * a caller that draws one row per group never draws an empty one.
 */
export function groupByCategory<T>(
  items: T[],
  nameOf: (item: T) => string,
  categoryOf: (name: string) => TagCategory,
): { category: TagCategory, items: T[] }[] {
  return CATEGORY_ORDER.map((category) => {
    const inCategory = items.filter((item) => categoryOf(nameOf(item)) === category)
    const byName = new Map(inCategory.map((item) => [nameOf(item), item] as const))
    return { category, items: sortTags(inCategory.map(nameOf)).map((name) => byName.get(name)!) }
  }).filter((group) => group.items.length > 0)
}
