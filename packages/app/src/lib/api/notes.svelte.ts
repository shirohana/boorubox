// The library's note as the panel holds it, with the debounced write that
// saves it (`notes` design D14). The debounce lives here rather than in the
// component because a library switch has to flush it from a menu the panel
// knows nothing about — `LibraryMenu` calls `flush()` before every action that
// changes which library is open, and a write that landed after the switch would
// put one library's note into another's file.

import { noteGet, noteSet } from './commands'
import { errorText } from './errors'

/** The legacy's interval, kept: a paragraph is one write, not two hundred. */
export const NOTE_DEBOUNCE_MS = 500

export class Notes {
  /** What the text area shows. Empty until the first `load()` answers. */
  content = $state('')
  /** Why the last read or write failed; `null` while the note is in step. */
  error = $state<string | null>(null)

  #timer: ReturnType<typeof setTimeout> | null = null
  /** The text still owed to Rust, or `null` when nothing is pending. */
  #pending: string | null = null
  /**
   * The library path `content` was read for; `undefined` before the first
   * read, which is why that is the sentinel and not `null` — no library open
   * is a path of its own.
   */
  #loadedPath: string | null | undefined = undefined

  /**
   * Read the note of the library at `path`. Called again whenever the library
   * changes, so the panel shows the note of the library it is sitting in — and
   * a second call naming the library already loaded is an effect re-running,
   * not a switch, so it does nothing: re-reading would drop the keystrokes the
   * debounce still owes and overwrite what is on screen mid-sentence.
   */
  async load(path: string | null): Promise<void> {
    if (path === this.#loadedPath) return
    this.#loadedPath = path
    this.#cancel()
    // Anything still owed belongs to the library being left, and this call is
    // the moment another one takes its place: writing it now would put it in
    // the wrong file. `flush()` before the switch is what saves it.
    this.#pending = null
    try {
      this.content = (await noteGet()).content
      this.error = null
    } catch (cause) {
      this.error = errorText(cause)
    }
  }

  /** A keystroke: the panel shows it at once, Rust hears about it on a pause. */
  edit(next: string): void {
    this.content = next
    this.#pending = next
    this.#cancel()
    this.#timer = setTimeout(() => {
      this.#timer = null
      void this.flush()
    }, NOTE_DEBOUNCE_MS)
  }

  /**
   * Write anything still owed, now. The panel calls this on unmount and on the
   * window closing, and `LibraryMenu` before it opens another library — the
   * spec's "text typed and then not changed again is not lost by leaving the
   * panel, switching libraries or closing the window".
   */
  async flush(): Promise<void> {
    this.#cancel()
    const owed = this.#pending
    if (owed === null) return
    this.#pending = null
    try {
      await noteSet(owed)
      this.error = null
    } catch (cause) {
      // Put it back: the text is still only on screen, and the next pause or
      // the next flush is the chance to get it written.
      this.#pending = owed
      this.error = errorText(cause)
    }
  }

  #cancel(): void {
    if (this.#timer === null) return
    clearTimeout(this.#timer)
    this.#timer = null
  }
}

export const notes = new Notes()
