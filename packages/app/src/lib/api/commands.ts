// One wrapper per Tauri command (design D12). Components import from here and
// never call `invoke` themselves (design D10), so the argument shape and the
// return type of every command have exactly one definition in the webview.
//
// Tauri v2 converts camelCase argument keys to the snake_case Rust parameters;
// the keys below are the Rust parameter names, so they must not be renamed
// without renaming the parameter in `src-tauri/src/commands.rs`.

import type {
  AppSettings,
  ImageCounts,
  ImportReport,
  LibraryStatus,
  ListenerStatus,
  RecentLibrary,
  SearchRequest,
  SearchResult,
  Theme,
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

/** Closes the open library; the folder stays first in the recent list (D3). */
export function closeLibrary(): Promise<LibraryStatus> {
  return invoke('close_library')
}

/** The start screen's list. `available` is checked on every call (design D4). */
export function recentLibraries(): Promise<RecentLibrary[]> {
  return invoke('recent_libraries')
}

/** Drops one entry and returns what is left; the folder is untouched. */
export function forgetRecent(path: string): Promise<RecentLibrary[]> {
  return invoke('forget_recent', { path })
}

/** Shows the open library's folder in the file manager. */
export function revealLibrary(): Promise<void> {
  return invoke('reveal_library')
}

/** The theme and the tile size; the listener port is on `LibraryStatus` (D5). */
export function appSettings(): Promise<AppSettings> {
  return invoke('app_settings')
}

export function setTheme(theme: Theme): Promise<AppSettings> {
  return invoke('set_theme', { theme })
}

/** Clamped by Rust to `GRID_TILE_MIN`…`GRID_TILE_MAX`, never refused. */
export function setGridTileSize(size: number): Promise<AppSettings> {
  return invoke('set_grid_tile_size', { size })
}

/**
 * Rebinds the capture listener (design D6). The port is stored whether or not
 * it binds, so a failure comes back as a stopped listener with its reason, not
 * as a rejection.
 */
export function setListenerPort(port: number): Promise<ListenerStatus> {
  return invoke('set_listener_port', { port })
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
