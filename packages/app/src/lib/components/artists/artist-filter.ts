import type { ArtistEntry } from '@boorubox/shared'

/**
 * Narrows `entries` to the ones whose tag or one of whose URLs contains
 * `query`, ignoring case (`settings-pages` design D8). An empty or
 * all-whitespace query answers every entry, in order — the field's default
 * shows the whole list.
 */
export function filterArtists(entries: ArtistEntry[], query: string): ArtistEntry[] {
  const needle = query.trim().toLowerCase()
  if (!needle) return entries
  return entries.filter(
    (entry) =>
      entry.tag.toLowerCase().includes(needle)
      || entry.urls.some((url) => url.toLowerCase().includes(needle)),
  )
}
