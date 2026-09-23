// The library's stamps, for the two places that list them (`stamps` design
// D3): the settings screen's `StampsSection` and the library screen's stamp
// bar. One store, the shape of `collections.svelte.ts`, so those two readers
// cannot disagree about a stamp's text a moment after either wrote one.
//
// No `create`/`upsert`/`delete` of its own: `StampForm` and `StampsTable`
// call the commands directly and then `refresh()`, `collections.svelte.ts`'s
// own rule.

import type { Stamp } from '@boorubox/shared'
import { stampsList } from './commands'
import { errorText } from './errors'

export class Stamps {
  /** Every stamp in the library, by creation order, as Rust answers. */
  list = $state<Stamp[]>([])
  /** Why the list could not be read; `null` while it is in step. */
  error = $state<string | null>(null)

  /**
   * Re-reads the list: on a library switch and after every create, edit or
   * delete.
   */
  async refresh(): Promise<void> {
    try {
      this.list = await stampsList()
      this.error = null
    } catch (cause) {
      this.error = errorText(cause)
    }
  }
}

export const stamps = new Stamps()
