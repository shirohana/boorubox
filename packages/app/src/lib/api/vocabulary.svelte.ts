// The library's tag vocabulary, for every place that reads "what category is
// this tag, and is it pinned" (`tag-vocabulary` design D5): the inspector's
// tags, the pinned chips, the sidebar's tag list and the editor's
// suggestion popover. One store, the shape of `collections.svelte.ts`, so
// those readers cannot disagree about a tag's colour a moment after its
// category changed.
//
// `entries` holds the exceptions only (design D2) — a general, unpinned tag
// with no note is never in it, so this is hundreds of names in a library of
// thousands of tags — and `#byName` is the Map that makes a lookup cheap
// over that list; `$derived` rebuilds it only when a refresh or a setter
// replaces `entries` wholesale, never once per `categoryOf` call from a
// panel full of badges.
//
// No `setCategory`/`place`/`setNote` write of its own kind to read back: each
// setter's command already answers the whole vocabulary, so the setters
// below replace `entries` and `groups` together with that answer, the
// cheapest way to keep the two in step — a compaction can renumber a group,
// and a name read apart from the tags would sit on the wrong row.

import type { PinnedGroup, PinTarget, TagCategory, TagEntry, Vocabulary as VocabularyAnswer } from '@boorubox/shared'
import { CATEGORY_ORDER, groupByCategory } from '$lib/domain/tag-categories'
import {
  createPinnedGroup,
  deletePinnedGroup,
  movePinnedGroup,
  movePinnedTags,
  renamePinnedGroup,
  setPinnedGroupCollapsed,
  setTagCategory,
  setTagNote,
  setTagPinnedGroup,
  tagVocabulary,
} from './commands'
import { errorText } from './errors'

/** One pinned group as the strip and the dialog draw it (`pinned-group-management` design D7). */
export interface PinnedGroupRow extends PinnedGroup {
  /** The group's tags in the sidebar's order, by name. */
  tags: string[]
}

export class Vocabulary {
  /** Every tag that is not `(general, unpinned)`, as Rust answers. */
  entries = $state<TagEntry[]>([])
  /** Every pinned group's name and fold; `groups[i]` is position `i + 1`. */
  groups = $state<PinnedGroup[]>([])
  /** Why the list could not be read; `null` while it is in step. */
  error = $state<string | null>(null)

  /* eslint-disable-next-line svelte/prefer-svelte-reactivity --
     A plain Map on purpose: `$derived` already rebuilds it wholesale on every
     `entries` change, so a `SvelteMap`'s own key-by-key reactivity would
     track nothing this doesn't already recompute (`search.svelte.ts`'s
     `#loaded`/`#pending` Sets read the same way). */
  #byName = $derived(new Map(this.entries.map((entry) => [entry.name, entry] as const)))

  /**
   * A tag outside the exceptions list is general — design D2's own default.
   * Arrow properties, not methods: both are handed around as values
   * (`editorText(tags, vocabulary.categoryOf)`), and a method detached from
   * the store reads `this.#byName` off `undefined` — which the tests never
   * saw, because they pass their own arrow, and the app saw the moment an
   * image with tags reached the editor. Declared before {@link pinned},
   * which reads it: a class field's initializer runs in declaration order.
   */
  categoryOf = (name: string): TagCategory =>
    this.#byName.get(name)?.category ?? 'general'

  /** `null` for a tag outside the exceptions list, which is never pinned. */
  groupOf = (name: string): number | null => this.#byName.get(name)?.pinnedGroup ?? null

  isPinned = (name: string): boolean => this.groupOf(name) !== null

  /** `null` for a tag outside the exceptions list, which carries no note. */
  noteOf = (name: string): string | null => this.#byName.get(name)?.note ?? null

  /**
   * Every pinned group in position order, each group's tags in the sidebar's
   * own order — `CATEGORY_ORDER` first, alphabetical within a category
   * (`pinned-collections` design D7, `pinned-tag-groups` design D4) — with
   * the group's name and fold (`pinned-group-management` design D7). Made
   * here, once, because the inspector's two placements both draw the pinned
   * rows from this list rather than sorting their own: a category changing
   * cannot leave one placement's chip row out of step with the other's.
   * `groupByCategory` is the sidebar's own grouping (`domain/tag-categories.ts`),
   * not a second copy of it. A named group with no tag is a row with `tags: []`.
   */
  pinnedGroups: PinnedGroupRow[] = $derived.by(() =>
    this.groups.map((group, index) => ({
      ...group,
      tags: groupByCategory(
        this.entries.filter((entry) => entry.pinnedGroup === index + 1),
        (entry) => entry.name,
        this.categoryOf,
        CATEGORY_ORDER,
      ).flatMap((byCategory) => byCategory.items.map((entry) => entry.name)),
    })),
  )

  /** The group's name, or `#n` when it has none: the one spelling the menu and the strip share. */
  labelOf = (position: number): string => this.groups[position - 1]?.name || `#${position}`

  /** The label of the group a tag is pinned in, `null` for a tag that is not pinned: what a pinned dot's hint names. */
  pinnedLabelOf = (name: string): string | null => {
    const group = this.groupOf(name)
    return group === null ? null : this.labelOf(group)
  }

  /** How many groups exist — the menu's "Move to #x" range (`pinned-tag-groups` design D4). */
  groupCount = $derived(this.pinnedGroups.length)

  /**
   * Every pinned tag, by name, groups flattened in order — `pinned-tag-groups`
   * design D4's flattening of {@link pinnedGroups}, kept so
   * `selectionTagCounts` and any other flat reader cannot disagree with the
   * grouped view about which tags are pinned.
   */
  pinned = $derived(this.pinnedGroups.flatMap((group) => group.tags))

  /**
   * Re-reads the exceptions: on a library switch and after every tag write
   * that could have changed one — a category set from a context menu, a
   * pin toggled, or an editor save whose prefix created a categorised tag.
   */
  async refresh(): Promise<void> {
    await this.#apply(tagVocabulary())
  }

  async setCategory(name: string, category: TagCategory): Promise<void> {
    await this.#apply(setTagCategory(name, category))
  }

  /** Answers whether it landed, for the pin field that must keep its text on a refusal. */
  place(name: string, target: PinTarget): Promise<boolean> {
    return this.#apply(setTagPinnedGroup(name, target))
  }

  /** All of `names` move or none do. */
  async placeMany(names: string[], target: PinTarget): Promise<void> {
    await this.#apply(movePinnedTags(names, target))
  }

  async setGroupCollapsed(position: number, collapsed: boolean): Promise<void> {
    await this.#apply(setPinnedGroupCollapsed(position, collapsed))
  }

  async moveGroup(from: number, to: number): Promise<void> {
    await this.#apply(movePinnedGroup(from, to))
  }

  async deleteGroup(position: number): Promise<void> {
    await this.#apply(deletePinnedGroup(position))
  }

  /**
   * Answers whether it landed (`tag-notes` design D7): a menu's
   * fire-and-forget setter has nowhere to show a failure, but the note dialog
   * does and must stay open on one.
   */
  setNote(name: string, note: string | null): Promise<boolean> {
    return this.#apply(setTagNote(name, note))
  }

  /** Answers whether it landed, for the dialog that must stay open on a refusal. */
  renameGroup(position: number, name: string): Promise<boolean> {
    return this.#apply(renamePinnedGroup(position, name))
  }

  /** Answers whether it landed; a blank name is refused. */
  createGroup(name: string): Promise<boolean> {
    return this.#apply(createPinnedGroup(name))
  }

  /**
   * Replaces both arrays with a command's answer. A refusal is reported in
   * `error` and leaves them untouched.
   */
  async #apply(answer: Promise<VocabularyAnswer>): Promise<boolean> {
    try {
      const { tags, groups } = await answer
      this.entries = tags
      this.groups = groups
      this.error = null
      return true
    } catch (cause) {
      this.error = errorText(cause)
      return false
    }
  }
}

export const vocabulary = new Vocabulary()
