// The one membership write behind every place a collection action lives
// (`collections` design D8): the tile's context menu, the selection toolbar's
// dropdown and the inspector's "Add to…" menu. It sits here rather than in
// `CollectionMenuItems.svelte` because that component is rendered *inside* a
// menu, and bits-ui unmounts a menu's content the moment it closes — so the
// "New collection…" dialog has to be mounted by the caller, outside its menu,
// and the create-then-add it ends in cannot live in the component that closed.

import type { ImageRecord } from '@boorubox/shared'
import { collectionAdd, collectionRemove, collections, errorText } from '$lib/api'

/** What a collection write needs from wherever it was started. */
export interface CollectionTarget {
  /**
   * The images the write applies to, resolved at write time rather than at
   * render: `selection.ids()` over a live range is a round trip, and an array
   * kept from before the menu opened would name the wrong rows (design D8).
   */
  resolveIds: () => Promise<string[]>
  /**
   * The collection ids the caller's own image is in, for the checkmark and for
   * "does this add or remove" — read through a function so it follows the
   * record the write itself replaced. `null` where the caller acts on ids it
   * never loaded (the toolbar), and every item shows unchecked (design D8).
   */
  memberships: () => ReadonlySet<string> | null
  /** The written records, for the caller's own `replaceMany` (design D8/D10). */
  onwritten: (records: ImageRecord[]) => void
  onerror: (message: string) => void
}

/**
 * The one write both `toggleCollection` and the toolbar's explicit
 * Add/Remove items end in (design D8, amended): a range selection can span
 * rows the app never loaded, so a target with no memberships cannot say
 * "toggle" for itself — `action` is picked by the caller instead of read off
 * the target.
 */
async function writeCollection(
  target: CollectionTarget,
  collectionId: string,
  action: 'add' | 'remove',
): Promise<void> {
  try {
    const ids = await target.resolveIds()
    if (ids.length === 0) return
    target.onwritten(
      action === 'remove'
        ? await collectionRemove(ids, collectionId)
        : await collectionAdd(ids, collectionId),
    )
  } catch (cause) {
    target.onerror(errorText(cause))
  }
}

/** The action is "add all" while unchecked, "remove all" while checked (design D8). */
export async function toggleCollection(
  target: CollectionTarget,
  collectionId: string,
): Promise<void> {
  const member = target.memberships()?.has(collectionId) ?? false
  await writeCollection(target, collectionId, member ? 'remove' : 'add')
}

/**
 * A target with no memberships (the toolbar) cannot show a checkbox, so its
 * menu offers these two directly instead of one item that toggles (design
 * D8, amended). Removing an image that is not a member is a no-op on the
 * Rust side already (`collections::remove` deletes what exists).
 */
export async function addToCollection(
  target: CollectionTarget,
  collectionId: string,
): Promise<void> {
  await writeCollection(target, collectionId, 'add')
}

export async function removeFromCollection(
  target: CollectionTarget,
  collectionId: string,
): Promise<void> {
  await writeCollection(target, collectionId, 'remove')
}

/**
 * A collection created from one of the menus: the ids the menu was opened for
 * go straight in, the one write an existing row's click already makes. The
 * alternative — create, then reopen the same menu to check a box it could not
 * have shown a moment ago — is two trips for the one thing the user opened the
 * menu to do. The sidebar section's own "New collection…" has no ids in scope
 * and creates an empty collection, which is why this does not live in the
 * dialog.
 */
export async function addToCreated(
  target: CollectionTarget,
  collection: { id: string },
): Promise<void> {
  await collections.refresh()
  await toggleCollection(target, collection.id)
}
