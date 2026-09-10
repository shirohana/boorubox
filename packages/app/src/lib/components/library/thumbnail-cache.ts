// One `thumbnail_path` round trip per image for the life of the window. The
// grid recycles its cards while scrolling and the inspector's selection strip
// draws some of the same images again, so without this every pass over a row
// would ask Rust for a path it already has.

import { thumbnailUrl } from '$lib/api'

const cache = new Map<string, string>()

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
