// The catch-up pass a library runs at open, writing the describing file any
// image is missing (`library-sidecars` design D7, `library-recovery` spec).
// The webview never starts this — Rust does, from `open_into_state` — so this
// store only shows it: one tile in the pending-work band while it goes.
//
// It lives only here, the same reasoning as `pending.svelte.ts`: nothing about
// the pass is content, so nothing here is read by a search or a count.

import type { UnlistenFn } from '@tauri-apps/api/event'
import { onSidecarsProgress } from './events'

export class SidecarsBackfill {
  /**
   * `null` with no pass running, including a library already in step — Rust
   * emits no tick at all for one (`pending-work` spec, "A library already in
   * step shows no tile at all").
   */
  progress = $state<{ done: number, total: number } | null>(null)

  /**
   * Listens for the life of the window, from the layout rather than the
   * library route — the same reasoning as `pendingCaptures.subscribe`: a
   * library can finish its catch-up while the user is on another screen.
   */
  async subscribe(): Promise<UnlistenFn> {
    return onSidecarsProgress((update) => {
      // The last tick always has `done === total` (mirroring
      // `ImportProgress`'s own guarantee). Showing it would be a result
      // nobody asked for and nothing to dismiss it (`pending-work` spec:
      // "leaves no result to dismiss"), so the tile goes at once instead.
      this.progress = update.done >= update.total ? null : update
    })
  }

  /**
   * A library switch or close leaves a running pass writing into a folder the
   * screen no longer shows (`pending-work` spec, "Switching libraries
   * mid-pass"). Rust stops the pass on its own; this only clears the tile,
   * since the webview has no way to tell "stopped" from "finished" apart from
   * silence either way.
   */
  reset(): void {
    this.progress = null
  }
}

export const sidecarsBackfill = new SidecarsBackfill()
