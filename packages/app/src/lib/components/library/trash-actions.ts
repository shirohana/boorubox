// The three writes the trash offers, as every control on the library screen
// receives them (`trash` design D13: the tile menu, the inspector's action row
// and the selection toolbar all offer the same pair). One object rather than
// three props at each level: the set grows and shrinks together, and the screen
// that owns them is the only place that knows what a write has to refresh
// afterwards.
//
// Which of them a control offers is decided by `SearchResults.view`, which
// every one of those controls already holds — this interface deliberately does
// not carry a second copy of it.

export interface TrashActions {
  /**
   * Moves the ids to the trash. Reversible, so one image goes straight in;
   * two or more ask first, naming the count (`needsTrashConfirmation`).
   */
  trash: (ids: string[]) => void
  /** Puts trashed ids back in the library, as they were. */
  restore: (ids: string[]) => void
  /**
   * Asks for the confirmation that names how many images will be destroyed
   * (design D7). Nothing is deleted until it is confirmed.
   */
  deleteForever: (ids: string[]) => void
}

/**
 * What a confirmed write will do; `null` on the screen while nothing is asked.
 * The two irreversible acts have always been here (design D7); `trash` joined
 * them for the multi-image case only (design D12, amended).
 */
export type PendingWrite
  = | { kind: 'trash', ids: string[] }
    | { kind: 'delete', ids: string[] }
    | { kind: 'empty' }

/**
 * Design D12, amended: trashing is reversible, so one image goes without a
 * word — but `Cmd A` then `Backspace` is one keystroke away from the whole
 * library, and the count is the fact the user is missing at that moment.
 */
export function needsTrashConfirmation(count: number): boolean {
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

/**
 * The question the one `ConfirmDialog` asks, per pending write. Here rather
 * than in the markup because the reversible act and the irreversible ones have
 * to read differently — "cannot be undone" belongs only to the two that cannot.
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
  return {
    title: `Permanently delete ${images(count)}?`,
    description: 'This cannot be undone. Their records, their thumbnails and their files in the '
      + 'library folder are removed.',
    confirmLabel: 'Delete forever',
    destructive: true,
  }
}
