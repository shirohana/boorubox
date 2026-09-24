// Images reach the webview through Tauri's asset protocol (design D7); the
// scope is granted for the library folder when it is opened.

import type { ImageRecord } from '@boorubox/shared'
import { convertFileSrc } from '@tauri-apps/api/core'
import { thumbnailPath } from './commands'

/**
 * The full image, from the record's own `file` (design D2). Joined with `/`
 * on every platform: the webview has no path module, and Windows accepts a
 * forward slash inside an absolute path, so this needs no per-platform
 * separator.
 */
export function imageUrl(libraryPath: string, image: Pick<ImageRecord, 'file'>): string {
  return convertFileSrc(`${libraryPath}/${image.file}`)
}

/**
 * The thumbnail; Rust generates it if it is not on disk yet. `?v=` carries the
 * file's own modified time (`one-level-buckets` design D6): `thumbnail_path`
 * always answers the same path for a given id, so without something in the
 * URL that changes when the file does, the browser's own cache would keep
 * showing the old thumbnail after a regeneration.
 */
export async function thumbnailUrl(id: string): Promise<string> {
  const { path, version } = await thumbnailPath(id)
  return `${convertFileSrc(path)}?v=${version}`
}
