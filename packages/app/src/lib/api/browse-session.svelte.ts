// Session state that a route remount must not reset (`browse-feedback` design
// D1): each view's result set, whether the inspector is shown, and the
// viewer's last mode. `LibraryScreen` reads all three from here instead of
// owning them, so a trip to the trash, the import or the settings screen and
// back finds the library exactly as it was left.

import { SearchResults } from './search.svelte'

export class BrowseSession {
  /** Session state, not a setting (design D9 non-goal): open until hidden. */
  inspectorOpen = $state(true)
  /**
   * The viewer's last mode, kept across a close (`inspector-polish` design
   * D6): the user who opened the panel wants the panel back next time.
   */
  lightboxMode = $state<'gallery' | 'inspect'>('gallery')

  /*
   eslint-disable-next-line svelte/prefer-svelte-reactivity --
   Plain Map on purpose: it holds at most the library's and the trash's
   instance, each created once and never removed, so nothing needs to react
   to the map's own shape — only the `SearchResults` it hands back are
   reactive, through their own `$state` fields.
  */
  #results = new Map<'library' | 'trash', SearchResults>()

  /** One `SearchResults` per view, created on first ask and kept for the run. */
  resultsFor(view: 'library' | 'trash'): SearchResults {
    let results = this.#results.get(view)
    if (!results) {
      results = new SearchResults(view)
      this.#results.set(view, results)
    }
    return results
  }
}

export const browseSession = new BrowseSession()
