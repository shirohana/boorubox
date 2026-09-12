// A rebuild the user asked for (`library-sidecars` design D12): from the
// start screen's damaged state, with no library open, or from Settings →
// Library, with one open. One store for both — `rebuild_library` only ever
// runs one at a time, and neither caller needs a queue.

import type { RebuildReport } from '@boorubox/shared'
import { rebuildLibrary } from './commands'
import { errorText } from './errors'
import { onRebuildProgress } from './events'

export class Rebuild {
  running = $state(false)
  /** `null` until the first `library:rebuild` tick. */
  progress = $state<{ done: number, total: number } | null>(null)
  /**
   * The finished run's report, kept until {@link reset} — the caller reads
   * what happened, then opens the library from it (design D12: "the user
   * SHALL see what the rebuild did before the library opens"). `$state.raw`:
   * always replaced whole, and `failures` can run to as many rows as the
   * rebuild found unreadable.
   */
  report = $state.raw<RebuildReport | null>(null)
  error = $state<string | null>(null)

  /** Rebuilds `path`. Resolves with the report, or `null` on failure. */
  async run(path: string): Promise<RebuildReport | null> {
    this.running = true
    this.progress = null
    this.report = null
    this.error = null
    // Subscribed before the command starts, or an early tick on a small
    // library is lost, the same ordering `Imports#execute` uses.
    const unlisten = await onRebuildProgress((update) => {
      this.progress = update
    })
    try {
      const report = await rebuildLibrary(path)
      this.report = report
      return report
    } catch (cause) {
      this.error = errorText(cause)
      return null
    } finally {
      this.running = false
      await unlisten()
    }
  }

  /** Clears the report and any error, so a second rebuild starts clean. */
  reset(): void {
    this.report = null
    this.error = null
  }
}

export const rebuild = new Rebuild()
