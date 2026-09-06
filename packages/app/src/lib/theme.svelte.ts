// Design D12: `app.css` already defines the whole dark palette under `.dark`,
// so painting a theme is toggling one class on the root element. Nothing else
// — no palette in JS, no CSS-only `prefers-color-scheme` query, which could not
// express "light while the system is dark".

import type { Theme } from '@boorubox/shared'

const DARK_QUERY = '(prefers-color-scheme: dark)'

/**
 * Kept pure and separate from the DOM work so it can be tested: jsdom has no
 * `matchMedia`, and the three settings against both system appearances are the
 * whole of the decision.
 */
export function resolveTheme(setting: Theme, systemDark: boolean): 'light' | 'dark' {
  if (setting === 'system') return systemDark ? 'dark' : 'light'
  return setting
}

/**
 * Paints `setting` now and, while it is `system`, repaints when the OS switches
 * appearance. Returns the teardown: run it before applying another setting, or
 * a listener left over from `system` repaints over the explicit choice.
 */
export function applyTheme(setting: Theme): () => void {
  const media = window.matchMedia(DARK_QUERY)
  const paint = () => {
    const dark = resolveTheme(setting, media.matches) === 'dark'
    document.documentElement.classList.toggle('dark', dark)
  }

  paint()
  if (setting !== 'system') return () => {}

  media.addEventListener('change', paint)
  return () => media.removeEventListener('change', paint)
}
