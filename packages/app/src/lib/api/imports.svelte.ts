import type { ImportProgress, ImportReport } from '@boorubox/shared'
import {
  importBundle,
  importCancel,
  importPause,
  importPaths,
  importResume,
  libraryStatus,
} from './commands'
import { errorText } from './errors'
import { onImportProgress } from './events'
import { library } from './library.svelte'

interface ImportRunFields {
  id: number
  status: 'queued' | 'running'
  /** `null` until the first `import:progress` event; Rust counts the files first. */
  progress: ImportProgress | null
}

/**
 * A run that is going, or one waiting behind it: a local paths run (files and
 * folders picked or dropped) or a legacy-bundle run (`.db` part files,
 * `legacy-bundle-import` design D6) — one queue, one progress band, so
 * `#execute` switches on `kind` to call the matching command.
 */
export type ImportRun
  = | (ImportRunFields & { kind: 'paths', paths: string[] })
    | (ImportRunFields & { kind: 'bundle', files: string[] })

/**
 * A stored report, extended with what only the webview knows: which kind of
 * run produced it — the two importers' re-run behaviour differs
 * (`local-file-import`/`legacy-bundle-import` specs) — and how many runs
 * queued behind it were discarded when it was cancelled
 * (`pending-work` spec, `import-pause-cancel` design D6). Rust's
 * `ImportReport` carries neither: the queue exists only in the webview.
 */
export interface ImportReportEntry extends ImportReport {
  kind: ImportRun['kind']
  queuedDiscarded: number
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
  reports = $state<ImportReportEntry[]>([])
  /**
   * The most recent bundle run's report, for the `/import` route (design D7):
   * `reports` above is the library band's per-run list and keeps showing
   * bundle reports too (design D6), but the route shows one report, not a
   * history. `$state.raw`: always replaced whole, never mutated in place, and
   * a bundle's `items` can run to thousands of rows not worth proxying.
   */
  latestBundleReport = $state.raw<ImportReportEntry | null>(null)
  error = $state<string | null>(null)
  /**
   * Whether the running import is parked (`import-pause-cancel` design D1).
   * One flag, not a per-run field: only the run in front can ever be paused,
   * and the band reads this directly rather than off a run object it would
   * have to derive from (CLAUDE.md: derive, never `$effect`, off a
   * wholesale-reassigned store field — this is a plain primitive instead).
   */
  paused = $state(false)

  #pumping = false
  /* eslint-disable-next-line svelte/prefer-svelte-reactivity --
     Listeners, never rendered from. */
  #finished = new Set<() => void>()
  /**
   * Runs dropped from the queue by a cancel that landed while the current one
   * was still going, counted here until `#execute` attaches the count to the
   * report the cancelled run produces (design D6). Not `$state`: nothing
   * renders it directly, only the report it ends up on. Reset in `#execute`'s
   * `finally`, win or lose: a rejected command must not leave this set for
   * the next, unrelated run to inherit.
   */
  #queuedDiscarded = 0
  /**
   * A Cancel/Pause pressed while Rust's control slot for the running run has
   * not been installed yet — the window between issuing `import_paths`/
   * `import_bundle` and Rust actually starting, which recurs between every
   * two queued runs even though the tile already reads `running` — is a
   * silent no-op there (design D4: the commands are a no-op with nothing
   * running). Latched here and replayed on the run's first `import:progress`
   * tick, which can only fire once the handle exists (design D2: the
   * checkpoint that would see Cancel/Pause runs after `on_progress`, and
   * needs the handle to do it).
   */
  #cancelRequested = false
  #pauseRequested = false

  /** Queues a paths run and starts it when nothing is going. No paths is the one no-op. */
  enqueue(paths: string[]): void {
    this.#enqueue(paths.length === 0 ? null : { kind: 'paths', paths })
  }

  /** Queues a bundle run (`legacy-bundle-import` design D5). No files is the one no-op. */
  enqueueBundle(files: string[]): void {
    this.#enqueue(files.length === 0 ? null : { kind: 'bundle', files })
  }

  #enqueue(
    started: { kind: 'paths', paths: string[] } | { kind: 'bundle', files: string[] } | null,
  ): void {
    if (!started) return
    this.error = null
    this.runs = [...this.runs, { id: nextRunId++, status: 'queued', progress: null, ...started }]
    void this.#pump()
  }

  /** A picker that the user cancelled answers with no paths, not an error. */
  async pick(picker: () => Promise<string[]>): Promise<void> {
    await this.#pick(picker, (paths) => this.enqueue(paths))
  }

  /** A bundle-file picker that the user cancelled answers with no files, not an error. */
  async pickBundle(picker: () => Promise<string[]>): Promise<void> {
    await this.#pick(picker, (files) => this.enqueueBundle(files))
  }

  async #pick(picker: () => Promise<string[]>, enqueue: (picked: string[]) => void): Promise<void> {
    try {
      enqueue(await picker())
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

  dismiss(report: ImportReportEntry): void {
    this.reports = this.reports.filter((kept) => kept !== report)
  }

  /** For a library switch: the reports describe runs into the folder that closed. */
  dismissAll(): void {
    this.reports = []
    this.latestBundleReport = null
  }

  /** Pauses the running import between items; a no-op with nothing running. */
  async pause(): Promise<void> {
    this.#pauseRequested = true
    try {
      await importPause()
      this.paused = true
    } catch (cause) {
      this.error = errorText(cause)
    }
  }

  /** Resumes a paused import; a no-op with nothing running or paused. */
  async resume(): Promise<void> {
    this.#pauseRequested = false
    try {
      await importResume()
      this.paused = false
    } catch (cause) {
      this.error = errorText(cause)
    }
  }

  /**
   * Cancels the *running* import and discards every run still `queued`
   * behind it (design D6, `pending-work` "Cancel discards the runs queued
   * behind it") — the running tile's Cancel. A waiting tile's Cancel is
   * {@link dequeue} instead, which drops only itself. The queue is dropped
   * synchronously, before the command round-trips, so the band reflects it
   * at once; the discarded count is attached once `#execute` builds the
   * report the cancelled run produces.
   */
  async cancel(): Promise<void> {
    this.#cancelRequested = true
    this.#pauseRequested = false
    const discarded = this.runs.filter((run) => run.status === 'queued')
    if (discarded.length > 0) {
      this.runs = this.runs.filter((run) => run.status !== 'queued')
    }
    this.#queuedDiscarded += discarded.length
    this.paused = false
    try {
      await importCancel()
    } catch (cause) {
      this.error = errorText(cause)
    }
  }

  /**
   * Removes one run still waiting in the queue — a waiting tile's Cancel
   * (`pending-work` spec: distinct from the running tile's {@link cancel},
   * which stops the run and drops the whole queue). Nothing has started for
   * this run, so there is no command to call and it never counts toward a
   * report's `queuedDiscarded` — that count is only what a *running* Cancel
   * drops (design D6).
   */
  dequeue(id: number): void {
    this.runs = this.runs.filter((run) => !(run.id === id && run.status === 'queued'))
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
    const unlisten = await onImportProgress((update) => {
      run.progress = update
      // The first tick is proof Rust's control slot for this run now exists
      // (see `#cancelRequested`'s comment): replay whichever request is still
      // outstanding, then clear it, so a request that landed too early to
      // take effect is not lost for the rest of the run.
      if (this.#cancelRequested) {
        this.#cancelRequested = false
        void importCancel()
      } else if (this.#pauseRequested) {
        this.#pauseRequested = false
        void importPause()
      }
    })
    try {
      const raw = run.kind === 'paths'
        ? await importPaths(run.paths)
        : await importBundle(run.files)
      // `raw` carries `cancelled` from Rust; `kind` and `queuedDiscarded` are
      // this run's own webview-side additions (design D6).
      const report: ImportReportEntry = {
        ...raw,
        kind: run.kind,
        queuedDiscarded: this.#queuedDiscarded,
      }
      this.reports = [report, ...this.reports]
      if (run.kind === 'bundle') this.latestBundleReport = report
      // `import_paths`/`import_bundle` answer with the report, not the status,
      // and the sidebar footer's total is on the status — so it is read once
      // here rather than by the footer, which would have nothing to tell it
      // the number moved.
      library.set(await libraryStatus())
    } catch (cause) {
      this.error = errorText(cause)
    } finally {
      this.paused = false
      this.#queuedDiscarded = 0
      this.#cancelRequested = false
      this.#pauseRequested = false
      await unlisten()
      this.#finished.forEach((listener) => listener())
    }
  }
}

export const imports = new Imports()
