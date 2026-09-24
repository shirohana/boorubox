// A thumbnail regeneration the user starts from Settings → Library
// (`one-level-buckets` design D4, D5). Progress is subscribed from the
// layout, not scoped to the call that started it — the same reasoning as
// `sidecarsBackfill`: a tick could in principle arrive while the settings
// screen is not mounted, and a listener installed only inside `start()` would
// miss it.

import type { ThumbsReport } from '@boorubox/shared'
import type { UnlistenFn } from '@tauri-apps/api/event'
import { forgetAll } from '$lib/components/library/thumbnail-cache.svelte'
import { regenerateThumbnails } from './commands'
import { errorText } from './errors'
import { onThumbsProgress } from './events'

export class ThumbsRegenerate {
  running = $state(false)
  /** `null` until the first `thumbs:progress` tick. */
  progress = $state<{ done: number, total: number } | null>(null)
  /** The finished run's report, kept until {@link reset} or the next run. */
  report = $state.raw<ThumbsReport | null>(null)
  error = $state<string | null>(null)

  /**
   * Bumped by {@link reset}. A library switch calls `reset` while a stopped
   * pass's `regenerate_thumbnails` may still be unwinding — Rust answers
   * `Ok(partial)` once the open library's root has already changed away from
   * the one the pass started against, and a tick already in flight can land
   * after that. `start` captures the token its own run owns in
   * {@link liveToken}; both its own continuation and the progress handler
   * below check that token is still current before writing, so a stale
   * report or tick never lands on the new library's screen.
   */
  #runToken = 0
  /** The token of the run currently allowed to write, `null` between runs. */
  #liveToken: number | null = null

  /**
   * Listens for the life of the window, mirroring
   * `sidecarsBackfill.subscribe`. Drops the thumbnail cache as soon as the
   * last tick arrives (design D6) — waiting for {@link start} to resolve
   * would leave the grid on the old bytes for however long the command takes
   * to unwind after its final progress emit.
   */
  async subscribe(): Promise<UnlistenFn> {
    return onThumbsProgress((update) => {
      if (this.#liveToken !== this.#runToken) return
      this.progress = update
      if (update.done >= update.total) forgetAll()
    })
  }

  /** Starts a pass. Resolves with the report, or `null` on failure. */
  async start(): Promise<ThumbsReport | null> {
    // Rust's own `Busy` refusal already keeps a second pass from running,
    // but without this a second call while one is in flight would still
    // clear the real pass's progress below and, once Rust's refusal lands in
    // its `catch`, re-enable the button through this call's own `finally`.
    if (this.running) return null
    this.reset()
    const token = this.#runToken
    this.#liveToken = token
    this.running = true
    try {
      const report = await regenerateThumbnails()
      if (token === this.#runToken) this.report = report
      return report
    } catch (cause) {
      if (token === this.#runToken) this.error = errorText(cause)
      return null
    } finally {
      // Unconditional: the button and Rust's own flag are what keep a second
      // run out until this one settles, so a superseded run still has to
      // release the guard when its command resolves.
      this.running = false
      // Idempotent with the drop above (design D6): a pass over zero images
      // never ticks, so this is the only drop that scenario gets, and a pass
      // stopped by a library switch may resolve before its last tick's drop
      // has run.
      forgetAll()
    }
  }

  /**
   * A library switch or close leaves a running pass writing into a folder the
   * screen no longer shows (D4: it stops on its own). Clears the tile the
   * same way `sidecarsBackfill.reset` does; `start`'s own `finally` clears
   * `running` when the stopped pass's command settles. Bumps {@link
   * #runToken} so that stopped pass's late report and ticks are dropped
   * rather than landing on whatever library is open when they arrive.
   */
  reset(): void {
    this.progress = null
    this.report = null
    this.error = null
    this.#runToken++
  }
}

export const thumbsRegenerate = new ThumbsRegenerate()
