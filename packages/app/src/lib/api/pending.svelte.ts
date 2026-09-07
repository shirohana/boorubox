// Captures the app has been told are coming. The extension announces one
// before it fetches the bytes, so the library screen can show a placeholder on
// the click instead of a second of nothing (design D3).
//
// It lives only here: nothing pending is written to the database or held by
// Rust, so no count, no search and no later reader of the library can mistake
// it for content.

import type { CaptureMeta } from '@boorubox/shared'
import type { UnlistenFn } from '@tauri-apps/api/event'
import { onCapturePending, onCaptureStored, onCaptureWithdrawn } from './events'

/**
 * How long a placeholder waits for its capture before dropping itself.
 *
 * The extension cannot always settle its own announcement: a worker killed
 * during the download has no history entry yet, so its startup sweep has no id
 * to withdraw, and a browser quit mid-capture sends nothing at all. The bound
 * has to outlast the slowest honest capture — the extension's own POST timeout
 * is 30 s, after a large original that can take tens of seconds to download —
 * without leaving a ghost tile for minutes (design D4). A capture that arrives
 * after it still lands: `capture:stored` refreshes the grid whether or not a
 * tile was waiting.
 */
export const PENDING_TTL_MS = 120_000

/**
 * Schedules `run` and answers with its cancel. The one thing a test has to
 * take over to assert on the expiry without waiting two minutes for it.
 */
export type Clock = (run: () => void, delay: number) => () => void

const timers: Clock = (run, delay) => {
  const handle = setTimeout(run, delay)
  return () => clearTimeout(handle)
}

export class PendingCaptures {
  #entries = $state<CaptureMeta[]>([])
  /* eslint-disable-next-line svelte/prefer-svelte-reactivity --
     Cancels, never rendered from: the entries are the reactive state. */
  #cancels = new Map<string, () => void>()
  #clock: Clock

  constructor(clock: Clock = timers) {
    this.#clock = clock
  }

  /** Newest first, so a new announcement is the first tile of the band. */
  get entries(): CaptureMeta[] {
    return this.#entries
  }

  /**
   * Listens for the life of the window, from the layout rather than the library
   * route: a capture announced while the user is on another screen has to be on
   * the band when they come back, and a Tauri event emitted with no listener is
   * simply lost.
   */
  async subscribe(): Promise<UnlistenFn> {
    const unlistens = await Promise.all([
      onCapturePending((meta) => this.#announce(meta)),
      onCaptureStored((image) => this.#drop(image.id)),
      onCaptureWithdrawn((withdrawn) => this.#drop(withdrawn.id)),
    ])
    return async () => {
      await Promise.all(unlistens.map((unlisten) => unlisten()))
      this.#forgetAll()
    }
  }

  #announce(meta: CaptureMeta): void {
    // Delivery is idempotent on the id, so a re-announced capture is the same
    // one arriving twice, not two tiles.
    this.#drop(meta.id)
    this.#entries = [meta, ...this.#entries]
    this.#cancels.set(meta.id, this.#clock(() => this.#drop(meta.id), PENDING_TTL_MS))
  }

  #drop(id: string): void {
    this.#cancels.get(id)?.()
    this.#cancels.delete(id)
    this.#entries = this.#entries.filter((entry) => entry.id !== id)
  }

  #forgetAll(): void {
    this.#cancels.forEach((cancel) => cancel())
    this.#cancels.clear()
    this.#entries = []
  }
}

export const pendingCaptures = new PendingCaptures()
