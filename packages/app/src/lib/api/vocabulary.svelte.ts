// The library's tag vocabulary, for every place that reads "what category is
// this tag, and is it pinned" (`tag-vocabulary` design D5): the inspector's
// tags, the pinned chips, the sidebar's tag list and the editor's
// suggestion popover. One store, the shape of `collections.svelte.ts`, so
// those readers cannot disagree about a tag's colour a moment after its
// category changed.
//
// `entries` holds the exceptions only (design D2) — a general, unpinned tag
// is never in it, so this is hundreds of names in a library of thousands of
// tags — and `#byName` is the Map that makes a lookup cheap over that list;
// `$derived` rebuilds it only when a refresh or a setter replaces `entries`
// wholesale, never once per `categoryOf` call from a panel full of badges.
//
// No `setCategory`/`place` write of its own kind to read back: each setter's
// command already answers the whole vocabulary, so the setters below replace
// `entries` with that answer directly, the cheapest way to keep the one list
// in step.

import type { PinTarget, TagCategory, TagEntry } from '@boorubox/shared'
import { groupByCategory } from '$lib/domain/tag-categories'
import { setTagCategory, setTagPinnedGroup, tagVocabulary } from './commands'
import { errorText } from './errors'

export class Vocabulary {
  /** Every tag that is not `(general, unpinned)`, as Rust answers. */
  entries = $state<TagEntry[]>([])
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

  /**
   * Every pinned tag's group, in group order, each group in the sidebar's
   * own order — `CATEGORY_ORDER` first, alphabetical within a category
   * (`pinned-collections` design D7, `pinned-tag-groups` design D4). Made
   * here, once, because the inspector's two placements both draw the pinned
   * rows from this list rather than sorting their own: a category changing
   * cannot leave one placement's chip row out of step with the other's.
   * `groupByCategory` is the sidebar's own grouping (`domain/tag-categories.ts`),
   * not a second copy of it. Group numbers come from the entries themselves,
   * not a counted range, so a group Rust has already compacted is never
   * second-guessed here.
   */
  pinnedGroups: string[][] = $derived.by(() => {
    const pinnedEntries = this.entries.filter((entry) => entry.pinnedGroup !== null)
    const groupNumbers = pinnedEntries
      .map((entry) => entry.pinnedGroup!)
      .filter((groupNumber, index, all) => all.indexOf(groupNumber) === index)
      .sort((a, b) => a - b)
    return groupNumbers.map((groupNumber) =>
      groupByCategory(
        pinnedEntries.filter((entry) => entry.pinnedGroup === groupNumber),
        (entry) => entry.name,
        this.categoryOf,
      ).flatMap((group) => group.items.map((entry) => entry.name)),
    )
  })

  /** How many groups exist — the menu's "Move to #x" range (design D4). */
  groupCount = $derived(this.pinnedGroups.length)

  /**
   * Every pinned tag, by name, groups flattened in order — `pinned-tag-groups`
   * design D4's flattening of {@link pinnedGroups}, kept so
   * `selectionTagCounts` and any other flat reader cannot disagree with the
   * grouped view about which tags are pinned.
   */
  pinned = $derived(this.pinnedGroups.flat())

  /**
   * Re-reads the exceptions: on a library switch and after every tag write
   * that could have changed one — a category set from a context menu, a
   * pin toggled, or an editor save whose prefix created a categorised tag.
   */
  async refresh(): Promise<void> {
    try {
      this.entries = await tagVocabulary()
      this.error = null
    } catch (cause) {
      this.error = errorText(cause)
    }
  }

  /** A refusal is reported the same way `refresh()` reports one: `entries` untouched. */
  async setCategory(name: string, category: TagCategory): Promise<void> {
    try {
      this.entries = await setTagCategory(name, category)
      this.error = null
    } catch (cause) {
      this.error = errorText(cause)
    }
  }

  /** A refusal is reported the same way `refresh()` reports one: `entries` untouched. */
  async place(name: string, target: PinTarget): Promise<void> {
    try {
      this.entries = await setTagPinnedGroup(name, target)
      this.error = null
    } catch (cause) {
      this.error = errorText(cause)
    }
  }
}

export const vocabulary = new Vocabulary()
