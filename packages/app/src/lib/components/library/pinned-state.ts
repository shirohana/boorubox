// The pinned chip's two pure rules (`tag-vocabulary` design D8), each written
// once here rather than once per placement: one image reads a chip by simple
// membership and writes by toggling the tag in its own set; a selection reads
// a chip by the count `selectionTagCounts` answered and writes `add`/`remove`
// against the same tri-state.

import type { TagCount } from '@boorubox/shared'

/** A pinned chip's fill, for one image or for a selection of them. */
export type FillState = 'all' | 'some' | 'none'

/**
 * How `count` of `total` reads as a chip's fill (`pinned-collections` design
 * D8) — the one rule both a tag's count and a collection's count are read
 * through, so a sixth fill state cannot appear in one and not the other.
 * `total` of 0 reads as `none`, the same fallback an empty selection's chip
 * draws as.
 */
export function fillOf(count: number, total: number): FillState {
  if (count <= 0) return 'none'
  if (count >= total) return 'all'
  return 'some'
}

/**
 * How many of `total` selected images carry `tag`, from the counts
 * `selectionTagCounts` answered — a name absent from `counts` is 0 of them
 * (`api/commands.ts`'s own doc comment on the `names` filter).
 */
export function fillState(tag: string, counts: TagCount[], total: number): FillState {
  return fillOf(counts.find((entry) => entry.name === tag)?.count ?? 0, total)
}

/**
 * `add`/`remove` for one activation over a selection (design D8): add unless
 * every selected image already carries the tag, in which case remove it from
 * all of them. Mirrors `toggledTag`'s "the opposite of what is there now" for
 * the one-image case, over a count instead of a membership test.
 */
export function toggledSelection(
  state: FillState,
  tag: string,
): { add: string[], remove: string[] } {
  return state === 'all' ? { add: [], remove: [tag] } : { add: [tag], remove: [] }
}

/** The one image's own toggle: in the set, or out of it — nothing else. */
export function toggledTag(tags: string[], tag: string): string[] {
  return tags.includes(tag) ? tags.filter((other) => other !== tag) : [...tags, tag]
}
