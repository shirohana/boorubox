// Images reach the webview through Tauri's asset protocol (design D7); the
// scope is granted for the library folder when it is opened.

import type { ImageRecord } from '@boorubox/shared'
import { convertFileSrc } from '@tauri-apps/api/core'
import { thumbnailPath } from './commands'

/**
 * The full image under `<library>/images/`. Joined with `/` on every platform:
 * the webview has no path module, and Windows accepts a forward slash inside
 * an absolute path, so this needs no per-platform separator.
 */
export function imageUrl(libraryPath: string, image: Pick<ImageRecord, 'id' | 'ext'>): string {
  return convertFileSrc(`${libraryPath}/images/${image.id}.${image.ext}`)
}

/** The thumbnail; Rust generates it if it is not on disk yet. */
export async function thumbnailUrl(id: string): Promise<string> {
  return convertFileSrc(await thumbnailPath(id))
}
