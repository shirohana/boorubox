import type { ImportProgress, ImportReport } from '@boorubox/shared'
import { importPaths, libraryStatus } from './commands'
import { errorText } from './errors'
import { onImportProgress } from './events'
import { library } from './library.svelte'

/** A run that is going, or one waiting behind it. */
export interface ImportRun {
  id: number
  /** What was dropped or picked: folders count as one item until Rust walks them. */
  paths: string[]
  status: 'queued' | 'running'
  /** `null` until the first `import:progress` event; Rust counts the files first. */
  progress: ImportProgress | null
}

let nextRunId = 0

/**
 * Every import the app has been asked for, whatever started it — a drop, or the
 * Import menu. One singleton for the window: a run must outlive the route that
 * started it (a run whose only reference was the library route lost its progress
 * the moment the user opened settings), and the drop handler, the menu and the
 * band all read the same queue (design D9).
 *
 * Runs go one at a time, in order. Sequential on purpose: `import_paths`
 * serialises on the library mutex anyway, and `import:progress` carries no run
 * id, so two at once would interleave their counts into one meaningless number.
 */
export class Imports {
  /** The running one first, then what waits, in the order it was asked for. */
  runs = $state<ImportRun[]>([])
  /** Newest first, kept until dismissed: the only place a skipped file is named. */
  reports = $state<ImportReport[]>([])
  error = $state<string | null>(null)

  #pumping = false
  /* eslint-disable-next-line svelte/prefer-svelte-reactivity --
     Listeners, never rendered from. */
  #finished = new Set<() => void>()

  /** Queues a run and starts it when nothing is going. No paths is the one no-op. */
  enqueue(paths: string[]): void {
    if (paths.length === 0) return
    this.error = null
    this.runs = [...this.runs, { id: nextRunId++, paths, status: 'queued', progress: null }]
    void this.#pump()
  }

  /** A picker that the user cancelled answers with no paths, not an error. */
  async pick(picker: () => Promise<string[]>): Promise<void> {
    try {
      this.enqueue(await picker())
    } catch (cause) {
      this.error = errorText(cause)
    }
  }

  /**
   * Called once per run that ends, so the screen showing the library can
   * re-search. Returns its own unsubscribe, for an `$effect` to return.
   */
  onfinished(listener: () => void): () => void {
    this.#finished.add(listener)
    return () => this.#finished.delete(listener)
  }

  dismiss(report: ImportReport): void {
    this.reports = this.reports.filter((kept) => kept !== report)
  }

  /** For a library switch: the reports describe runs into the folder that closed. */
  dismissAll(): void {
    this.reports = []
  }

  async #pump(): Promise<void> {
    if (this.#pumping) return
    this.#pumping = true
    try {
      for (let run = this.runs[0]; run; run = this.runs[0]) {
        run.status = 'running'
        await this.#execute(run)
        this.runs = this.runs.slice(1)
      }
    } finally {
      this.#pumping = false
    }
  }

  async #execute(run: ImportRun): Promise<void> {
    // Subscribed before the command starts, or the first events of a fast
    // import are lost and the count jumps.
    const unlisten = await onImportProgress((update) => (run.progress = update))
    try {
      this.reports = [await importPaths(run.paths), ...this.reports]
      // `import_paths` answers with the report, not the status, and the sidebar
      // footer's total is on the status — so it is read once here rather than by
      // the footer, which would have nothing to tell it the number moved.
      library.set(await libraryStatus())
    } catch (cause) {
      this.error = errorText(cause)
    } finally {
      await unlisten()
      this.#finished.forEach((listener) => listener())
    }
  }
}

export const imports = new Imports()
