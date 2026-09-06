import type { ImportProgress, ImportReport } from '@boorubox/shared'
import { errorText, importPaths, library, libraryStatus, onImportProgress } from '$lib/api'

/**
 * One import run, whatever started it. Design D8 moved the window's drag-and-drop
 * subscription out of the panel and up to the route, and the `Import` menu that
 * replaced the panel is unmounted while its dropdown is closed — so the run
 * itself cannot live in either of them, or a drop and a picked folder would be
 * two implementations of the same thing.
 */
export class Imports {
  running = $state(false)
  /** `null` until the first `import:progress` event; Rust counts the files first. */
  progress = $state<ImportProgress | null>(null)
  report = $state<ImportReport | null>(null)
  error = $state<string | null>(null)

  #onimported: () => void

  constructor(onimported: () => void) {
    this.#onimported = onimported
  }

  async run(paths: string[]): Promise<void> {
    if (this.running || paths.length === 0) return
    this.running = true
    this.report = null
    this.error = null
    this.progress = null

    // Subscribed before the command starts, or the first events of a fast
    // import are lost and the count jumps.
    const unlisten = await onImportProgress((update) => (this.progress = update))
    try {
      this.report = await importPaths(paths)
      // `import_paths` answers with the report, not the status, and the sidebar
      // footer's total is on the status — so it is read once here rather than by
      // the footer, which would have nothing to tell it the number moved.
      library.set(await libraryStatus())
      this.#onimported()
    } catch (cause) {
      this.error = errorText(cause)
    } finally {
      await unlisten()
      this.running = false
      this.progress = null
    }
  }

  /** A picker that the user cancelled answers with no paths, not an error. */
  async pick(picker: () => Promise<string[]>): Promise<void> {
    try {
      await this.run(await picker())
    } catch (cause) {
      this.error = errorText(cause)
    }
  }

  dismiss(): void {
    this.report = null
  }
}
