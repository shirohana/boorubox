// `app-update`: the app learns a newer version exists, tells the user, and
// installs it only after they agree. `tauri-plugin-updater` does the
// mechanism — fetch the manifest, compare versions, download, verify the
// minisign signature, install — and `tauri-plugin-process` does the restart;
// this store owns the conversation around them (design D1). No Rust command
// of our own: `check()`, `Update#download()`/`Update#install()` and
// `relaunch()` are the plugins' JS guest bindings, called straight from here.

import type { Update } from '@tauri-apps/plugin-updater'
import { relaunch } from '@tauri-apps/plugin-process'
import { check } from '@tauri-apps/plugin-updater'
import { errorText } from './errors'

export type CheckOutcome = 'available' | 'current' | 'failed'

/**
 * The one update the app has learned about this session, and the session's
 * answer to it. A singleton like `library`/`settings`: the launch check runs
 * once from the root layout, and the confirmation it may open has to survive
 * a navigation between routes.
 */
export class AppUpdate {
  /**
   * The update on offer, once a check has found one. `$state.raw`: `Update`
   * is a Tauri `Resource` wrapping a Rust-side handle and its own `close()`
   * — proxying it the way `$state` proxies a plain object would be wrapping
   * lifecycle machinery that already manages itself, for no reactive field
   * anything here reads through the proxy.
   *
   * Cleared (and its handle closed) on decline, on a failed install, and
   * whenever a fresh check finds a different one to hold instead — never
   * left to sit forever once it stops being the answer, which would strand
   * a Rust-side resource id and show a stale "Update to…" nothing can
   * re-check without a restart.
   */
  available = $state.raw<Update | null>(null)
  /**
   * Whether {@link available} has already been downloaded. A refusal that
   * lands *after* the download (design D6: work started while it ran) keeps
   * the bytes rather than re-fetching them on the next attempt — see
   * {@link install}.
   */
  downloaded = $state(false)
  /** Whether the confirmation is on screen — the layout's one dialog reads this. */
  promptOpen = $state(false)
  /**
   * Declined this session (design D5): once set, a later discovery does not
   * reopen the prompt on its own — the Settings "Update to…" control is
   * where it opens back up, deliberately, from then on. Session-scoped and
   * never reset by clearing {@link available}: it is the session's answer to
   * "have I already been asked", not a property of any one update.
   */
  declined = $state(false)
  /** An explicit (Settings) check in flight. The launch check never sets this. */
  checking = $state(false)
  /**
   * The explicit check's last failure, for Settings to show (design D4). A
   * launch check's failure never reaches here — it is silent by spec.
   */
  lastCheckError = $state<string | null>(null)
  /** An install is downloading/verifying/installing. */
  installing = $state(false)
  /** Why the last install attempt did not finish, or was refused. */
  installError = $state<string | null>(null)

  #launchChecked = false
  /**
   * The in-flight explicit check, if any, so a second call while one is
   * running answers with the same result instead of starting a race.
   */
  #pendingCheck: Promise<CheckOutcome> | null = null

  /**
   * The launch check (design D4): fire-and-forget, silent about everything
   * except an available update. Never awaited by its caller — nothing may
   * delay the window appearing (spec `app-update`, "Offline at launch").
   */
  async checkOnLaunch(): Promise<void> {
    if (this.#launchChecked) return
    this.#launchChecked = true
    try {
      const found = await check()
      if (found) this.#found(found)
    } catch {
      // Silent by spec: a person opening an image library did not ask about
      // the network, and an error toast on every offline launch is noise
      // that teaches people to ignore toasts.
    }
  }

  /**
   * The Settings check (design D4): explicit, so it answers in all three
   * cases — the caller renders `'current'`/`'failed'`, and `'available'`
   * is already on screen via {@link available} and {@link promptOpen}.
   */
  checkNow(): Promise<CheckOutcome> {
    // Same shape as `install`'s re-entrancy guard: the button that calls
    // this disables itself on `checking`, but only after the render that
    // follows the click, so a second click in between must not start a
    // second network round trip — it gets the first one's answer instead.
    this.#pendingCheck ??= this.#checkNow().finally(() => {
      this.#pendingCheck = null
    })
    return this.#pendingCheck
  }

  async #checkNow(): Promise<CheckOutcome> {
    this.checking = true
    this.lastCheckError = null
    try {
      const found = await check()
      if (found) {
        this.#found(found)
        return 'available'
      }
      return 'current'
    } catch (cause) {
      this.lastCheckError = errorText(cause)
      return 'failed'
    } finally {
      this.checking = false
    }
  }

  #found(update: Update): void {
    this.#discard()
    this.available = update
    if (!this.declined) this.promptOpen = true
  }

  /** Closes and drops whatever is on offer, if anything. */
  #discard(): void {
    this.available?.close().catch(() => {})
    this.available = null
    this.downloaded = false
  }

  /**
   * Reopens the confirmation for an update already found — the Settings
   * "Update to…" control, for someone who declined and changed their mind.
   * A no-op with nothing on offer.
   */
  reopen(): void {
    if (this.available) this.promptOpen = true
  }

  /**
   * Declining leaves the running version installed (nothing here ever
   * downloaded anything on its own) and does not repeat the prompt for the
   * rest of the session (spec `app-update`, "Declining"). The update itself
   * is dropped too: holding it would show a stale "Update to…" with no way
   * back to Check, and outlive a release that got pulled after it was found.
   */
  decline(): void {
    this.#discard()
    this.declined = true
    this.promptOpen = false
    this.installError = null
  }

  /**
   * Downloads, verifies and installs the update on offer, then restarts.
   *
   * `busyWith` names what is running, if anything — called twice, so it must
   * read live state each time rather than close over a snapshot: once before
   * downloading (the cheap, obvious refusal) and again immediately before
   * {@link Update.install} (design D6: "a real refusal, not a delay-and-hope"
   * — `download()` alone can run for minutes, long enough for an import or a
   * capture to start after the first check and before the point of no
   * return). This is why `download()` and `install()` are called separately
   * rather than through the plugin's combined `downloadAndInstall()`: the
   * combined call gives JS no hook between "downloaded" and "installed", and
   * on Windows `install()` itself is what exits the process — there is no
   * install-without-restarting on the platform this app actually ships for,
   * so the second check has to land before `install()` is called at all, not
   * merely before {@link relaunch}.
   *
   * A refusal after the download completes does not discard it: `available`
   * and {@link downloaded} are left as they are, so calling this again once
   * the busy work has cleared installs from what is already on disk instead
   * of fetching it twice.
   */
  async install(busyWith: () => string | null): Promise<void> {
    if (this.installing) return
    const before = busyWith()
    if (before) {
      this.installError = `Can't update yet — ${before}.`
      return
    }
    const update = this.available
    if (!update) return
    this.installing = true
    this.installError = null
    try {
      if (!this.downloaded) {
        await update.download()
        this.downloaded = true
      }
      const after = busyWith()
      if (after) {
        // Design D6, spec "Update during a migration": leave the work
        // alone and do not restart — but the download is not thrown away,
        // so the next Install (once it is free) installs at once.
        this.installing = false
        this.installError
          = `Downloaded, but can't install yet — ${after}. Install again once it's done.`
        return
      }
      // The plugin verifies the minisign signature before this resolves and
      // rejects a tampered artifact rather than installing it (design D1;
      // spec `app-update`, "Tampered artifact") — whatever it throws is
      // exactly the refusal to report, via the catch below.
      await update.install()
      // Windows: `install()` has already exited the process by the time
      // this resolves, so the call below never runs. macOS/Linux: it has
      // not, and this is what brings the new version up.
      await relaunch()
    } catch (cause) {
      this.#discard()
      this.installError = errorText(cause)
      this.installing = false
    }
  }
}

export const appUpdate = new AppUpdate()
