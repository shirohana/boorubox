// The collections a library has, for every place that offers adding to or
// removing from one (`collections` design D7): the tile's context menu, the
// selection toolbar's dropdown, the inspector's "Add to…" menu, and the
// sidebar's own section. One store, so those readers cannot disagree about
// the list a moment after a collection was created — `booru.svelte.ts`'s own
// reason for keeping one list of sites.
//
// Counts are not here: `results.counts.collections` already answers "how many
// of what I am looking at" per search (design D7), so this store only ever
// holds the collections themselves, never how many images are in one.
//
// No `create`/`rename`/`delete` of its own: `CollectionNameDialog` and
// `CollectionsSection` call the commands directly and then `refresh()`, the
// way `RuleList` re-reads its list after a write rather than the store
// guessing at what its own write changed.

import type { Collection } from '@boorubox/shared'
import { collectionList, setCollectionPinned } from './commands'
import { errorText } from './errors'

export class Collections {
  /** Every collection in the library, by name, as Rust answers. */
  list = $state<Collection[]>([])
  /** Why the list could not be read; `null` while it is in step. */
  error = $state<string | null>(null)

  /**
   * Every pinned collection, by name — the chip row at the top of the
   * inspector's Collections section (`pinned-collections` design D5, D7).
   * `list` is already in name order from Rust, so this needs no sort of its own.
   */
  pinned = $derived(this.list.filter((collection) => collection.pinned))

  /**
   * Re-reads the list: on a library switch and after every create, rename or
   * delete.
   */
  async refresh(): Promise<void> {
    try {
      this.list = await collectionList()
      this.error = null
    } catch (cause) {
      this.error = errorText(cause)
    }
  }

  byId(id: string): Collection | undefined {
    return this.list.find((collection) => collection.id === id)
  }

  /** A refusal is reported the same way `refresh()` reports one: `list` untouched. */
  async setPinned(id: string, pinned: boolean): Promise<void> {
    try {
      this.list = await setCollectionPinned(id, pinned)
      this.error = null
    } catch (cause) {
      this.error = errorText(cause)
    }
  }
}

export const collections = new Collections()
