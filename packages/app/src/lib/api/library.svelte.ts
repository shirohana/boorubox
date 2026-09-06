import type { LibraryStatus } from '@boorubox/shared'
import { libraryStatus } from './commands'
import { errorText } from './errors'

/**
 * The webview's one copy of `LibraryStatus`. The root layout loads it once at
 * startup and the gate to `/setup` reads it; every command that can change the
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

  set(status: LibraryStatus): void {
    this.status = status
    this.error = null
  }
}

export const library = new Library()
