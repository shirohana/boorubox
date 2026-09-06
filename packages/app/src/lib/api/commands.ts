// One wrapper per Tauri command (design D12). Components import from here and
// never call `invoke` themselves (design D10), so the argument shape and the
// return type of every command have exactly one definition in the webview.
//
// Tauri v2 converts camelCase argument keys to the snake_case Rust parameters;
// the keys below are the Rust parameter names, so they must not be renamed
// without renaming the parameter in `src-tauri/src/commands.rs`.

import type {
  ImageCounts,
  ImportReport,
  LibraryStatus,
  SearchRequest,
  SearchResult,
} from '@boorubox/shared'
import { invoke } from '@tauri-apps/api/core'

/** Opens the folder dialog. Cancelling returns the current status unchanged. */
export function pickLibrary(): Promise<LibraryStatus> {
  return invoke('pick_library')
}

export function openLibrary(path: string): Promise<LibraryStatus> {
  return invoke('open_library', { path })
}

export function libraryStatus(): Promise<LibraryStatus> {
  return invoke('library_status')
}

export function search(req: SearchRequest): Promise<SearchResult> {
  return invoke('search', { req })
}

export function imageCounts(): Promise<ImageCounts> {
  return invoke('image_counts')
}

/** Forgets an image whose file is gone; returns the status with the new count. */
export function dropImageRecord(id: string): Promise<LibraryStatus> {
  return invoke('drop_image_record', { id })
}

/** Absolute path to `<library>/.thumbs/<id>.jpg`, generated on demand. */
export function thumbnailPath(id: string): Promise<string> {
  return invoke('thumbnail_path', { id })
}

/** Imports files and folders; progress arrives on the `import:progress` event. */
export function importPaths(paths: string[]): Promise<ImportReport> {
  return invoke('import_paths', { paths })
}
