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
 * The library order: the sidebar's rows and toggles, the pinned chips and the
 * menu's category list (`single-image-tag-order` design D1/D2). Danbooru's
 * sidebar order, fixed here rather than left to enum declaration order, since
 * the wire's own alphabetical sort would put `general` first.
 */
export const CATEGORY_ORDER: TagCategory[] = ['artist', 'copyright', 'character', 'general', 'meta']

/**
 * The order of one image's tags: the inspector's tag list, the tile footer
 * and the editor's lines (`single-image-tag-order` design D1/D2). Differs from
 * {@link CATEGORY_ORDER} on purpose, by the owner's call; pick by what the
 * view shows, never by which panel it sits in.
 */
export const IMAGE_CATEGORY_ORDER: TagCategory[] = ['artist', 'copyright', 'character', 'meta', 'general']

/** Every category name is already its own label; this only capitalises it. */
export function categoryLabel(category: TagCategory): string {
  return category[0].toUpperCase() + category.slice(1)
}

/**
 * "Group by `order`, `sortTags` within" (`tag-panel-polish` design D7): the
 * one grouping every reader of the vocabulary needs — the editor's lines
 * (`tag-input.ts`'s `editorText`), the inspector's tag list, the tile footer,
 * the pinned chips and the sidebar's rows. `order` has no default
 * (`single-image-tag-order` design D3): each caller names the library order or
 * the single-image one, so a new caller cannot fall into the wrong one
 * silently. `nameOf` lets the caller group whatever it
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
  order: TagCategory[],
): { category: TagCategory, items: T[] }[] {
  return order.map((category) => {
    const inCategory = items.filter((item) => categoryOf(nameOf(item)) === category)
    const byName = new Map(inCategory.map((item) => [nameOf(item), item] as const))
    return { category, items: sortTags(inCategory.map(nameOf)).map((name) => byName.get(name)!) }
  }).filter((group) => group.items.length > 0)
}

/**
 * The sidebar's whole list order (`tag-category-visibility` design D3): the
 * rows the search is using first, then the rest — each half through
 * {@link groupByCategory} — with `hidden` applied to the second half only.
 * That placement *is* the rule "a tag the search uses stays visible, whatever
 * its category": a row `isActive` calls out is never dropped for being
 * hidden, so a term can always be taken out of the search from where it is
 * shown. Generic over the row like `groupByCategory`, so a caller passes a
 * `TagCount` (or anything else with a name) without this module importing it.
 */
export function sidebarRows<T>(
  rows: T[],
  nameOf: (row: T) => string,
  isActive: (name: string) => boolean,
  categoryOf: (name: string) => TagCategory,
  hidden: ReadonlySet<TagCategory>,
): T[] {
  const grouped = (part: T[]) =>
    groupByCategory(part, nameOf, categoryOf, CATEGORY_ORDER).flatMap((group) => group.items)
  const active = rows.filter((row) => isActive(nameOf(row)))
  const inactive = rows.filter(
    (row) => !isActive(nameOf(row)) && !hidden.has(categoryOf(nameOf(row))),
  )
  return [...grouped(active), ...grouped(inactive)]
}
