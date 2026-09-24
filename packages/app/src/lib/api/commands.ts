// One wrapper per Tauri command (design D12). Components import from here and
// never call `invoke` themselves (design D10), so the argument shape and the
// return type of every command have exactly one definition in the webview.
//
// Tauri v2 converts camelCase argument keys to the snake_case Rust parameters;
// the keys below are the Rust parameter names, so they must not be renamed
// without renaming the parameter in `src-tauri/src/commands.rs`.

import type {
  AppSettings,
  BundlePlan,
  Collection,
  CollectionCount,
  DeleteReport,
  ExportReport,
  FactsEdit,
  ImageCounts,
  ImageRecord,
  ImportReport,
  LibraryStatus,
  ListenerStatus,
  Note,
  PinTarget,
  Rating,
  RebuildReport,
  RecentLibrary,
  Rule,
  RuleInput,
  RuleListEntry,
  RulesImportReport,
  RulesRunReport,
  SearchRequest,
  SearchResult,
  Stamp,
  StampInput,
  TagCategory,
  TagCount,
  TagCounts,
  TagEditSpec,
  TagEntry,
  Theme,
  ThumbnailRef,
  ThumbsReport,
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

/**
 * Rebuilds `path`'s database from the files in its folder alone
 * (`library-sidecars` design D11, D12). Closes the library first if `path` is
 * the one open and leaves nothing open afterwards — the caller reopens it
 * through {@link openLibrary}. Progress arrives on the `library:rebuild`
 * event.
 */
export function rebuildLibrary(path: string): Promise<RebuildReport> {
  return invoke('rebuild_library', { path })
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

/**
 * Folds the sidebar's collections section away, or unfolds it
 * (`browse-feedback` design D4), copied from {@link setNotesCollapsed}.
 */
export function setCollectionsCollapsed(collapsed: boolean): Promise<AppSettings> {
  return invoke('set_collections_collapsed', { collapsed })
}

/**
 * Hides `category`'s tags from the sidebar's list, or shows them again
 * (`tag-category-visibility` design D2). One category per call: the dedupe
 * rule lives in Rust, next to the load that validates the same list.
 */
export function setTagCategoryHidden(category: TagCategory, hidden: boolean): Promise<AppSettings> {
  return invoke('set_tag_category_hidden', { category, hidden })
}

/** Clamped by Rust to `GRID_TILE_MIN`…`GRID_TILE_MAX`, never refused. */
export function setGridTileSize(size: number): Promise<AppSettings> {
  return invoke('set_grid_tile_size', { size })
}

/**
 * Whether the grid's tag footer shows outside edit mode, where the mode
 * forces it on regardless (`tile-tags-in-edit-mode` design D3).
 */
export function setShowTileTags(value: boolean): Promise<AppSettings> {
  return invoke('set_show_tile_tags', { value })
}

/**
 * Clamped by Rust to `CLICK_ZOOM_CEILING_MIN`…`CLICK_ZOOM_CEILING_MAX`, never
 * refused (`click-zoom-ceiling` design D4).
 */
export function setClickZoomCeilingPercent(percent: number): Promise<AppSettings> {
  return invoke('set_click_zoom_ceiling_percent', { percent })
}

/**
 * Whether the app reopens the remembered library automatically at launch
 * (`launch-screen` design D3). Takes effect at the next launch.
 */
export function setOpenLastOnLaunch(value: boolean): Promise<AppSettings> {
  return invoke('set_open_last_on_launch', { value })
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

/**
 * Which of `ids` a `search` of `req` still matches (design D1) — what the
 * selection prunes itself by after a write re-reads the search, so the
 * count, the strip and the next bulk action describe only images the result
 * still shows.
 */
export function matchingIds(req: SearchRequest, ids: string[]): Promise<string[]> {
  return invoke('matching_ids', { req, ids })
}

/**
 * The zero-based row `id` occupies in the order a `search` of `req` would
 * page, or `null` when `id` is not in `req`'s matched set — what a
 * click-driven search rewrite asks to keep its subject current
 * (`inspector-polish` design D2).
 */
export function searchPosition(req: SearchRequest, id: string): Promise<number | null> {
  return invoke('search_position', { req, id })
}

export function imageCounts(): Promise<ImageCounts> {
  return invoke('image_counts')
}

/**
 * The id's bucketed thumbnail under `<library>/.thumbs/`, generated on demand
 * (`one-level-buckets` design D6): its absolute path and the file's own
 * modified time, which `assets.ts`'s `thumbnailUrl` appends as a query string
 * so a regenerated file is fetched under a new URL.
 */
export function thumbnailPath(id: string): Promise<ThumbnailRef> {
  return invoke('thumbnail_path', { id })
}

/**
 * Re-renders every thumbnail at the current edge (`one-level-buckets` design
 * D4), against whichever library is open when the call starts. Progress
 * arrives on the `thumbs:progress` event; refused with a `Busy` error while a
 * pass is already running.
 */
export function regenerateThumbnails(): Promise<ThumbsReport> {
  return invoke('regenerate_thumbnails')
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
 * Reads every picked file's row count, or why it would not open, without
 * opening a library (`import-confirm` design D2, D3). `total` is the same
 * number the run's first `import:progress` will carry, computed once here so
 * the confirm screen and the run never disagree about how much work this is.
 */
export function bundlePlan(files: string[]): Promise<BundlePlan> {
  return invoke('bundle_plan', { files })
}

/**
 * Pauses the running import between items (`import-pause-cancel` design D1,
 * D2); a silent no-op with nothing running. Rust emits no event for it — the
 * webview knows the run is paused because it pressed Pause.
 */
export function importPause(): Promise<void> {
  return invoke('import_pause')
}

/** Resumes a paused import; a silent no-op with nothing running or paused. */
export function importResume(): Promise<void> {
  return invoke('import_resume')
}

/**
 * Cancels the running import between items; a silent no-op with nothing
 * running. Never waits on the library (design D5), so it answers even while
 * an import holds it mid-run.
 */
export function importCancel(): Promise<void> {
  return invoke('import_cancel')
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
 * Writes the title, page address and image address (`editable-info` design
 * D1) and returns the row as it now stands, so the caller can redraw it
 * without re-running the search (design D2) — the returned record's `account`
 * follows a changed page address for free. A non-empty address that is not
 * `http`/`https` is refused with a reason; the form stays open on the typed
 * text either way, since nothing here is written.
 */
export function updateFacts(id: string, edit: FactsEdit): Promise<ImageRecord> {
  return invoke('update_facts', { id, edit })
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
 * Applies one edit — tags added and removed, collection memberships moved,
 * the rating set, a categorised tag created — to every id in `ids`, in one
 * transaction (`stamps` design D2, replacing `bulk_update_tags`, which this
 * folds in as the `add`/`remove`-only case). Answers with the written rows:
 * a single-tile apply in edit mode shows the result without a search re-run
 * (design D2), the same reason `collectionAdd`/`collectionRemove` answer
 * this way. A caller over a selection that can span pages it never loaded
 * re-runs its own search instead of trusting these back (`browse-fixes`
 * design D1's rule, unchanged).
 */
export function applyEdit(ids: string[], edit: TagEditSpec): Promise<ImageRecord[]> {
  return invoke('apply_edit', { ids, edit })
}

/** One rating across every id in `ids` (design D10). `null` clears it. */
export function bulkSetRating(ids: string[], rating: Rating | null): Promise<void> {
  return invoke('bulk_set_rating', { ids, rating })
}

/**
 * The `limit` tags most common among `ids`, with their counts — the bulk tag
 * dialog's quick-remove pills (design D9). `names`, when given, asks for the
 * counts of exactly those tags instead of the top `limit` by frequency
 * (`tag-vocabulary` design D8, the pinned chips' tri-state read): a name the
 * selection carries zero times is simply absent from the answer, not a
 * zero-count entry. Omitted, Tauri resolves the argument to `None` on the
 * Rust side, so the bulk dialog's own call above is unchanged.
 */
export function selectionTagCounts(
  ids: string[],
  limit: number,
  names?: string[],
): Promise<TagCount[]> {
  return invoke('selection_tag_counts', { ids, limit, names })
}

/**
 * The vocabulary's exceptions (design D2): every tag that is not
 * `(general, unpinned)`, sorted by name — what `api/vocabulary.svelte.ts`
 * reads on refresh.
 */
export function tagVocabulary(): Promise<TagEntry[]> {
  return invoke('tag_vocabulary')
}

/**
 * Sets `name`'s category from its context menu (design D9), never from typed
 * text (design D4's deliberate rule). Refused when `name` is not a tag at
 * all. Answers the vocabulary as it now stands, the cheapest way for the
 * store to stay in step with a write it did not read back itself.
 */
export function setTagCategory(name: string, category: TagCategory): Promise<TagEntry[]> {
  return invoke('set_tag_category', { name, category })
}

/**
 * Places or unplaces `name` per `target` (`pinned-tag-groups` design D3); same
 * refusal, same answer shape as {@link setTagCategory}.
 */
export function setTagPinnedGroup(name: string, target: PinTarget): Promise<TagEntry[]> {
  return invoke('set_tag_pinned_group', { name, target })
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
 * The library's stamps, by creation order (`stamps` design D3) — what
 * `api/stamps.svelte.ts` reads on refresh.
 */
export function stampsList(): Promise<Stamp[]> {
  return invoke('stamps_list')
}

/**
 * Creates a stamp when `input.id` is absent, or edits the one it names;
 * refused with the reason for an empty name or an empty text (design D3).
 * `input.text` is stored exactly as sent — validate it with `domain/stamp`'s
 * `parseStamp` before calling, since Rust only refuses an empty string, never
 * an invalid grammar.
 */
export function stampsUpsert(input: StampInput): Promise<Stamp> {
  return invoke('stamps_upsert', { input })
}

/** Deletes a stamp; idempotent, and no image it was ever applied to is touched. */
export function stampsDelete(id: string): Promise<void> {
  return invoke('stamps_delete', { id })
}

/** Every collection in the library, by name (`collections` design D3). */
export function collectionList(): Promise<Collection[]> {
  return invoke('collection_list')
}

/**
 * Creates a collection named `name`; refused with the reason for a blank name
 * or a slug clash, naming the collection that already holds it (design D3).
 */
export function collectionCreate(name: string): Promise<Collection> {
  return invoke('collection_create', { name })
}

/**
 * Renames collection `id` to `name`; same refusals as {@link collectionCreate}.
 * Rewrites the library-level file and no image's sidecar (design D3).
 */
export function collectionRename(id: string, name: string): Promise<Collection> {
  return invoke('collection_rename', { id, name })
}

/**
 * Deletes collection `id`: its memberships go with it, and no image otherwise
 * changes (design D3). Deleting Favorites is allowed.
 */
export function collectionDelete(id: string): Promise<void> {
  return invoke('collection_delete', { id })
}

/**
 * Puts every id in `ids` into `collectionId`, idempotent per id (design D3).
 * Answers with the written rows for the caller to `replace` — no re-run of
 * the search (design D8).
 */
export function collectionAdd(ids: string[], collectionId: string): Promise<ImageRecord[]> {
  return invoke('collection_add', { ids, collectionId })
}

/**
 * Takes every id in `ids` out of `collectionId`, idempotent per id (design
 * D3). Answers with the written rows, the same as {@link collectionAdd}.
 */
export function collectionRemove(ids: string[], collectionId: string): Promise<ImageRecord[]> {
  return invoke('collection_remove', { ids, collectionId })
}

/**
 * Pins or unpins collection `id`, without touching any image
 * (`pinned-collections` design D3); same refusal, same answer shape as
 * {@link collectionRename}.
 */
export function setCollectionPinned(id: string, pinned: boolean): Promise<Collection[]> {
  return invoke('set_collection_pinned', { id, pinned })
}

/**
 * For exactly `collectionIds`, how many of `ids` are in each
 * (`pinned-collections` design D4): the pinned collection chip's tri-state
 * over a selection, the collection-keyed twin of {@link selectionTagCounts}'s
 * `names` filter.
 */
export function selectionCollectionCounts(
  ids: string[],
  collectionIds: string[],
): Promise<CollectionCount[]> {
  return invoke('selection_collection_counts', { ids, collectionIds })
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
