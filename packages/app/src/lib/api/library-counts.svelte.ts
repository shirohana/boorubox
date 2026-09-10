// The per-source counts `/settings` and `/import` both show (`library-browse`
// design D7, `legacy-bundle-import` design D7): one store, so the two routes
// share one fetch and one pair of triggers instead of each keeping its own
// copy wired to the same events. `Sidebar.svelte` drives the refresh — it is
// mounted for every route with a library open, so neither screen has to be
// the one that is (`trash` design D11 does the same for the trash count).

import type { ImageCounts } from '@boorubox/shared'
import { imageCounts } from './commands'
import { errorText } from './errors'

export class LibraryCounts {
  current = $state<ImageCounts | null>(null)
  error = $state<string | null>(null)

  /** Re-reads the counts: on a library switch, and after every import run. */
  async refresh(): Promise<void> {
    try {
      this.current = await imageCounts()
      this.error = null
    } catch (cause) {
      this.error = errorText(cause)
    }
  }
}

export const libraryCounts = new LibraryCounts()
