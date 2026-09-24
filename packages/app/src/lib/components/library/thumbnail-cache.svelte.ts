// One `thumbnail_path` round trip per image for the life of the window. The
// grid recycles its cards while scrolling and the inspector's selection strip
// draws some of the same images again, so without this every pass over a row
// would ask Rust for a path it already has.

// Imported from the module directly, not the `$lib/api` barrel: the barrel
// also re-exports `thumbsRegenerate`, which imports `forgetAll` from this
// file, and a barrel import here would make that a cycle.
import { thumbnailUrl } from '$lib/api/assets'

/* eslint-disable-next-line svelte/prefer-svelte-reactivity --
   `cachedThumbnail`/`thumbnail` are plain functions, called imperatively from
   inside an effect rather than read as a tracked expression — a `SvelteMap`
   would not make `forgetAll()`'s `.clear()` wake an effect that only ever
   calls `.get(id)` through a function boundary. `epoch` below is the signal
   an effect actually depends on. */
const cache = new Map<string, string>()

// Bumped by `forgetAll()`. Clearing the `Map` wakes nothing on its own: an
// `$effect` that closes over an id and never reads the `Map` itself has
// nothing reactive to re-run on, so a card already on screen would never ask
// again. An effect that also reads `cacheEpoch()` gets a signal it does
// re-run on, and can tell "still loading" apart from "cleared and reloading"
// by comparing the id across the bump.
let epoch = $state(0)

/** The current cache generation, so an effect can depend on a `forgetAll()`. */
export function cacheEpoch(): number {
  return epoch
}

/** What is already known, so a redraw paints without a flash of empty tile. */
export function cachedThumbnail(id: string): string | null {
  return cache.get(id) ?? null
}

/** The thumbnail url, asked for at most once per image. */
export async function thumbnail(id: string): Promise<string> {
  const known = cache.get(id)
  if (known !== undefined) return known
  const url = await thumbnailUrl(id)
  cache.set(id, url)
  return url
}

/**
 * Drops every cached url, so the next `thumbnail` call for each id asks Rust
 * again and gets the new file's version (`one-level-buckets` design D6).
 * Called by `thumbsRegenerate` when a regeneration pass finishes.
 */
export function forgetAll(): void {
  cache.clear()
  epoch++
}
