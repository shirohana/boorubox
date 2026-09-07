import type { LibraryStatus, ListenerStatus } from '@boorubox/shared'
import { libraryStatus } from './commands'
import { errorText } from './errors'

/**
 * The webview's one copy of `LibraryStatus`. The root layout loads it once at
 * startup and the gate to `/start` reads it; every command that can change the
 * status returns the new one, so hand that to `set` instead of asking Rust
 * again.
 */
class Library {
  /** `null` until the first `library_status()` call answers. */
  status = $state<LibraryStatus | null>(null)
  /** Why the status could not be read at all — not a missing library folder. */
  error = $state<string | null>(null)
  #asked = false

  async load(): Promise<void> {
    if (this.#asked) return
    this.#asked = true
    try {
      this.status = await libraryStatus()
    } catch (error) {
      this.error = errorText(error)
    }
  }

  /**
   * Re-reads the status from Rust, for a change no command in the webview made
   * — a capture arriving over the listener is the only one so far.
   */
  async refresh(): Promise<void> {
    try {
      this.status = await libraryStatus()
    } catch {
      // The status on screen is a moment old, not wrong. Replacing the frame
      // with an error because one refresh failed would be the larger loss.
    }
  }

  set(status: LibraryStatus): void {
    this.status = status
    this.error = null
  }

  /**
   * `set_listener_port` answers with the listener alone — the rest of the
   * status did not change, and asking for the whole thing again would read the
   * image count a capture may have moved in between.
   */
  setListener(listener: ListenerStatus): void {
    if (this.status) this.status = { ...this.status, listener }
  }
}

export const library = new Library()
