/**
 * The settings pages, in the order the nav lists them: General, Library,
 * Artists, Rules, Stamps, Pinned tags, Booru, Keyboard, About (Pinned tags' place: owner,
 * 2026-10-02). General holds the settings set once and then left alone — how the app looks and
 * how it listens for captures — because a page holding a single field is not
 * worth a place in the nav (spec `app-frame`, "Settings is a set of pages
 * with a nav"). `path` is a literal route id, `as const`, so the nav, the
 * redirect and the fallback all read this one list.
 *
 * No `$app` import here: callers turn `path` into an href with `resolve()`,
 * which keeps this module testable under plain vitest.
 */
export const SETTINGS_PAGES = [
  { slug: 'general', label: 'General', path: '/settings/general' },
  { slug: 'library', label: 'Library', path: '/settings/library' },
  { slug: 'artists', label: 'Artists', path: '/settings/artists' },
  { slug: 'rules', label: 'Rules', path: '/settings/rules' },
  { slug: 'stamps', label: 'Stamps', path: '/settings/stamps' },
  { slug: 'pinned-tags', label: 'Pinned tags', path: '/settings/pinned-tags' },
  { slug: 'booru', label: 'Booru', path: '/settings/booru' },
  { slug: 'keyboard', label: 'Keyboard', path: '/settings/keyboard' },
  { slug: 'about', label: 'About', path: '/settings/about' },
] as const

export type SettingsPage = (typeof SETTINGS_PAGES)[number]
export type SettingsPageSlug = SettingsPage['slug']

const STORAGE_KEY = 'boorubox.settings.lastPage'

const GENERAL = SETTINGS_PAGES[0]

/** The named page, or General for `null`, an empty string or an unknown slug. */
export function settingsPageFor(stored: string | null): SettingsPage {
  return SETTINGS_PAGES.find((page) => page.slug === stored) ?? GENERAL
}

/**
 * The last settings page shown on this machine (`settings-pages` design D3: view state kept in
 * `localStorage`, not `AppSettings` — its loss costs one click). A read that
 * throws (a disabled or full storage) answers General rather than failing
 * the page.
 */
export function lastSettingsPage(): SettingsPage {
  try {
    return settingsPageFor(localStorage.getItem(STORAGE_KEY))
  } catch {
    return GENERAL
  }
}

/** Remembers `slug` for `lastSettingsPage()`. A write that throws is dropped. */
export function rememberSettingsPage(slug: SettingsPageSlug): void {
  try {
    localStorage.setItem(STORAGE_KEY, slug)
  } catch {
    // Dropped (`settings-pages` design D3): a page shown is not worth failing over.
  }
}
