// One wrapper per Tauri command (design D12). Components import from here and
// never call `invoke` themselves (design D10), so the argument shape and the
// return type of every command have exactly one definition in the webview.
//
// Tauri v2 converts camelCase argument keys to the snake_case Rust parameters;
// the keys below are the Rust parameter names, so they must not be renamed
// without renaming the parameter in `src-tauri/src/commands.rs`.

import type {
  AppSettings,
  DeleteReport,
  ExportReport,
  ImageCounts,
  ImageRecord,
  ImportReport,
  LibraryStatus,
  ListenerStatus,
  Note,
  Rating,
  RecentLibrary,
  Rule,
  RuleInput,
  RuleListEntry,
  RulesImportReport,
  RulesRunReport,
  SearchRequest,
  SearchResult,
  TagCount,
  TagCounts,
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

/**
 * Folds the sidebar's notes panel away, or unfolds it (`notes` design D13).
 * A preference rather than session state, which is why it is stored at all.
 */
export function setNotesCollapsed(collapsed: boolean): Promise<AppSettings> {
  return invoke('set_notes_collapsed', { collapsed })
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

/**
 * Just the ids a `search` of `req` would page, in its order, with no records
 * (design D3). What the selection store resolves a range against, so its row
 * *n* and the grid's row *n* are always the same image.
 */
export function searchIds(req: SearchRequest): Promise<string[]> {
  return invoke('search_ids', { req })
}

export function imageCounts(): Promise<ImageCounts> {
  return invoke('image_counts')
}

/** Absolute path to the id's bucketed thumbnail under `<library>/.thumbs/`, generated on demand. */
export function thumbnailPath(id: string): Promise<string> {
  return invoke('thumbnail_path', { id })
}

/** Imports files and folders; progress arrives on the `import:progress` event. */
export function importPaths(paths: string[]): Promise<ImportReport> {
  return invoke('import_paths', { paths })
}

/**
 * Imports a legacy bundle's `.db` part files (`legacy-bundle-import` design D5,
 * D6). Progress arrives on the same `import:progress` event as {@link importPaths}.
 */
export function importBundle(files: string[]): Promise<ImportReport> {
  return invoke('import_bundle', { files })
}

/**
 * Replaces the image's whole tag set (design D2) and returns the row as it now
 * stands, so the caller can redraw the tile and the inspector without re-running
 * the search (design D10). A `rating:g|s|q|e` among the tags sets the rating
 * instead of being stored; Rust owns that rule, since bulk edits and auto-tag
 * rules write tags without passing through this editor (design D3).
 */
export function updateTags(id: string, tags: string[]): Promise<ImageRecord> {
  return invoke('update_tags', { id, tags })
}

/** `null` clears the rating. Anything but `g`/`s`/`q`/`e`/null is refused (D11). */
export function setRating(id: string, rating: Rating | null): Promise<ImageRecord> {
  return invoke('set_rating', { id, rating })
}

/**
 * Tag names starting with `prefix`, most used first then by name (design D12).
 * Tags already in the input are dropped by the caller, not by SQL: the input is
 * a query string, and the parser that reads one is in the webview.
 */
export function tagSuggestions(prefix: string, limit: number): Promise<TagCount[]> {
  return invoke('tag_suggestions', { prefix, limit })
}

/**
 * The sidebar's two halves for one search, over the whole result set rather
 * than the loaded pages: `limit` and `offset` are ignored (design D8).
 */
export function tagCounts(req: SearchRequest): Promise<TagCounts> {
  return invoke('tag_counts', { req })
}

/**
 * Adds `add` and removes `remove` across every id in `ids`, as one transaction
 * (design D10). The caller re-runs its current search once this resolves,
 * rather than being handed back updated rows: the selection can span pages it
 * never loaded.
 */
export function bulkUpdateTags(ids: string[], add: string[], remove: string[]): Promise<void> {
  return invoke('bulk_update_tags', { ids, add, remove })
}

/** One rating across every id in `ids` (design D10). `null` clears it. */
export function bulkSetRating(ids: string[], rating: Rating | null): Promise<void> {
  return invoke('bulk_set_rating', { ids, rating })
}

/**
 * The `limit` tags most common among `ids`, with their counts — the bulk tag
 * dialog's quick-remove pills (design D9).
 */
export function selectionTagCounts(ids: string[], limit: number): Promise<TagCount[]> {
  return invoke('selection_tag_counts', { ids, limit })
}

/**
 * Writes `ids` to a zip at `path`, emitting `export:progress` as it runs
 * (design D11–D13). The entries are dated by capture time in this webview's
 * zone: a zip timestamp has no zone and is shown as local time, and Rust
 * has no sound way to ask for the offset from a multithreaded process, so
 * it is read here, east-positive (`getTimezoneOffset` is west-positive).
 */
export function exportZip(ids: string[], path: string): Promise<ExportReport> {
  return invoke('export_zip', { ids, path, utcOffsetMinutes: -new Date().getTimezoneOffset() })
}

/**
 * Moves every id in `ids` to the trash (`trash` design D3): the file, the
 * thumbnail, the tags and the rating are untouched, so a restore hands the
 * image back exactly as it was.
 */
export function trashImages(ids: string[]): Promise<void> {
  return invoke('trash_images', { ids })
}

/** Puts every id in `ids` back in the library, reversing {@link trashImages}. */
export function restoreImages(ids: string[]): Promise<void> {
  return invoke('restore_images', { ids })
}

/**
 * Permanently deletes every id in `ids`: the record, its thumbnail and its
 * file under `images/` (`trash` design D4–D6). A file that could not be
 * removed is named in `DeleteReport.filesLeft` rather than failing the call.
 */
export function deleteForever(ids: string[]): Promise<DeleteReport> {
  return invoke('delete_forever', { ids })
}

/** Permanently deletes everything in the trash (`trash` design D7). */
export function emptyTrash(): Promise<DeleteReport> {
  return invoke('empty_trash')
}

/** How many images are in the trash — the sidebar's badge (`trash` design D11). */
export function trashCount(): Promise<number> {
  return invoke('trash_count')
}

/**
 * The library's rules, ordered by name, each carrying its pattern's validity
 * compiled at read time (`auto-tag-rules` design D4, D6).
 */
export function rulesList(): Promise<RuleListEntry[]> {
  return invoke('rules_list')
}

/**
 * Creates a rule when `rule.id` is absent, or edits the one it names; refused
 * with the reason for an empty name, no tags, or an unusable regular
 * expression (design D6).
 */
export function rulesUpsert(rule: RuleInput): Promise<Rule> {
  return invoke('rules_upsert', { rule })
}

/** Deletes a rule; idempotent, and no image it ever tagged is touched. */
export function rulesDelete(id: string): Promise<void> {
  return invoke('rules_delete', { id })
}

/**
 * Applies the current rules to every image already in the library, emitting
 * `rules:progress` as it runs (design D9, D12).
 */
export function rulesRun(): Promise<RulesRunReport> {
  return invoke('rules_run')
}

/** Writes the library's rules to `path` in the legacy JSON shape (design D10). */
export function rulesExport(path: string): Promise<void> {
  return invoke('rules_export', { path })
}

/**
 * Reads `path` and imports its rules, skipping ones already present by
 * fingerprint (design D10).
 */
export function rulesImport(path: string): Promise<RulesImportReport> {
  return invoke('rules_import', { path })
}

/**
 * The open library's one note, empty on a library that has never been written
 * to (`notes` design D3).
 */
export function noteGet(): Promise<Note> {
  return invoke('note_get')
}

/** Writes the note and answers with it as stored, stamp included. */
export function noteSet(content: string): Promise<Note> {
  return invoke('note_set', { content })
}
