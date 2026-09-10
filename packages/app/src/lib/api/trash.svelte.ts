// The number in the sidebar's Trash badge, held here rather than on
// `LibraryStatus` (design D11): the four trash commands are its only writers,
// and nothing else in the app invalidates it. A capture, an import, a tag edit
// and a library switch all leave it alone — the switch replaces it wholesale,
// which is a fresh read either way.

import { trashCount } from './commands'

export class Trash {
  /** How many images are in the trash; 0 until the first read answers. */
  count = $state(0)

  /**
   * Re-reads the count: on a library opening, and after every trash, restore,
   * delete-forever and empty.
   *
   * A failed read leaves the last known number on screen. Falling back to zero
   * would say the trash is empty — which is both wrong and the one state in
   * which the app stops offering "Empty trash…" at all.
   */
  async refresh(): Promise<void> {
    try {
      this.count = await trashCount()
    } catch {
      // The badge is a moment old, not wrong.
    }
  }
}

export const trash = new Trash()
