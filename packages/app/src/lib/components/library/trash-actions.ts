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
   * two or more ask first, naming the count (`needsConfirmation`, in
   * `pending-write.ts`).
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
