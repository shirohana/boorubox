// The write waiting on the one confirmation the library screen owns: the two
// irreversible trash acts, the multi-image trash move, and the multi-image
// rating write (`bulk-confirm` design D1). One shape here — instead of one
// per caller — is what lets every caller ask through the same `ConfirmDialog`
// rather than each keeping its own copy of the question and the count rule.

import type { Rating, TagEditSpec } from '@boorubox/shared'
import { ratingLabel } from '$lib/domain/format'

/**
 * What a confirmed write will do; `null` on the screen while nothing is asked.
 * The two irreversible acts have always been here (design D7); `trash` joined
 * them for the multi-image case only (design D12, amended), and `rate` for
 * the same reason (`bulk-confirm` design D1): a bulk rating write replaces
 * ratings that cannot be recovered afterwards. `edit` is one apply of a
 * `TagEditSpec` over a selection (`stamps` design D5, widened from
 * `tag-vocabulary` design D8's `{ add, remove }`): the pinned chip's single
 * tag, the bulk dialog's two lists, or a stamp's whole edit, across every id
 * in `ids`. `label` names what a stamp apply is for, read only when `spec` is
 * not a single tag add or remove — `confirmPrompt` below decides which.
 */
export type PendingWrite
  = | { kind: 'trash', ids: string[] }
    | { kind: 'delete', ids: string[] }
    | { kind: 'empty' }
    | { kind: 'rate', ids: string[], rating: Rating | null }
    | { kind: 'edit', ids: string[], spec: TagEditSpec, label: string }

/**
 * Design D12, amended: trashing is reversible, so one image goes without a
 * word — but `Cmd A` then `Backspace` is one keystroke away from the whole
 * library, and the count is the fact the user is missing at that moment. The
 * rating write reads the same rule (`bulk-confirm` design D1): one function
 * for "how many is many", whichever kind of write is asking.
 */
export function needsConfirmation(count: number): boolean {
  return count > 1
}

/** The trash's own count answers for `empty`, which names no ids. */
export function confirmedCount(pending: PendingWrite, trashCount: number): number {
  return pending.kind === 'empty' ? trashCount : pending.ids.length
}

export interface ConfirmPrompt {
  title: string
  description: string
  confirmLabel: string
  /** Whether confirming destroys something — the red button is for those only. */
  destructive: boolean
}

function images(count: number): string {
  return `${count.toLocaleString()} ${count === 1 ? 'image' : 'images'}`
}

function capitalized(word: string): string {
  return word[0].toUpperCase() + word.slice(1)
}

/**
 * The pinned chip's own shape of edit — exactly one tag, added or removed,
 * nothing else touched — so its prompt can keep naming the tag the way it
 * always has (design D5), rather than falling back to the stamp wording a
 * chip never carries a label for.
 */
function singleTagEdit(spec: TagEditSpec): { adding: boolean, tag: string } | null {
  const nothingElse = spec.addCollections.length === 0 && spec.removeCollections.length === 0
    && spec.rating === undefined
  if (!nothingElse) return null
  if (spec.add.length === 1 && spec.remove.length === 0) {
    return { adding: true, tag: spec.add[0] }
  }
  if (spec.remove.length === 1 && spec.add.length === 0) {
    return { adding: false, tag: spec.remove[0] }
  }
  return null
}

/**
 * The collection chip's own shape of edit — exactly one collection, added or
 * removed, nothing else touched (`pinned-collections` design D10), the
 * collection-keyed twin of {@link singleTagEdit}. `adding` alone is enough:
 * the collection's name is `pending.label`, not a field on the spec itself.
 */
function singleCollectionEdit(spec: TagEditSpec): { adding: boolean } | null {
  const nothingElse = spec.add.length === 0 && spec.remove.length === 0 && spec.rating === undefined
  if (!nothingElse) return null
  if (spec.addCollections.length === 1 && spec.removeCollections.length === 0) {
    return { adding: true }
  }
  if (spec.removeCollections.length === 1 && spec.addCollections.length === 0) {
    return { adding: false }
  }
  return null
}

/**
 * The question the one `ConfirmDialog` asks, per pending write. Here rather
 * than in the markup because the reversible act and the irreversible ones have
 * to read differently — "cannot be undone" belongs only to the ones that cannot.
 */
export function confirmPrompt(pending: PendingWrite, trashCount: number): ConfirmPrompt {
  const count = confirmedCount(pending, trashCount)
  if (pending.kind === 'trash') {
    return {
      title: `Move ${images(count)} to the trash?`,
      description: 'They leave the library and nothing on disk changes: their files, thumbnails, '
        + 'tags and ratings stay, and Restore puts them back.',
      confirmLabel: 'Move to trash',
      destructive: false,
    }
  }
  if (pending.kind === 'rate') {
    // Rating names come from the one place that already spells them for the
    // rating control, capitalized here: a title names the choice, where
    // `ratingLabel`'s callers so far have all wanted a lowercase tooltip.
    return {
      title: pending.rating === null
        ? `Clear the rating of ${images(count)}?`
        : `Set ${images(count)} to ${capitalized(ratingLabel(pending.rating))}?`,
      description: 'Their current ratings are replaced. There is no undo.',
      confirmLabel: pending.rating === null ? 'Clear rating' : 'Set rating',
      destructive: true,
    }
  }
  if (pending.kind === 'edit') {
    const single = singleTagEdit(pending.spec)
    if (single) {
      return {
        title: single.adding
          ? `Add “${single.tag}” to ${images(count)}?`
          : `Remove “${single.tag}” from ${images(count)}?`,
        description: 'Every other tag each image carries is left as it is.',
        confirmLabel: single.adding ? 'Add tag' : 'Remove tag',
        destructive: false,
      }
    }
    const singleCollection = singleCollectionEdit(pending.spec)
    if (singleCollection) {
      return {
        title: singleCollection.adding
          ? `Add ${images(count)} to “${pending.label}”?`
          : `Remove ${images(count)} from “${pending.label}”?`,
        description: 'Nothing else about them changes.',
        confirmLabel: singleCollection.adding ? 'Add' : 'Remove',
        destructive: false,
      }
    }
    // `stamps` design D5: a stamp's edit can touch tags, collections and the
    // rating at once, so there is no one part left to name — the label the
    // caller gave it (the stamp's own name, or its text with no name yet) is
    // the only thing that says what this apply does.
    return {
      title: `Apply “${pending.label}” to ${images(count)}?`,
      description: 'There is no undo.',
      confirmLabel: 'Apply',
      destructive: false,
    }
  }
  return {
    title: `Permanently delete ${images(count)}?`,
    description: 'This cannot be undone. Their records, their thumbnails and their files in the '
      + 'library folder are removed.',
    confirmLabel: 'Delete forever',
    destructive: true,
  }
}
