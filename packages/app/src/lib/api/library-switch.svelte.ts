// The one guard every library-swap path goes through (`import-confirm`
// design D6, spec `library-switching`): the switch menu's Switch to, Choose
// folder and Close library, and the start screen's recent entries and its
// picker. Confirming cancels from the webview and waits for the run to
// settle before the swap proceeds, which is what makes "the waiting runs are
// discarded" and "the reports describe the library that was open" true —
// Rust's own `cancel_running_import` only ever sees the run in flight, not
// the queue behind it (`import-pause-cancel` design D6, `legacy-bundle-import`
// design D6 quoted in this change's design doc).

import type { ImportRun } from './imports.svelte'
import { Imports, imports } from './imports.svelte'

/** Whether a guarded action closes the library or opens a different one. */
export type LibrarySwitchAction = 'close' | 'switch'

/**
 * The running run's kind and how many wait behind it, and which kind of swap
 * is asking — snapshotted once, when the question is asked, so the dialog's
 * words cannot change out from under the user while it is up.
 */
export interface PendingLibrarySwitch {
  kind: ImportRun['kind']
  queued: number
  action: LibrarySwitchAction
}

export class LibrarySwitch {
  /** The question up, or `null` when nothing is being asked. */
  pending = $state<PendingLibrarySwitch | null>(null)
  /**
   * Set once confirmed, while {@link Imports.cancelAndSettle} is still
   * waiting on the run's own report — {@link pending} stays set through
   * this so the dialog keeps showing (with different words) rather than
   * vanishing before the swap it promised has happened.
   */
  stopping = $state(false)

  #resolve?: (confirmed: boolean) => void

  /**
   * The queue read for {@link pending} and cancelled by a confirmed guard.
   * The real singleton by default; a test builds a `LibrarySwitch` on a
   * fresh {@link Imports} instead, the same way `imports.svelte.test.ts`
   * tests the queue itself without touching the app's one singleton.
   */
  constructor(private readonly queue: Imports = imports) {}

  /**
   * Runs `action` at once when nothing is running or queued. Otherwise asks
   * first, awaiting the answer: confirming cancels the run, discards the
   * queue, waits for the cancelled run to settle, and only then runs
   * `action`; declining runs nothing and leaves the queue exactly as it was.
   */
  async guard(action: LibrarySwitchAction, run: () => Promise<void>): Promise<void> {
    // A second question while one is up would replace the first caller's
    // resolver and strand its promise. The dialog is modal, so no click can
    // reach here meanwhile; dropping the late caller is the honest answer.
    if (this.pending) return
    const [running] = this.queue.runs
    if (!running) {
      await run()
      return
    }
    this.pending = { kind: running.kind, queued: this.queue.runs.length - 1, action }
    const confirmed = await new Promise<boolean>((resolve) => {
      this.#resolve = resolve
    })
    if (!confirmed) {
      this.pending = null
      return
    }
    this.stopping = true
    try {
      await this.queue.cancelAndSettle()
      await run()
    } finally {
      this.stopping = false
      this.pending = null
    }
  }

  /** Answers a pending question with yes. A no-op with no question up. */
  confirm(): void {
    this.#resolve?.(true)
    this.#resolve = undefined
  }

  /** Answers a pending question with no. A no-op with no question up. */
  decline(): void {
    this.#resolve?.(false)
    this.#resolve = undefined
  }
}

export const librarySwitch = new LibrarySwitch()
