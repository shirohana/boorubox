import type { AppSettings, Theme } from '@boorubox/shared'
import {
  appSettings,
  setClickZoomCeilingPercent,
  setCollectionsCollapsed,
  setGridTileSize,
  setNotesCollapsed,
  setOpenLastOnLaunch,
  setTheme,
} from './commands'
import { errorText } from './errors'

/**
 * The webview's one copy of `AppSettings`, in the shape of `library.svelte.ts`:
 * the root layout loads it once at startup and waits for it before painting
 * anything (design D12), and every writing command answers with the whole new
 * settings object, so a write updates this copy without a second read.
 */
class Settings {
  /** `null` until the first `app_settings()` call answers. */
  current = $state<AppSettings | null>(null)
  /** Why the settings could not be read at all. */
  error = $state<string | null>(null)
  #asked = false

  async load(): Promise<void> {
    if (this.#asked) return
    this.#asked = true
    try {
      this.current = await appSettings()
    } catch (error) {
      this.error = errorText(error)
    }
  }

  /** Rejects on failure: the caller is a control the user just touched. */
  async setTheme(theme: Theme): Promise<void> {
    this.current = await setTheme(theme)
  }

  /** Rust clamps the size, so the stored value is what comes back, not `size`. */
  async setGridTileSize(size: number): Promise<void> {
    this.current = await setGridTileSize(size)
  }

  /**
   * Rust clamps the percent, so the stored value is what comes back, not
   * `percent` (`click-zoom-ceiling` design D4).
   */
  async setClickZoomCeilingPercent(percent: number): Promise<void> {
    this.current = await setClickZoomCeilingPercent(percent)
  }

  /** Folds the sidebar's notes panel away, or unfolds it (`notes` design D13). */
  async setNotesCollapsed(collapsed: boolean): Promise<void> {
    this.current = await setNotesCollapsed(collapsed)
  }

  /**
   * Folds the sidebar's collections section away, or unfolds it
   * (`browse-feedback` design D4).
   */
  async setCollectionsCollapsed(collapsed: boolean): Promise<void> {
    this.current = await setCollectionsCollapsed(collapsed)
  }

  /**
   * Whether the app reopens the remembered library at launch, or waits on the
   * start screen instead (`launch-screen` design D3). Takes effect at the next
   * launch, not this one.
   */
  async setOpenLastOnLaunch(value: boolean): Promise<void> {
    this.current = await setOpenLastOnLaunch(value)
  }
}

export const settings = new Settings()
