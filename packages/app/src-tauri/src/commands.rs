//! The `#[tauri::command]` surface (design D12) — argument marshalling only.
//! Every parameter name here is part of the contract: `tauri::command` derives
//! the JSON key the webview sends from it, so renaming one breaks `lib/api/`.
//!
//! The commands are generic over the runtime so the tests below can drive them
//! on `tauri::test`'s mock runtime; `generate_handler!` fills `R` in with the
//! real one.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use tauri::{AppHandle, Emitter, Manager, Runtime, State};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;

use crate::booru::credentials::Credentials;
use crate::error::{AppError, Result};
use crate::library::{self, Library, SharedLibrary, with_library, with_library_if_open};
use crate::model::{
    AppSettings, BooruConnectionTest, BooruSite, BooruUploadForm, BooruUploadOutcome, BundlePlan,
    CLICK_ZOOM_CEILING_MAX, CLICK_ZOOM_CEILING_MIN, Collection, DeleteReport, ExportProgress,
    ExportReport, FactsEdit, GRID_TILE_MAX, GRID_TILE_MIN, ImageCounts, ImageRecord, ImportReport,
    LibraryStatus, ListenerStatus, Note, PostRef, RebuildProgress, RebuildReport, RecentLibrary,
    Rule, RuleInput, RuleListEntry, RulesImportReport, RulesRunReport, SearchRequest, SearchResult,
    SidecarsProgress, Stamp, StampInput, TagCategory, TagCount, TagCounts, TagEditSpec, TagEntry,
    Theme,
};
use crate::settings::Settings;
use crate::{
    AppState, OpenFailureKind, VERSION, booru, collections, db, export, facts, from_tauri, http,
    import, ingest, lock, maintenance, notes, query, recover, rules, settings, stamps, tags,
    thumbs, trash,
};

/// Progress while `import_paths` runs. The webview subscribes under this name;
/// change it here and the import screen sits at zero until the run finishes.
const IMPORT_PROGRESS_EVENT: &str = "import:progress";

/// A capture the listener stored, as the webview hears about it. The library
/// screen re-runs its search on this; change the name here and an image saved
/// from the browser stays invisible until something else reloads the grid.
pub const CAPTURE_STORED_EVENT: &str = "capture:stored";

/// A capture the extension says is coming, relayed with the `CaptureMeta` it
/// announced. The library screen puts a placeholder up on this and takes it
/// down on one of the two below, so all three names are one contract with
/// `lib/api/events.ts`.
pub const CAPTURE_PENDING_EVENT: &str = "capture:pending";

/// An announced capture that will not arrive, stored or refused. Every answer
/// to `POST /captures` settles the announcement (design D6); without this the
/// placeholder would hang until its timeout.
pub const CAPTURE_WITHDRAWN_EVENT: &str = "capture:withdrawn";

/// Progress while `export_zip` runs, emitted once per id (`selection-and-bulk`
/// design D13), mirroring `IMPORT_PROGRESS_EVENT`.
const EXPORT_PROGRESS_EVENT: &str = "export:progress";

/// Progress while `rules_run` runs, the same `{ done, total }` shape as
/// `IMPORT_PROGRESS_EVENT` (`auto-tag-rules` design D12).
const RULES_PROGRESS_EVENT: &str = "rules:progress";

/// Progress while `rebuild_library` runs, shown where no library is open
/// (`library-sidecars` design D13) — its own event, not `import:progress`.
const REBUILD_PROGRESS_EVENT: &str = "library:rebuild";

/// Progress while `open_into_state`'s backfill writes the sidecars a library
/// is missing, shown as one tile in the pending-work band (`library-sidecars`
/// design D7, D13, spec `pending-work`).
const SIDECARS_PROGRESS_EVENT: &str = "library:sidecars";

/// Emitted once the launch-time open of the remembered library settles,
/// success or failure alike (`launch-screen` design D1). Carries no payload:
/// `library_status` is the one truth, and this event only tells the webview
/// to re-read it — the same path a capture or a failure already reports
/// through, so nothing about that path has to change for the launch case.
pub const LIBRARY_OPENED_EVENT: &str = "library:opened";

/// Run `work` against the shared library on a blocking thread.
///
/// Tauri runs a synchronous command on the app's main thread, and on macOS that
/// thread is also the window's run loop: a command that takes the library mutex
/// there stops the window scrolling and repainting for as long as a capture or
/// an import holds it (design D13). Every command that reads or writes through
/// the open library goes through here, so none of them can put that back.
///
/// The two that swap the library itself — `open_library` and `close_library`,
/// through `open_into_state` — still take the mutex where they stand: they are
/// a deliberate move between libraries rather than work on the open one, and
/// with the import locking per file the wait is one image at most.
///
/// `work` is a plain blocking closure, and the library is reached through its
/// `Arc` rather than through `State`: it outlives this call's borrow of the
/// managed state, and nothing holding a `std::sync::Mutex` may `.await` (see
/// `http::captures::store`).
async fn off_main_thread<T: Send + 'static>(
    library: &SharedLibrary,
    work: impl FnOnce(&SharedLibrary) -> Result<T> + Send + 'static,
) -> Result<T> {
    let library = library.clone();
    tauri::async_runtime::spawn_blocking(move || work(&library))
        .await
        .map_err(from_tauri)?
}

/// `off_main_thread` for the commands that need a library open, and hold it for
/// exactly the one piece of work they do.
async fn with_library_off_main_thread<T: Send + 'static>(
    library: &SharedLibrary,
    work: impl FnOnce(&Library) -> Result<T> + Send + 'static,
) -> Result<T> {
    off_main_thread(library, |library| with_library(library, work)).await
}

/// Ask for a folder and open it. Cancelling returns the status unchanged.
#[tauri::command]
pub async fn pick_library<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
) -> Result<LibraryStatus> {
    // Async, and the dialog is driven by its callback: tauri-plugin-dialog's
    // blocking pickers wait on the thread that owns the UI, which is the thread
    // that would have to show the dialog.
    match pick_folder(&app).await {
        Some(path) => open_and_remember(&app, &state, &path).await,
        None => status(&state).await,
    }
}

#[tauri::command]
pub async fn open_library<R: Runtime>(
    path: String,
    app: AppHandle<R>,
    state: State<'_, AppState>,
) -> Result<LibraryStatus> {
    open_and_remember(&app, &state, Path::new(&path)).await
}

#[tauri::command]
pub async fn library_status(state: State<'_, AppState>) -> Result<LibraryStatus> {
    status(&state).await
}

/// Close the open library and forget the remembered path, so the next launch
/// starts on the start screen rather than reporting a folder as missing — which
/// is what `status` would derive from a path with no library open (design D3).
/// The folder is not forgotten: it is the first entry of the recent list.
#[tauri::command]
pub async fn close_library<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
) -> Result<LibraryStatus> {
    // A paused run cannot be allowed to hold the app open (`import-pause-
    // cancel` design D7): cancel it first, waking a thread parked at the
    // checkpoint, before the library it was importing into disappears out
    // from under it.
    cancel_running_import(&state);
    // Dropping the `Library` closes its connection. Nothing may hold a second
    // one on that database (§7).
    *lock(&state.library) = None;
    write_settings(&app, &state, |settings| settings.library_path = None)?;
    status(&state).await
}

/// Rebuild `path`'s database from the sidecars in its folder alone
/// (`library-sidecars` design D11, D12), emitting `library:rebuild` as it
/// goes. Closes the library first if `path` is the one open, and leaves
/// nothing open either way — the caller reopens it through [`open_library`]
/// once the report is on screen, the two callers being the start screen (a
/// damaged remembered library) and Settings (the one already open).
#[tauri::command]
pub async fn rebuild_library<R: Runtime>(
    path: String,
    app: AppHandle<R>,
    state: State<'_, AppState>,
) -> Result<RebuildReport> {
    let root = PathBuf::from(path);
    let target = root.clone();
    // A rebuild tears the library down and renames a database onto its folder,
    // so it is one of the "every place that tears down or replaces
    // `state.library`" `cancel_running_import` names: an import left running
    // would go on writing rows into the database about to be moved aside, and
    // those rows would come back only as whatever sidecars happened to land
    // before the walk reached their bucket.
    cancel_running_import(&state);
    off_main_thread(&state.library, move |shared| {
        {
            let mut guard = shared
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if guard.as_ref().is_some_and(|open| open.paths.root == target) {
                *guard = None;
            }
        }
        let paths = library::LibraryPaths { root };
        recover::rebuild(&paths, &mut |done, total| {
            // A dropped tick is a progress bar that skips a number; the
            // rebuild itself is unaffected, so it is not worth failing over.
            let _ = app.emit(REBUILD_PROGRESS_EVENT, RebuildProgress { done, total });
        })
    })
    .await
}

/// Cancel whatever import is running, waking a thread parked at the
/// checkpoint (`import-pause-cancel` design D7). Every place that tears down
/// or replaces `state.library` calls this first — `close_library` as much as
/// `open_into_state`. A library *switch* is exactly as much a teardown of the
/// old library as a close is: the run holds no lock at its checkpoint (design
/// D13), so nothing about swapping `state.library` out from under it stops it
/// on its own — left uncancelled, it would carry on writing whatever is left
/// of its 25,000 rows into whichever library happens to be open when its next
/// item lands.
fn cancel_running_import(state: &AppState) {
    if let Some(control) = lock(&state.import_control).clone() {
        control.cancel();
    }
}

/// The start screen's list. Availability is read here rather than at startup:
/// ten `stat` calls on a volume that is not mounted would hold the window back,
/// and nothing needs the answer until the list is drawn (design D4).
#[tauri::command]
pub fn recent_libraries(state: State<'_, AppState>) -> Vec<RecentLibrary> {
    lock(&state.settings)
        .recent_libraries
        .iter()
        .map(|path| recent_entry(path))
        .collect()
}

/// Drop an entry from the recent list. Nothing inside the folder is touched.
#[tauri::command]
pub fn forget_recent<R: Runtime>(
    path: String,
    app: AppHandle<R>,
    state: State<'_, AppState>,
) -> Result<Vec<RecentLibrary>> {
    write_settings(&app, &state, |settings| {
        settings.forget_recent(Path::new(&path));
    })?;
    Ok(recent_libraries(state))
}

/// Move the capture listener to `port` without a restart (design D6). The port
/// is stored whether or not it bound, so the field shows what the user chose
/// when they come back to it (spec `capture-ingest`).
#[tauri::command]
pub async fn set_listener_port<R: Runtime>(
    port: u16,
    app: AppHandle<R>,
    state: State<'_, AppState>,
) -> Result<ListenerStatus> {
    let status = rebind_listener(&app, &state, port).await;
    write_settings(&app, &state, |settings| settings.port = port)?;
    Ok(status)
}

/// Stop whatever is listening and bind `port`, leaving the outcome in the state.
/// A port that will not bind is a state, not an error: the app opens, and keeps
/// working, either way. `setup` binds the first listener through this too.
pub async fn rebind_listener<R: Runtime>(
    app: &AppHandle<R>,
    state: &AppState,
    port: u16,
) -> ListenerStatus {
    // Taken out of the state before the `await`, not inside an `if let`: the
    // guard would otherwise be held across it, and a `MutexGuard` is not `Send`.
    let running = lock(&state.listener_shutdown).take();
    if let Some(handle) = running {
        handle.stop().await;
    }

    let (status, handle) = http::start(state.http_state(app), port).await;
    *lock(&state.listener_shutdown) = handle;
    *lock(&state.listener) = status.clone();
    status
}

/// Show the open library's folder in the file manager, so it can be backed up
/// or copied (docs/requirements.md §6).
#[tauri::command]
pub async fn reveal_library<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
) -> Result<()> {
    let root =
        with_library_off_main_thread(&state.library, |library| Ok(library.paths.root.clone()))
            .await?;
    app.opener().reveal_item_in_dir(root).map_err(from_tauri)
}

fn recent_entry(path: &Path) -> RecentLibrary {
    let database = library::database_path(path);
    RecentLibrary {
        path: path.display().to_string(),
        // A library folder with no basename is `/`, which nobody picks; naming
        // it by its path is better than an empty row.
        name: path.file_name().map_or_else(
            || path.display().to_string(),
            |name| name.to_string_lossy().into_owned(),
        ),
        // Opened, not merely stat'ed: a file the user cannot read would
        // otherwise offer an entry that fails the moment it is clicked.
        available: database.is_file() && std::fs::File::open(&database).is_ok(),
    }
}

/// The two preferences the webview paints with (design D5). The listener port
/// is not among them: it is on `LibraryStatus.listener`, and a second copy would
/// be a second thing to keep in step.
#[tauri::command]
pub fn app_settings(state: State<'_, AppState>) -> AppSettings {
    AppSettings::from(&*lock(&state.settings))
}

#[tauri::command]
pub fn set_theme<R: Runtime>(
    theme: Theme,
    app: AppHandle<R>,
    state: State<'_, AppState>,
) -> Result<AppSettings> {
    write_settings(&app, &state, |settings| settings.theme = theme)
}

/// Whether the sidebar's notes panel is folded away (`notes` design D13). One
/// command per field, like the two above; the note's own text is library data
/// and goes through `note_set` instead.
#[tauri::command]
pub fn set_notes_collapsed<R: Runtime>(
    collapsed: bool,
    app: AppHandle<R>,
    state: State<'_, AppState>,
) -> Result<AppSettings> {
    write_settings(&app, &state, |settings| {
        settings.notes_collapsed = collapsed;
    })
}

/// Whether the sidebar's collections section is folded away (`browse-feedback`
/// design D4), copied from `set_notes_collapsed`'s line.
#[tauri::command]
pub fn set_collections_collapsed<R: Runtime>(
    collapsed: bool,
    app: AppHandle<R>,
    state: State<'_, AppState>,
) -> Result<AppSettings> {
    write_settings(&app, &state, |settings| {
        settings.collections_collapsed = collapsed;
    })
}

/// Whether `setup` reopens the remembered library automatically at the next
/// launch (`launch-screen` design D3). Takes effect next launch: this launch
/// already decided whether to open before the webview could call it.
#[tauri::command]
pub fn set_open_last_on_launch<R: Runtime>(
    value: bool,
    app: AppHandle<R>,
    state: State<'_, AppState>,
) -> Result<AppSettings> {
    write_settings(&app, &state, |settings| {
        settings.open_last_on_launch = value;
    })
}

/// Clamped, never refused: the slider is what sends this, and a rejected size
/// would leave the grid disagreeing with the control that set it.
#[tauri::command]
pub fn set_grid_tile_size<R: Runtime>(
    size: u32,
    app: AppHandle<R>,
    state: State<'_, AppState>,
) -> Result<AppSettings> {
    write_settings(&app, &state, |settings| {
        settings.grid_tile_size = size.clamp(GRID_TILE_MIN, GRID_TILE_MAX);
    })
}

/// Whether the grid's tag footer shows outside edit mode, where the mode
/// forces it on regardless (`tile-tags-in-edit-mode` design D3). No clamp:
/// unlike the tile size, there is no out-of-range value for a bare flag.
#[tauri::command]
pub fn set_show_tile_tags<R: Runtime>(
    value: bool,
    app: AppHandle<R>,
    state: State<'_, AppState>,
) -> Result<AppSettings> {
    write_settings(&app, &state, |settings| {
        settings.show_tile_tags = value;
    })
}

/// Clamped, never refused, for `set_grid_tile_size`'s reason: a rejected
/// ceiling would leave the settings screen disagreeing with the slider that
/// set it.
#[tauri::command]
pub fn set_click_zoom_ceiling_percent<R: Runtime>(
    percent: u32,
    app: AppHandle<R>,
    state: State<'_, AppState>,
) -> Result<AppSettings> {
    write_settings(&app, &state, |settings| {
        settings.click_zoom_ceiling_percent =
            percent.clamp(CLICK_ZOOM_CEILING_MIN, CLICK_ZOOM_CEILING_MAX);
    })
}

/// Change the settings the app is running on and the file they came from
/// together. A preference that reached only one of the two is one the running
/// app ignores, or one that comes back on the next launch.
fn write_settings<R: Runtime>(
    app: &AppHandle<R>,
    state: &AppState,
    edit: impl FnOnce(&mut Settings),
) -> Result<AppSettings> {
    let mut settings = lock(&state.settings);
    edit(&mut settings);
    settings::save(app, &settings)?;
    Ok(AppSettings::from(&*settings))
}

#[tauri::command]
pub async fn search(req: SearchRequest, state: State<'_, AppState>) -> Result<SearchResult> {
    with_library_off_main_thread(&state.library, move |library| {
        let page = query::search(&library.conn, &req)?;
        let ids: Vec<String> = page.images.iter().map(|image| image.id.clone()).collect();
        // The page, never the library: this stats one file per id, and a
        // ten-thousand-image library would pay ten thousand stat calls on every
        // keystroke in the search box.
        maintenance::refresh_missing_for(library, &ids)?;
        // `page.images` was read before that pass, so it still carries the flags
        // the pass has just replaced. Reloading is what makes a file that
        // vanished — or came back — since ingest show as such in the grid.
        Ok(SearchResult {
            images: ingest::load_records(&library.conn, &ids)?,
            total: page.total,
            groups: page.groups,
        })
    })
    .await
}

/// Just the ids a `search` of `req` would page, in its order, with no records
/// (`selection-and-bulk` design D3) — what a range selection resolves against,
/// so its row *n* and the grid's row *n* are always the same image.
#[tauri::command]
pub async fn search_ids(req: SearchRequest, state: State<'_, AppState>) -> Result<Vec<String>> {
    with_library_off_main_thread(&state.library, move |library| {
        query::search_ids(&library.conn, &req)
    })
    .await
}

/// Which of `ids` a `search` of `req` still matches — what the selection
/// prunes itself by after a write re-reads the search (`browse-fixes` design
/// D1), so the count, the thumbnail strip and the next bulk action describe
/// only images the result still shows.
#[tauri::command]
pub async fn matching_ids(
    req: SearchRequest,
    ids: Vec<String>,
    state: State<'_, AppState>,
) -> Result<Vec<String>> {
    with_library_off_main_thread(&state.library, move |library| {
        query::matching_ids(&library.conn, &req, &ids)
    })
    .await
}

/// The zero-based row `id` occupies in the order a `search` of `req` would
/// page, or `None` when `id` is not in `req`'s matched set — what a
/// click-driven search rewrite asks to keep its subject current
/// (`inspector-polish` design D2).
#[tauri::command]
pub async fn search_position(
    req: SearchRequest,
    id: String,
    state: State<'_, AppState>,
) -> Result<Option<i64>> {
    with_library_off_main_thread(&state.library, move |library| {
        query::search_position(&library.conn, &req, &id)
    })
    .await
}

/// The sidebar's tag list and rating pills for one search (design D8).
#[tauri::command]
pub async fn tag_counts(req: SearchRequest, state: State<'_, AppState>) -> Result<TagCounts> {
    with_library_off_main_thread(&state.library, move |library| {
        query::tag_counts(&library.conn, &req)
    })
    .await
}

/// Replace the image's whole tag set (design D2) and answer with the row as it
/// now stands, so the webview can redraw the tile and the inspector without
/// re-running the search (design D10).
#[tauri::command]
pub async fn update_tags(
    id: String,
    tags: Vec<String>,
    state: State<'_, AppState>,
) -> Result<ImageRecord> {
    with_library_off_main_thread(&state.library, move |library| {
        tags::update_tags(library, &id, &tags)
    })
    .await
}

/// `None` clears the rating; anything but `g`/`s`/`q`/`e` is refused (D11).
#[tauri::command]
pub async fn set_rating(
    id: String,
    rating: Option<String>,
    state: State<'_, AppState>,
) -> Result<ImageRecord> {
    with_library_off_main_thread(&state.library, move |library| {
        tags::set_rating(library, &id, rating.as_deref())
    })
    .await
}

/// Title, page address and image address, the three facts a local import
/// starts with none of (`editable-info` design D1). A non-empty address that
/// is not `http`/`https` is refused; empty clears the field. Answers with the
/// row as it now stands, so the webview can redraw it without re-running the
/// search (design D2).
#[tauri::command]
pub async fn update_facts(
    id: String,
    edit: FactsEdit,
    state: State<'_, AppState>,
) -> Result<ImageRecord> {
    with_library_off_main_thread(&state.library, move |library| {
        facts::update(library, &id, &edit)
    })
    .await
}

/// The autocomplete's vocabulary, straight out of the library's own tags (D12).
#[tauri::command]
pub async fn tag_suggestions(
    prefix: String,
    limit: i64,
    state: State<'_, AppState>,
) -> Result<Vec<TagCount>> {
    with_library_off_main_thread(&state.library, move |library| {
        tags::suggestions(&library.conn, &prefix, limit)
    })
    .await
}

/// One stamp's write across every id in `ids` (`stamps` design D2): tags
/// added and removed, collections joined and left, and the rating set, all in
/// one transaction. Answers with the written rows, so a single-tile apply in
/// edit mode can show the result without a search re-run; the bulk tag dialog
/// and the pinned chip's edit call this too, each filling only their own part
/// of `edit`.
#[tauri::command]
pub async fn apply_edit(
    ids: Vec<String>,
    edit: TagEditSpec,
    state: State<'_, AppState>,
) -> Result<Vec<ImageRecord>> {
    with_library_off_main_thread(&state.library, move |library| {
        tags::apply_edit(library, &ids, &edit)
    })
    .await
}

/// One rating across every id in `ids`, as a single `UPDATE` (design D10).
/// `None` clears every id to unrated.
#[tauri::command]
pub async fn bulk_set_rating(
    ids: Vec<String>,
    rating: Option<String>,
    state: State<'_, AppState>,
) -> Result<()> {
    with_library_off_main_thread(&state.library, move |library| {
        tags::bulk_set_rating(library, &ids, rating.as_deref())
    })
    .await
}

/// The `limit` tags most common among `ids`, with their counts — the bulk tag
/// dialog's quick-remove pills (design D9), and, with `names` given, the
/// pinned chips' tri-state over a selection (`tag-vocabulary` design D8): the
/// counts of exactly the pinned tags, `limit` ignored since `names` is
/// already the bound. `names` absent is the bulk dialog's call, unchanged.
#[tauri::command]
pub async fn selection_tag_counts(
    ids: Vec<String>,
    limit: i64,
    names: Option<Vec<String>>,
    state: State<'_, AppState>,
) -> Result<Vec<TagCount>> {
    with_library_off_main_thread(&state.library, move |library| {
        tags::selection_tag_counts(&library.conn, &ids, limit, names.as_deref())
    })
    .await
}

/// The tag vocabulary's exceptions (`tag-vocabulary` design D2): every tag
/// that is not `(general, unpinned)`.
#[tauri::command]
pub async fn tag_vocabulary(state: State<'_, AppState>) -> Result<Vec<TagEntry>> {
    with_library_off_main_thread(&state.library, |library| tags::vocabulary(&library.conn)).await
}

/// Change a tag's category everywhere it is shown, without touching any
/// image's tag set (`tag-vocabulary` design D4); answers with the vocabulary
/// as it now stands.
#[tauri::command]
pub async fn set_tag_category(
    name: String,
    category: TagCategory,
    state: State<'_, AppState>,
) -> Result<Vec<TagEntry>> {
    with_library_off_main_thread(&state.library, move |library| {
        tags::set_category(library, &name, category)
    })
    .await
}

/// Pin or unpin a tag (`tag-vocabulary` design D8); answers with the
/// vocabulary as it now stands.
#[tauri::command]
pub async fn set_tag_pinned(
    name: String,
    pinned: bool,
    state: State<'_, AppState>,
) -> Result<Vec<TagEntry>> {
    with_library_off_main_thread(&state.library, move |library| {
        tags::set_pinned(library, &name, pinned)
    })
    .await
}

/// Writes `ids` to a zip at `path`, emitting `export:progress` as it goes
/// (design D11–D13). The save dialog runs in the webview (design D12); this
/// takes a path so a Rust test can drive it against a `tempfile::TempDir`.
/// `utc_offset_minutes` is the webview's zone, east-positive, for the entry
/// timestamps (`export::entry_mtime`).
#[tauri::command]
pub async fn export_zip<R: Runtime>(
    ids: Vec<String>,
    path: String,
    utc_offset_minutes: i32,
    app: AppHandle<R>,
    state: State<'_, AppState>,
) -> Result<ExportReport> {
    off_main_thread(&state.library, move |library| {
        export::export_zip(
            library,
            &ids,
            Path::new(&path),
            utc_offset_minutes,
            &mut |progress: ExportProgress| {
                // A dropped tick is a bar that skips a number; the export itself is
                // unaffected, so it is not worth failing over (design D13).
                let _ = app.emit(EXPORT_PROGRESS_EVENT, progress);
            },
        )
    })
    .await
}

#[tauri::command]
pub async fn image_counts(state: State<'_, AppState>) -> Result<ImageCounts> {
    with_library_off_main_thread(&state.library, maintenance::image_counts).await
}

/// Move every id in `ids` to the trash (`trash` design D3): the file, the
/// thumbnail, the tags and the rating are untouched, so a restore hands the
/// image back exactly as it was.
#[tauri::command]
pub async fn trash_images(ids: Vec<String>, state: State<'_, AppState>) -> Result<()> {
    with_library_off_main_thread(&state.library, move |library| {
        trash::trash_images(library, &ids)
    })
    .await
}

/// Put every id in `ids` back in the library, exactly reversing
/// [`trash_images`].
#[tauri::command]
pub async fn restore_images(ids: Vec<String>, state: State<'_, AppState>) -> Result<()> {
    with_library_off_main_thread(&state.library, move |library| {
        trash::restore_images(library, &ids)
    })
    .await
}

/// Permanently delete every id in `ids`: the record, its thumbnail and its
/// file under `images/` (`trash` design D4–D6). A file that will not go is
/// named in `DeleteReport.filesLeft` rather than failing the call.
#[tauri::command]
pub async fn delete_forever(ids: Vec<String>, state: State<'_, AppState>) -> Result<DeleteReport> {
    with_library_off_main_thread(&state.library, move |library| {
        trash::delete_forever(library, &ids)
    })
    .await
}

/// Permanently delete everything in the trash (`trash` design D7).
#[tauri::command]
pub async fn empty_trash(state: State<'_, AppState>) -> Result<DeleteReport> {
    with_library_off_main_thread(&state.library, trash::empty_trash).await
}

/// How many images are in the trash — the sidebar's badge (`trash` design
/// D11).
#[tauri::command]
pub async fn trash_count(state: State<'_, AppState>) -> Result<i64> {
    with_library_off_main_thread(&state.library, trash::trash_count).await
}

/// The library's rules, ordered by name, each carrying its pattern's validity
/// (`auto-tag-rules` design D4, D6).
#[tauri::command]
pub async fn rules_list(state: State<'_, AppState>) -> Result<Vec<RuleListEntry>> {
    with_library_off_main_thread(&state.library, |library| rules::list(&library.conn)).await
}

/// Create a rule when `rule.id` is absent, or edit the one it names; refused
/// with the reason for an empty name, no tags, or an unusable regular
/// expression (design D6).
#[tauri::command]
pub async fn rules_upsert(rule: RuleInput, state: State<'_, AppState>) -> Result<Rule> {
    with_library_off_main_thread(&state.library, move |library| rules::upsert(library, &rule)).await
}

/// Delete a rule; idempotent, and no image it ever tagged is touched.
#[tauri::command]
pub async fn rules_delete(id: String, state: State<'_, AppState>) -> Result<()> {
    with_library_off_main_thread(&state.library, move |library| rules::delete(library, &id)).await
}

/// Apply the current rules to every image already in the library, emitting
/// `rules:progress` as it goes (design D9, D12). Mirrors `import_paths`: a
/// blocking run over the shared library, released between images.
#[tauri::command]
pub async fn rules_run<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
) -> Result<RulesRunReport> {
    off_main_thread(&state.library, move |library| {
        rules::run(library, &mut |done, total| {
            // A dropped tick is a progress bar that skips a number; the run
            // itself is unaffected, so it is not worth failing over.
            let _ = app.emit(RULES_PROGRESS_EVENT, ExportProgress { done, total });
        })
    })
    .await
}

/// Write the library's rules to `path` in the legacy JSON shape (design D10).
#[tauri::command]
pub async fn rules_export(path: String, state: State<'_, AppState>) -> Result<()> {
    with_library_off_main_thread(&state.library, move |library| {
        let json = rules::export_json(library)?;
        std::fs::write(&path, json).map_err(AppError::Io)
    })
    .await
}

/// Read `path` and import its rules, skipping ones already present by
/// fingerprint (design D10).
#[tauri::command]
pub async fn rules_import(path: String, state: State<'_, AppState>) -> Result<RulesImportReport> {
    with_library_off_main_thread(&state.library, move |library| {
        let text = std::fs::read_to_string(&path).map_err(AppError::Io)?;
        rules::import_json(library, &text)
    })
    .await
}

/// Every collection in the library, by name (`collections` design D3).
#[tauri::command]
pub async fn collection_list(state: State<'_, AppState>) -> Result<Vec<Collection>> {
    with_library_off_main_thread(&state.library, |library| collections::list(&library.conn)).await
}

/// Create a collection named `name`; refused with the reason for a blank name
/// or a slug clash, naming the collection that already holds it (design D3).
#[tauri::command]
pub async fn collection_create(name: String, state: State<'_, AppState>) -> Result<Collection> {
    with_library_off_main_thread(&state.library, move |library| {
        collections::create(library, &name)
    })
    .await
}

/// Rename collection `id` to `name`; same refusals as [`collection_create`].
/// Rewrites `library.json` and no image's sidecar (design D3).
#[tauri::command]
pub async fn collection_rename(
    id: String,
    name: String,
    state: State<'_, AppState>,
) -> Result<Collection> {
    with_library_off_main_thread(&state.library, move |library| {
        collections::rename(library, &id, &name)
    })
    .await
}

/// Delete collection `id`: its memberships go with it, and no image otherwise
/// changes (design D3). Deleting Favorites is allowed.
#[tauri::command]
pub async fn collection_delete(id: String, state: State<'_, AppState>) -> Result<()> {
    with_library_off_main_thread(&state.library, move |library| {
        collections::delete(library, &id)
    })
    .await
}

/// Put every id in `ids` into `collection_id`, idempotent per id (design D3).
/// Answers with the written rows, so the caller can `replace` them without a
/// second search (design D8). Design D9: a selection spanning a whole large
/// library pays the `library-sidecars` cost here unreduced — one sidecar
/// rewritten per id, holding the library for the whole call; the per-item-lock
/// shape from `library::backfill_sidecars` is not built, `FIXME`d at the write
/// itself in `collections::add`.
#[tauri::command]
pub async fn collection_add(
    ids: Vec<String>,
    collection_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<ImageRecord>> {
    with_library_off_main_thread(&state.library, move |library| {
        collections::add(library, &ids, &collection_id)
    })
    .await
}

/// Take every id in `ids` out of `collection_id`, idempotent per id (design
/// D3). Answers with the written rows, the same as [`collection_add`].
#[tauri::command]
pub async fn collection_remove(
    ids: Vec<String>,
    collection_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<ImageRecord>> {
    with_library_off_main_thread(&state.library, move |library| {
        collections::remove(library, &ids, &collection_id)
    })
    .await
}

/// The library's stamps, in creation order (`stamps` design D3).
#[tauri::command]
pub async fn stamps_list(state: State<'_, AppState>) -> Result<Vec<Stamp>> {
    with_library_off_main_thread(&state.library, |library| stamps::list(&library.conn)).await
}

/// Create a stamp when `input.id` is absent, or edit the one it names;
/// refused with the reason for an empty name or an empty text (design D3).
#[tauri::command]
pub async fn stamps_upsert(input: StampInput, state: State<'_, AppState>) -> Result<Stamp> {
    with_library_off_main_thread(&state.library, move |library| {
        stamps::upsert(library, &input)
    })
    .await
}

/// Delete a stamp; idempotent, and no image it was ever applied to is
/// touched.
#[tauri::command]
pub async fn stamps_delete(id: String, state: State<'_, AppState>) -> Result<()> {
    with_library_off_main_thread(&state.library, move |library| stamps::delete(library, &id)).await
}

/// The library's one note, empty on a library that has never been written to
/// (`notes` design D3).
#[tauri::command]
pub async fn note_get(state: State<'_, AppState>) -> Result<Note> {
    with_library_off_main_thread(&state.library, |library| notes::get(&library.conn)).await
}

/// Write the note and answer with it as stored, so the panel's autosave has
/// the timestamp without a second read.
#[tauri::command]
pub async fn note_set(content: String, state: State<'_, AppState>) -> Result<Note> {
    with_library_off_main_thread(&state.library, move |library| notes::set(library, &content)).await
}

/// The library's configured booru sites, ordered by name (`booru-sites`
/// design D1).
#[tauri::command]
pub async fn booru_site_list(state: State<'_, AppState>) -> Result<Vec<BooruSite>> {
    with_library_off_main_thread(&state.library, |library| booru::sites::list(&library.conn)).await
}

/// Create a site when `id` is absent, or edit the one it names; `id` itself
/// never changes on an edit (design D2). `api_key` left absent leaves
/// whatever credential the site already has untouched — the form field it
/// backs is write-only and starts blank on every edit (task 5.2).
///
/// The database write always happens before the credential is ever touched
/// (design D7): if the credential store then refuses `api_key`, this call
/// fails with that reason, but the site's name, address and username are
/// already saved — `booru_site_list` shows them.
#[tauri::command]
pub async fn booru_site_save(
    id: Option<String>,
    name: String,
    base_url: String,
    username: String,
    api_key: Option<String>,
    state: State<'_, AppState>,
) -> Result<BooruSite> {
    let credentials = state.credentials.clone();
    with_library_off_main_thread(&state.library, move |library| {
        booru::sites::save(
            library,
            credentials.as_ref(),
            id.as_deref(),
            &name,
            &base_url,
            &username,
            api_key.as_deref(),
        )
    })
    .await
}

/// Remove a site and its stored credential; posts already recorded against it
/// are kept, since `posts.site` carries no reference to it (`booru-sites`
/// design D2).
#[tauri::command]
pub async fn booru_site_delete(id: String, state: State<'_, AppState>) -> Result<()> {
    let credentials = state.credentials.clone();
    with_library_off_main_thread(&state.library, move |library| {
        booru::sites::delete(library, credentials.as_ref(), &id)
    })
    .await
}

/// The credential a site posts with, or the `AppError::Credential` that
/// refuses the call before it opens a socket (`booru-sites` design D7):
/// missing entirely and refused by the store are both "cannot be read", and
/// neither reaches the booru.
fn require_credential(credentials: &dyn Credentials, site: &BooruSite) -> Result<String> {
    let host = booru::sites::host_of(&site.base_url)?;
    credentials
        .get(&host, &site.username)?
        .ok_or_else(|| AppError::Credential {
            reason: format!(
                "no API key is stored for {} on {}",
                site.username, site.base_url
            ),
        })
}

/// Contact the site with its stored credential and report what happened,
/// without changing anything on the booru (design D8).
#[tauri::command]
pub async fn booru_site_test(
    id: String,
    state: State<'_, AppState>,
) -> Result<BooruConnectionTest> {
    let credentials = state.credentials.clone();
    let (site, api_key) = with_library_off_main_thread(&state.library, move |library| {
        let site = booru::sites::require(&library.conn, &id)?;
        let api_key = require_credential(credentials.as_ref(), &site)?;
        Ok((site, api_key))
    })
    .await?;

    let client = booru::client::BooruClient::new(&site.base_url, &site.username, &api_key);
    Ok(client.test_connection().await)
}

/// An upload will not be sent without tags and a rating (spec `booru-upload`).
/// The webview's own form already refuses to submit without both (task 3.2);
/// this is the same check made again on the far side of the IPC boundary,
/// the way `rules::upsert` checks its own inputs regardless of what the form
/// already enforced.
fn validate_upload_form(form: &BooruUploadForm) -> Result<()> {
    if form.tags.is_empty() {
        return Err(AppError::BadRequest(
            "an upload needs at least one tag".to_string(),
        ));
    }
    if !tags::RATINGS.contains(&form.rating.as_str()) {
        return Err(AppError::BadRequest(format!(
            "{:?} is not a rating",
            form.rating
        )));
    }
    Ok(())
}

/// Read the image and the site, run the four-step upload sequence, and — only
/// on success — record the post in the same transaction as the
/// `images.updated_at` bump (`booru-upload` design D5). The library mutex is
/// taken twice, briefly, and never across the network `.await`s in between: a
/// missing or refused credential fails the call outright, before either
/// touch, so an upload to an unusable site never opens a socket
/// (`booru-sites` design D7).
#[tauri::command]
pub async fn booru_upload(
    image_id: String,
    site_id: String,
    form: BooruUploadForm,
    state: State<'_, AppState>,
) -> Result<BooruUploadOutcome> {
    validate_upload_form(&form)?;

    let credentials = state.credentials.clone();
    let (paths, image, site, api_key) =
        with_library_off_main_thread(&state.library, move |library| {
            let image = ingest::require_record(&library.conn, &image_id)?;
            let site = booru::sites::require(&library.conn, &site_id)?;
            let api_key = require_credential(credentials.as_ref(), &site)?;
            Ok((library.paths.clone(), image, site, api_key))
        })
        .await?;

    // Off the async thread: the file is as large as whatever was captured,
    // and this runs on the runtime that is also serving the listener. Outside
    // the library mutex too — nothing here needs it (design D5).
    let image_path = paths.image_path(&image.id, &image.ext);
    let bytes = tauri::async_runtime::spawn_blocking(move || {
        std::fs::read(image_path).map_err(AppError::Io)
    })
    .await
    .map_err(from_tauri)??;
    let payload = booru::upload::UploadPayload {
        filename: format!("{}.{}", image.id, image.ext),
        mime: image.mime.clone(),
        bytes,
    };

    let client = booru::client::BooruClient::new(&site.base_url, &site.username, &api_key);
    let outcome = booru::upload::run(
        &client,
        payload,
        &site.id,
        &form,
        booru::client::PollSchedule::default(),
    )
    .await;

    let BooruUploadOutcome::Posted { post, commentary } = outcome else {
        return Ok(outcome);
    };
    let post = PostRef {
        posted_at: db::now_ms(),
        ..post
    };
    let record_post = post.clone();
    let image_id = image.id.clone();
    with_library_off_main_thread(&state.library, move |library| {
        booru::posts::record(library, &image_id, &record_post)
    })
    .await?;

    Ok(BooruUploadOutcome::Posted { post, commentary })
}

/// Absolute path; the thumbnail is generated if it is not there yet.
#[tauri::command]
pub async fn thumbnail_path(id: String, state: State<'_, AppState>) -> Result<String> {
    off_main_thread(&state.library, move |library| {
        // Only reading the record needs the library. Generating the thumbnail
        // is a read, a decode, a downscale and an encode of the full image, and
        // the grid asks for one per tile as it scrolls (design D13).
        let (paths, record) = with_library(library, |library| {
            let record = ingest::load_record(&library.conn, &id)?
                .ok_or_else(|| AppError::NotFound(format!("image {id}")))?;
            Ok((library.paths.clone(), record))
        })?;
        Ok(thumbs::ensure_thumbnail(&paths, &record)?
            .display()
            .to_string())
    })
    .await
}

/// Emits `import:progress` while it runs. Installs a fresh
/// `ImportControl` in `AppState.import_control` before the run and clears the
/// slot once it returns, however it returns, so `import_pause`/
/// `import_resume`/`import_cancel` never reach a stale handle
/// (`import-pause-cancel` design D4).
#[tauri::command]
pub async fn import_paths<R: Runtime>(
    paths: Vec<String>,
    app: AppHandle<R>,
    state: State<'_, AppState>,
) -> Result<ImportReport> {
    let paths: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
    let control = install_import_control(&state);
    let installed = control.clone();
    let result = off_main_thread(&state.library, move |library| {
        // The whole run is one blocking call, but it takes the library one file
        // at a time (design D13), so a search or a thumbnail asked for while it
        // goes is answered between two files rather than after the last.
        import::import_paths(library, &paths, &control, &mut |progress| {
            // A dropped tick is a progress bar that skips a number; the import
            // itself is unaffected, so it is not worth failing over.
            let _ = app.emit(IMPORT_PROGRESS_EVENT, progress);
        })
    })
    .await;
    clear_import_control(&state, &installed);
    result
}

/// Plan a legacy bundle pick without importing anything (`import-confirm`
/// design D2): the same part-planning step `import_bundle` runs before its
/// first row, run here with no library open and nothing written. `files` are
/// absolute paths the webview's multi-select picker chose, per the contract
/// pinned in the change's `tasks.md` header. Opening each part and counting
/// its rows is filesystem work — like `booru_upload`'s file read, it goes
/// straight on a blocking thread rather than through `off_main_thread`, which
/// exists to take the library this command never touches.
#[tauri::command]
pub async fn bundle_plan(files: Vec<String>) -> Result<BundlePlan> {
    let files: Vec<PathBuf> = files.into_iter().map(PathBuf::from).collect();
    tauri::async_runtime::spawn_blocking(move || crate::bundle::plan(&files))
        .await
        .map_err(from_tauri)
}

/// Import a legacy bundle's SQLite parts (`legacy-bundle-import` design D5,
/// D6). `files` are absolute paths the webview's multi-select picker chose;
/// the contract is pinned in the change's `tasks.md` header, shared with the
/// webview wrapper that calls this with `{ files }`. Emits the same
/// `import:progress` event as `import_paths` — one queue, one progress band,
/// one report shape (design D6). Installs and clears the same
/// `ImportControl` slot `import_paths` does (`import-pause-cancel` design
/// D4).
#[tauri::command]
pub async fn import_bundle<R: Runtime>(
    files: Vec<String>,
    app: AppHandle<R>,
    state: State<'_, AppState>,
) -> Result<ImportReport> {
    let files: Vec<PathBuf> = files.into_iter().map(PathBuf::from).collect();
    let control = install_import_control(&state);
    let installed = control.clone();
    let result = off_main_thread(&state.library, move |library| {
        crate::bundle::import_bundle(library, &files, &control, &mut |progress| {
            let _ = app.emit(IMPORT_PROGRESS_EVENT, progress);
        })
    })
    .await;
    clear_import_control(&state, &installed);
    result
}

/// A fresh handle for the run about to start, installed in `AppState` so
/// `import_pause`/`import_resume`/`import_cancel` can reach it (design D4).
/// A handle per run, never reused, is what makes a control signal pressed
/// after a run has finished cancel nothing rather than the next run.
///
/// Nothing in Rust serialises two calls into `import_paths`/`import_bundle`;
/// "one run at a time" is an invariant the webview singleton (`Imports`) owns,
/// not one enforced here. The assert pins that assumption where it is made,
/// so a future second caller overwriting a live handle fails loudly in a debug
/// build rather than silently making the run it clobbered uncancellable.
fn install_import_control(state: &AppState) -> Arc<import::ImportControl> {
    let control = Arc::new(import::ImportControl::default());
    let mut slot = lock(&state.import_control);
    debug_assert!(
        slot.is_none(),
        "an import command started while another's control handle was still \
         installed — the 'one run at a time' invariant the webview is meant \
         to keep was broken"
    );
    *slot = Some(control.clone());
    control
}

/// Empty the slot `install_import_control` filled, but only if it still holds
/// *this* run's handle. Called once the run's future has resolved — success,
/// per-item failure, refusal, or the closed-library `Err` — never left for a
/// later run to find. Comparing identity rather than clearing unconditionally
/// means a run that (in violation of the invariant above) got its handle
/// overwritten by a second one clears nothing here, instead of reaching in
/// and clobbering the second run's own live handle.
fn clear_import_control(state: &AppState, control: &Arc<import::ImportControl>) {
    let mut slot = lock(&state.import_control);
    if slot
        .as_ref()
        .is_some_and(|current| Arc::ptr_eq(current, control))
    {
        *slot = None;
    }
}

/// A silent no-op when nothing is running (design D4): the slot is only
/// `Some` while `import_paths` or `import_bundle` is on its blocking thread.
/// Locks only `import_control`, never the library (design D5) — the library
/// mutex is held, per item, by the very run this is meant to stop, so a
/// command that waited for it would wait for the run it is trying to signal.
#[tauri::command]
pub fn import_pause(state: State<'_, AppState>) -> Result<()> {
    let control = lock(&state.import_control).clone();
    if let Some(control) = control {
        control.pause();
    }
    Ok(())
}

/// See `import_pause`.
#[tauri::command]
pub fn import_resume(state: State<'_, AppState>) -> Result<()> {
    let control = lock(&state.import_control).clone();
    if let Some(control) = control {
        control.resume();
    }
    Ok(())
}

/// See `import_pause`.
#[tauri::command]
pub fn import_cancel(state: State<'_, AppState>) -> Result<()> {
    let control = lock(&state.import_control).clone();
    if let Some(control) = control {
        control.cancel();
    }
    Ok(())
}

/// Open `path`, remember it, and let the webview read images out of it.
///
/// The path is stored only once the folder has actually opened: `status` reads a
/// remembered path with no open library as `missing_path`, so storing it earlier
/// would name a perfectly good folder as missing.
async fn open_and_remember<R: Runtime>(
    app: &AppHandle<R>,
    state: &AppState,
    path: &Path,
) -> Result<LibraryStatus> {
    open_into_state(app, state, path, OpenMode::CreateIfMissing)?;
    write_settings(app, state, |settings| {
        settings.library_path = Some(path.to_path_buf());
        settings.remember_recent(path);
    })?;
    status(state).await
}

/// Whether opening a library folder may bring one into being.
///
/// Picking a folder creates the layout if it is not there — that is the
/// `library-folder` spec's "Folder chosen" scenario. Reopening a remembered
/// path must not: a folder deleted, renamed, or on a volume that is not mounted
/// yet has to surface as missing, and creating one silently puts an empty
/// library where the user's images used to be.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum OpenMode {
    CreateIfMissing,
    ExistingOnly,
}

/// Open the library at `path` into the shared state. `setup` calls this too,
/// for the path it remembered, which is why it is not folded into the command.
///
/// `AppState.open_failure` is written on both outcomes (`library-sidecars`
/// design D9) — set to `path` and why on a refusal, cleared on success — so
/// `status` can tell a damaged remembered library from one that is simply
/// missing without repeating the open. A successful open also starts the
/// sidecar backfill (design D7) on its own blocking thread, emitting
/// `library:sidecars`: the command that opened the library returns without
/// waiting for it, exactly as `pending-work`'s tile is meant to appear after
/// the grid is already on screen.
pub fn open_into_state<R: Runtime>(
    app: &AppHandle<R>,
    state: &AppState,
    path: &Path,
    mode: OpenMode,
) -> Result<()> {
    match open_and_start_backfill(app, state, path, mode) {
        Ok(()) => {
            *lock(&state.open_failure) = None;
            Ok(())
        }
        Err(error) => {
            *lock(&state.open_failure) = Some((path.to_path_buf(), open_failure_kind(&error)));
            Err(error)
        }
    }
}

fn open_failure_kind(error: &AppError) -> OpenFailureKind {
    match error {
        AppError::LibraryCorrupt { .. } => OpenFailureKind::Corrupt,
        AppError::SchemaTooNew { .. } => OpenFailureKind::SchemaTooNew,
        _ => OpenFailureKind::Other,
    }
}

fn open_and_start_backfill<R: Runtime>(
    app: &AppHandle<R>,
    state: &AppState,
    path: &Path,
    mode: OpenMode,
) -> Result<()> {
    let library = match mode {
        OpenMode::CreateIfMissing => Library::open_or_create(path)?,
        OpenMode::ExistingOnly => Library::open_existing(path)?,
    };
    grant_asset_scope(app, &library)?;
    // Only once the new library has actually opened: a switch that fails here
    // must leave whatever is running against the old library alone (design
    // D7's "tears a library down" — a failed open never does).
    cancel_running_import(state);
    *lock(&state.library) = Some(library);

    let root = path.to_path_buf();
    let shared = state.library.clone();
    let handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _ = library::backfill_sidecars(&shared, &root, &mut |done, total| {
            // A dropped tick is a tile that skips a number; the pass itself is
            // unaffected, so it is not worth failing over (mirrors every other
            // progress emitter in this file).
            let _ = handle.emit(SIDECARS_PROGRESS_EVENT, SidecarsProgress { done, total });
        });
    });
    Ok(())
}

/// Let the webview load `<library>/images/…` and `<library>/.thumbs/…` through
/// the asset protocol (design D7). Granted at runtime rather than as a scope in
/// `tauri.conf.json`, because the folder is the user's to choose and no build
/// knows it.
///
/// Grant the two directories by name, never the library root: Tauri's fs scope
/// sets `require_literal_leading_dot` on unix, so a recursive pattern over the
/// root matches nothing under the dotted `.thumbs/` and every thumbnail comes
/// back 403. Spelling `.thumbs` out is what makes it match. Granting these two
/// alone also keeps `library.sqlite` and `inbox/` out of the webview's reach.
fn grant_asset_scope<R: Runtime>(app: &AppHandle<R>, library: &Library) -> Result<()> {
    let scope = app.asset_protocol_scope();
    for directory in [library.paths.images_dir(), library.paths.thumbs_dir()] {
        scope
            .allow_directory(&directory, true)
            .map_err(from_tauri)?;
    }
    Ok(())
}

/// What `LibraryStatus` reports about a library that is open. Read on a
/// blocking thread with the count beside it, so a status poll during a capture
/// never parks the main thread on the mutex (design D13).
struct OpenLibrary {
    path: String,
    image_count: i64,
}

async fn status(state: &AppState) -> Result<LibraryStatus> {
    let open = off_main_thread(&state.library, |library| {
        with_library_if_open(library, |open| match open {
            Some(library) => Ok(Some(OpenLibrary {
                path: library.paths.root.display().to_string(),
                image_count: library.image_count()?,
            })),
            None => Ok(None),
        })
    })
    .await?;
    let remembered = lock(&state.settings).library_path.clone();

    // The one failure kind recorded against exactly `remembered`, if any
    // (design D9) — set by `open_into_state` on the same attempt that left it
    // unopened. A failure recorded against some other path (a `pick_library`
    // the user then abandoned) never surfaces here.
    let failure_kind = remembered.as_ref().and_then(|remembered| {
        lock(&state.open_failure)
            .as_ref()
            .filter(|(path, _)| path == remembered)
            .map(|(_, kind)| *kind)
    });

    // Three slots, at most one of them ever set (`library-recovery`'s "reported
    // as damaged, distinctly from a library folder that is missing and from one
    // written by a newer version"): `LibraryCorrupt` is `damaged_path`,
    // `SchemaTooNew` is `newer_path`, everything else keeps `missing_path`.
    // A newer library has to name its folder in a slot of its own — reporting
    // none of the three leaves the start screen on "choose a library folder",
    // which says nothing about the library the user already has.
    let failed_path = |kind: OpenFailureKind| match (&open, failure_kind) {
        (None, Some(recorded)) if recorded == kind => {
            remembered.as_ref().map(|path| path.display().to_string())
        }
        _ => None,
    };
    let damaged_path = failed_path(OpenFailureKind::Corrupt);
    let newer_path = failed_path(OpenFailureKind::SchemaTooNew);

    Ok(LibraryStatus {
        opened: open.is_some(),
        opening: lock(&state.launch_opening)
            .as_ref()
            .map(|path| path.display().to_string()),
        library_path: open.as_ref().map(|library| library.path.clone()),
        // Derived, not stored: a remembered path with no library open is exactly
        // the path that would not open, which the spec keeps until the user
        // picks another. A second field would have to be cleared by hand
        // wherever a library opens, and one day would not be.
        missing_path: match &open {
            Some(_) => None,
            None if damaged_path.is_some() || newer_path.is_some() => None,
            None => remembered.map(|path| path.display().to_string()),
        },
        damaged_path,
        newer_path,
        image_count: open.map_or(0, |library| library.image_count),
        listener: lock(&state.listener).clone(),
        version: VERSION.to_string(),
    })
}

async fn pick_folder<R: Runtime>(app: &AppHandle<R>) -> Option<PathBuf> {
    let (send, receive) = tokio::sync::oneshot::channel();
    app.dialog().file().pick_folder(|picked| {
        // The receiver is only gone if this command was cancelled; nothing to do.
        let _ = send.send(picked);
    });
    receive
        .await
        .ok()?
        .and_then(|picked| picked.into_path().ok())
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::mpsc;
    use std::time::Duration;

    use tauri::Listener;
    use tauri::test::MockRuntime;

    use super::*;
    use crate::http::test_support::{ask_for_status, free_port, nothing_answers_on, png_bytes};
    use crate::model::{
        CLICK_ZOOM_CEILING_DEFAULT, GRID_TILE_DEFAULT, GroupBy, ImportProgress, ImportStatus,
        ParsedTagSearch, SearchView, Sort,
    };
    use crate::test_support::mock_app;

    type App = tauri::App<MockRuntime>;

    fn everything() -> SearchRequest {
        SearchRequest {
            query: ParsedTagSearch::default(),
            text: String::new(),
            view: SearchView::Library,
            sort: Sort::default(),
            group: GroupBy::default(),
            limit: 100,
            offset: 0,
        }
    }

    /// Drive a command to completion. They are `async` so that none of them
    /// waits for the library on the app's main thread (design D13); a test has
    /// no run loop to keep free, so it simply blocks here.
    fn now<T>(command: impl std::future::Future<Output = T>) -> T {
        tauri::async_runtime::block_on(command)
    }

    fn open(app: &App, path: &Path) -> Result<LibraryStatus> {
        now(open_library(
            path.display().to_string(),
            app.handle().clone(),
            app.state(),
        ))
    }

    fn status_of(app: &App) -> LibraryStatus {
        now(library_status(app.state())).unwrap()
    }

    fn search_all(app: &App) -> SearchResult {
        now(search(everything(), app.state())).unwrap()
    }

    /// An app with a library open in a temp folder. The `TempDir` comes back
    /// with it: dropping it deletes the library.
    fn app_with_library() -> (tempfile::TempDir, App) {
        let dir = tempfile::tempdir().unwrap();
        let app = mock_app();
        open(&app, dir.path()).unwrap();
        (dir, app)
    }

    /// `count` PNG files in a fresh folder, for import to walk.
    fn folder_of_images(count: u32) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        for index in 0..count {
            std::fs::write(dir.path().join(format!("{index}.png")), png_bytes(8, 6)).unwrap();
        }
        dir
    }

    fn import(app: &App, dir: &tempfile::TempDir) -> ImportReport {
        now(import_paths(
            vec![dir.path().display().to_string()],
            app.handle().clone(),
            app.state(),
        ))
        .unwrap()
    }

    fn ids_in_library(app: &App) -> Vec<String> {
        search_all(app)
            .images
            .into_iter()
            .map(|image| image.id)
            .collect()
    }

    /// `launch-screen` task 1.3 / design D4: what is slow is measured, not
    /// guessed. Ignored because it needs a real library folder on disk and
    /// exists to be run by hand — `LAUNCH_SCREEN_MEASURE_DIR=<path> cargo test
    /// --release -- --ignored --nocapture measure_launch_open_time` — whenever
    /// the number needs rechecking against a larger library than the one
    /// `design.md` Risks records.
    #[test]
    #[ignore = "manual timing measurement, run by hand against a real library"]
    fn measure_launch_open_time() {
        let dir = std::env::var("LAUNCH_SCREEN_MEASURE_DIR")
            .expect("set LAUNCH_SCREEN_MEASURE_DIR to a library folder to time");
        let path = Path::new(&dir);

        let start = std::time::Instant::now();
        let opened = Library::open_existing(path).unwrap();
        let open_existing = start.elapsed();
        let count = opened.image_count().unwrap();
        drop(opened);

        let app = mock_app();
        let state = app.state::<AppState>();
        let start = std::time::Instant::now();
        open_into_state(app.handle(), &state, path, OpenMode::ExistingOnly).unwrap();
        let full = start.elapsed();

        eprintln!(
            "measure_launch_open_time: {count} images, open_existing={open_existing:?}, open_into_state={full:?}"
        );
    }

    #[test]
    fn open_library_creates_the_folder_and_reports_it_open() {
        let dir = tempfile::tempdir().unwrap();
        let app = mock_app();

        let status = open(&app, dir.path()).unwrap();

        assert!(status.opened);
        assert_eq!(status.library_path, Some(dir.path().display().to_string()));
        assert_eq!(status.missing_path, None);
        assert_eq!(status.image_count, 0);
        assert_eq!(status.version, VERSION);
        assert!(dir.path().join("library.sqlite").is_file());
    }

    #[test]
    fn library_status_without_a_library_offers_no_path() {
        let app = mock_app();

        let status = status_of(&app);

        assert!(!status.opened);
        assert_eq!(status.library_path, None);
        assert_eq!(status.missing_path, None);
    }

    /// `launch-screen` design D1: `library_status.opening` is what lets the
    /// webview name the folder on the opening screen before the launch open
    /// has set `state.library` — `opened` must still read false meanwhile.
    #[test]
    fn library_status_reports_the_opening_path_while_the_launch_flag_is_set() {
        let app = mock_app();
        let opening = PathBuf::from("/library/opening");
        *lock(&app.state::<AppState>().launch_opening) = Some(opening.clone());

        let status = status_of(&app);

        assert!(!status.opened);
        assert_eq!(status.opening, Some(opening.display().to_string()));
    }

    #[test]
    fn a_remembered_path_that_did_not_open_is_reported_as_missing() {
        let app = mock_app();
        let gone = PathBuf::from("/nonexistent/boorubox/library");
        lock(&app.state::<AppState>().settings).library_path = Some(gone.clone());

        let status = status_of(&app);

        assert!(!status.opened);
        assert_eq!(status.missing_path, Some(gone.display().to_string()));
    }

    #[test]
    fn opening_a_library_clears_a_missing_path() {
        let dir = tempfile::tempdir().unwrap();
        let app = mock_app();
        lock(&app.state::<AppState>().settings).library_path =
            Some(PathBuf::from("/nonexistent/boorubox/library"));

        let status = open(&app, dir.path()).unwrap();

        assert_eq!(status.missing_path, None);
        assert_eq!(status.library_path, Some(dir.path().display().to_string()));
    }

    #[test]
    fn a_fresh_profile_reads_the_default_preferences() {
        let app = mock_app();

        let settings = app_settings(app.state());

        assert_eq!(settings.theme, Theme::System);
        assert_eq!(settings.grid_tile_size, GRID_TILE_DEFAULT);
        assert_eq!(
            settings.click_zoom_ceiling_percent,
            CLICK_ZOOM_CEILING_DEFAULT
        );
    }

    #[test]
    fn the_theme_reaches_the_state_and_the_store() {
        let app = mock_app();

        let answered = set_theme(Theme::Dark, app.handle().clone(), app.state()).unwrap();

        assert_eq!(answered.theme, Theme::Dark);
        assert_eq!(app_settings(app.state()).theme, Theme::Dark);
        assert_eq!(settings::load(app.handle()).theme, Theme::Dark);
    }

    /// A slider that sent 10 000 must leave the grid and the setting agreeing,
    /// so the size is clamped where it is stored rather than refused.
    #[test]
    fn a_tile_size_past_the_range_is_clamped_rather_than_refused() {
        let app = mock_app();

        let small = set_grid_tile_size(1, app.handle().clone(), app.state()).unwrap();
        assert_eq!(small.grid_tile_size, GRID_TILE_MIN);

        let large = set_grid_tile_size(10_000, app.handle().clone(), app.state()).unwrap();
        assert_eq!(large.grid_tile_size, GRID_TILE_MAX);
        assert_eq!(settings::load(app.handle()).grid_tile_size, GRID_TILE_MAX);
    }

    /// Copied from the tile size's round trip above, minus the clamp: a bare
    /// flag has no out-of-range value to prove, only that it reaches the
    /// running state and the file.
    #[test]
    fn show_tile_tags_reaches_the_state_and_the_store() {
        let app = mock_app();

        let updated = set_show_tile_tags(true, app.handle().clone(), app.state()).unwrap();

        assert!(updated.show_tile_tags);
        assert!(settings::load(app.handle()).show_tile_tags);
    }

    /// Same rule as the tile size's: a slider that sent 10 000 must leave the
    /// viewer and the setting agreeing, so the ceiling is clamped rather than
    /// refused.
    #[test]
    fn a_click_zoom_ceiling_past_the_range_is_clamped_rather_than_refused() {
        let app = mock_app();

        let small = set_click_zoom_ceiling_percent(1, app.handle().clone(), app.state()).unwrap();
        assert_eq!(small.click_zoom_ceiling_percent, CLICK_ZOOM_CEILING_MIN);

        let large =
            set_click_zoom_ceiling_percent(10_000, app.handle().clone(), app.state()).unwrap();
        assert_eq!(large.click_zoom_ceiling_percent, CLICK_ZOOM_CEILING_MAX);
        assert_eq!(
            settings::load(app.handle()).click_zoom_ceiling_percent,
            CLICK_ZOOM_CEILING_MAX
        );
    }

    /// The `library-switching` spec's "Two libraries seen": the list is what
    /// the start screen offers, so its order is the requirement.
    #[test]
    fn switching_from_one_library_to_another_lists_the_newest_first() {
        let first = tempfile::tempdir().unwrap();
        let second = tempfile::tempdir().unwrap();
        let app = mock_app();

        for dir in [&first, &second] {
            open(&app, dir.path()).unwrap();
        }

        let recent = recent_libraries(app.state());

        assert_eq!(
            recent.iter().map(|entry| &entry.path).collect::<Vec<_>>(),
            vec![
                &second.path().display().to_string(),
                &first.path().display().to_string(),
            ],
        );
        assert!(recent.iter().all(|entry| entry.available));
        assert_eq!(
            recent[0].name,
            second.path().file_name().unwrap().to_string_lossy(),
        );
    }

    #[test]
    fn closing_a_library_leaves_nothing_open_and_nothing_missing() {
        let (dir, app) = app_with_library();

        let status = now(close_library(app.handle().clone(), app.state())).unwrap();

        assert!(!status.opened);
        assert_eq!(status.library_path, None);
        assert_eq!(
            status.missing_path, None,
            "a deliberate close is not a missing folder (design D3)",
        );
        assert_eq!(settings::load(app.handle()).library_path, None);
        assert_eq!(
            recent_libraries(app.state())[0].path,
            dir.path().display().to_string(),
            "the folder stays one click away",
        );
    }

    #[test]
    fn a_recent_folder_that_is_gone_is_listed_unavailable() {
        let parent = tempfile::tempdir().unwrap();
        let gone = parent.path().join("moved-away");
        std::fs::create_dir(&gone).unwrap();
        let app = mock_app();
        open(&app, &gone).unwrap();
        std::fs::remove_dir_all(&gone).unwrap();

        let recent = recent_libraries(app.state());

        assert_eq!(recent.len(), 1, "an entry stays listed until it is removed");
        assert!(!recent[0].available);
        assert_eq!(recent[0].name, "moved-away");
    }

    #[test]
    fn forgetting_an_entry_leaves_the_folder_on_disk() {
        let (dir, app) = app_with_library();

        let recent = forget_recent(
            dir.path().display().to_string(),
            app.handle().clone(),
            app.state(),
        )
        .unwrap();

        assert!(recent.is_empty());
        assert!(settings::load(app.handle()).recent_libraries.is_empty());
        assert!(
            dir.path().join("library.sqlite").is_file(),
            "forgetting an entry must not touch the folder",
        );
    }

    /// Design D1: a switch opens the new library before it lets go of the old
    /// one, so a typo or an unmounted volume cannot close the library the user
    /// is working in.
    #[test]
    fn an_open_that_fails_leaves_the_previous_library_open() {
        let (dir, app) = app_with_library();
        let blocked = dir.path().join("a-file");
        std::fs::write(&blocked, b"not a folder").unwrap();

        let error = open(&app, &blocked.join("library")).unwrap_err();

        let status = status_of(&app);
        assert!(status.opened, "the working library must survive: {error:?}");
        assert_eq!(status.library_path, Some(dir.path().display().to_string()));
        assert_eq!(recent_libraries(app.state()).len(), 1);
    }

    /// The reveal itself opens a file manager window, so only the refusal is
    /// testable here; the rest is the manual pass (task 5.4).
    #[test]
    fn revealing_the_library_folder_needs_one_to_be_open() {
        let app = mock_app();

        let error = now(reveal_library(app.handle().clone(), app.state())).unwrap_err();

        assert!(matches!(error, AppError::NoLibrary), "{error:?}");
    }

    fn set_port(app: &App, port: u16) -> ListenerStatus {
        now(set_listener_port(port, app.handle().clone(), app.state())).unwrap()
    }

    /// The `capture-ingest` scenario "Port changed to a free one". The old port
    /// must be released, not merely stopped being served: the rebind waits for
    /// the accept loop to end before it binds (design D6).
    #[test]
    fn changing_the_port_moves_the_listener_and_releases_the_old_one() {
        let (_library, app) = app_with_library();
        let was = free_port();
        assert!(set_port(&app, was).running);
        // Asked for only once `was` is held, so the kernel cannot hand it out
        // again as the new port.
        let now = free_port();

        let status = set_port(&app, now);

        assert!(status.running, "rebind failed: {:?}", status.error);
        assert_eq!(status.port, now);
        assert!(
            ask_for_status(now).starts_with("HTTP/1.1 200"),
            "the new port must serve the open library",
        );
        assert!(nothing_answers_on(was), "the old port must be released");
        assert_eq!(status_of(&app).listener.port, now);
    }

    /// The `capture-ingest` scenario "Port changed to one already in use": the
    /// chosen port is stored anyway, so the settings field shows what the user
    /// typed rather than silently reverting.
    #[test]
    fn a_port_another_socket_holds_is_reported_and_the_app_keeps_working() {
        let (_library, app) = app_with_library();
        let taken = std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).unwrap();
        let port = taken.local_addr().unwrap().port();

        let status = set_port(&app, port);

        assert!(!status.running);
        assert_eq!(status.port, port);
        assert!(
            status.error.is_some(),
            "settings show the reason, so it must be carried",
        );
        assert_eq!(settings::load(app.handle()).port, port);
        assert!(
            status_of(&app).opened,
            "a listener that will not bind must not take the app down",
        );
    }

    #[test]
    fn import_paths_reports_every_file_and_ticks_progress() {
        let (_library, app) = app_with_library();
        let images = folder_of_images(3);
        let ticks = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let seen = ticks.clone();
        app.listen(IMPORT_PROGRESS_EVENT, move |event| {
            let progress: ImportProgress = serde_json::from_str(event.payload()).unwrap();
            lock(&seen).push(progress);
        });

        let report = import(&app, &images);

        assert_eq!(report.imported, 3);
        assert_eq!(report.failed, 0);
        assert_eq!(report.items.len(), 3);
        let ticks = lock(&ticks).clone();
        assert_eq!(
            ticks.last().map(|last| (last.done, last.total)),
            Some((3, 3)),
            "the last tick must show the run finished: {ticks:?}"
        );
    }

    /// `legacy-bundle-import` task 1.4: the command's own contract — the
    /// argument name (`files`), the answer shape and the progress event it
    /// reuses from `import_paths` — driven against the checked-in fixture.
    #[test]
    fn import_bundle_reports_the_fixture_and_ticks_the_same_progress_event() {
        let (_library, app) = app_with_library();
        let fixture = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/fixtures/legacy-bundle/database.db"
        );
        let ticks = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let seen = ticks.clone();
        app.listen(IMPORT_PROGRESS_EVENT, move |event| {
            let progress: ImportProgress = serde_json::from_str(event.payload()).unwrap();
            lock(&seen).push(progress);
        });

        let report = now(import_bundle(
            vec![fixture.to_string()],
            app.handle().clone(),
            app.state(),
        ))
        .unwrap();

        assert_eq!((report.imported, report.skipped, report.failed), (3, 0, 1));
        let ticks = lock(&ticks).clone();
        assert_eq!(
            ticks.last().map(|last| (last.done, last.total)),
            Some((4, 4)),
            "the last tick must show the run finished: {ticks:?}"
        );
        assert_eq!(
            search_all(&app)
                .images
                .iter()
                .filter(|image| image.source == crate::model::ImageSource::LegacyBundle)
                .count(),
            2,
            "one of the three imported rows is deleted and stays out of the library view"
        );
    }

    /// `import-confirm` task 1.3: the command's own contract — it answers
    /// with no library open, and its answer is `bundle::plan` for the same
    /// files, argument name and all.
    #[test]
    fn bundle_plan_answers_with_no_library_open_and_matches_bundle_plan() {
        let fixture = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/fixtures/legacy-bundle/database.db"
        );

        let plan = now(bundle_plan(vec![fixture.to_string()])).unwrap();

        assert_eq!(plan, crate::bundle::plan(&[PathBuf::from(fixture)]));
    }

    /// Long enough that a machine under load does not fail the test, short
    /// enough that a command that really is stuck ends the suite instead of
    /// hanging it (mirrors `import.rs`'s `A_LOCK_IS_NOT_COMING`).
    const A_LOCK_IS_NOT_COMING: Duration = Duration::from_secs(5);

    /// `import-pause-cancel` task 1.6: with nothing running, the slot is
    /// `None`, so all three commands are a silent no-op rather than an error.
    #[test]
    fn the_control_commands_are_no_ops_with_nothing_running() {
        let (_dir, app) = app_with_library();

        import_pause(app.state()).unwrap();
        import_resume(app.state()).unwrap();
        import_cancel(app.state()).unwrap();
    }

    /// `import-pause-cancel` design D5: the control commands lock only
    /// `import_control`, never the library — proven here by holding the
    /// library's own mutex on another thread and confirming `import_cancel`
    /// still returns promptly rather than queuing up behind it.
    #[test]
    fn import_cancel_returns_without_waiting_for_the_library() {
        let (_dir, app) = app_with_library();
        *lock(&app.state::<AppState>().import_control) =
            Some(Arc::new(import::ImportControl::default()));

        let library = app.state::<AppState>().library.clone();
        let (locked, confirm_locked) = mpsc::channel();
        let (release, wait_for_release) = mpsc::channel();
        let holder = std::thread::spawn(move || {
            let _guard = library
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            locked.send(()).unwrap();
            let _ = wait_for_release.recv();
        });
        confirm_locked.recv_timeout(A_LOCK_IS_NOT_COMING).unwrap();

        let handle = app.handle().clone();
        let (done, answer) = mpsc::channel();
        std::thread::spawn(move || {
            done.send(import_cancel(handle.state())).unwrap();
        });
        answer
            .recv_timeout(A_LOCK_IS_NOT_COMING)
            .expect("import_cancel must not wait for the library lock")
            .unwrap();

        release.send(()).unwrap();
        holder.join().unwrap();
    }

    /// `import-pause-cancel` task 1.7 (design D7): closing the library cancels
    /// a run parked at the checkpoint, so a paused import cannot hold the app
    /// open. The item in flight when Pause was pressed is still counted.
    #[test]
    fn closing_the_library_wakes_a_paused_import_and_ends_it() {
        let (_dir, app) = app_with_library();
        let images = folder_of_images(3);
        let control = Arc::new(import::ImportControl::default());
        // Paused before the run starts: the run's own first checkpoint call
        // is guaranteed to observe `Paused`, never a race against `pause`.
        control.pause();
        *lock(&app.state::<AppState>().import_control) = Some(control.clone());

        let library = app.state::<AppState>().library.clone();
        let paths = vec![images.path().to_path_buf()];
        let run_control = control.clone();
        let (first_item_done, confirm_first_item) = mpsc::channel();
        let run = std::thread::spawn(move || {
            import::import_paths(&library, &paths, &run_control, &mut |progress| {
                if progress.done == 1 {
                    let _ = first_item_done.send(());
                }
            })
        });
        confirm_first_item
            .recv_timeout(A_LOCK_IS_NOT_COMING)
            .unwrap();

        now(close_library(app.handle().clone(), app.state())).unwrap();

        let (finished, report) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = finished.send(run.join().unwrap());
        });
        let report = report
            .recv_timeout(A_LOCK_IS_NOT_COMING)
            .expect("closing the library must wake the paused run rather than hang it")
            .expect("a cancelled run still answers with a report, not an error");

        assert_eq!(
            report.imported, 1,
            "the item in flight when the library closed is still counted"
        );
        assert!(report.cancelled);
    }

    /// `import-pause-cancel` design D7, the blocker found in review: a
    /// library *switch* — `open_library`/`pick_library`'s path, through
    /// `open_into_state` — tears the old library down exactly as much as
    /// `close_library` does. The run holds no lock at its checkpoint (design
    /// D13), so without a cancel here it would carry on writing whatever was
    /// left of the old run straight into the library it got switched to.
    #[test]
    fn switching_the_library_wakes_a_paused_import_and_ends_it() {
        let (_first_dir, app) = app_with_library();
        let images = folder_of_images(3);
        let control = Arc::new(import::ImportControl::default());
        control.pause();
        *lock(&app.state::<AppState>().import_control) = Some(control.clone());

        let old_library = app.state::<AppState>().library.clone();
        let paths = vec![images.path().to_path_buf()];
        let run_control = control.clone();
        let (first_item_done, confirm_first_item) = mpsc::channel();
        let run = std::thread::spawn(move || {
            import::import_paths(&old_library, &paths, &run_control, &mut |progress| {
                if progress.done == 1 {
                    let _ = first_item_done.send(());
                }
            })
        });
        confirm_first_item
            .recv_timeout(A_LOCK_IS_NOT_COMING)
            .unwrap();

        let second_dir = tempfile::tempdir().unwrap();
        open(&app, second_dir.path()).unwrap();

        let (finished, report) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = finished.send(run.join().unwrap());
        });
        let report = report
            .recv_timeout(A_LOCK_IS_NOT_COMING)
            .expect("switching the library must wake the paused run rather than hang it")
            .expect("a cancelled run still answers with a report, not an error");

        assert_eq!(
            report.imported, 1,
            "the item in flight when the library switched is still counted"
        );
        assert!(report.cancelled);

        let new_library = app.state::<AppState>().library.clone();
        assert_eq!(
            with_library(&new_library, |library| library.image_count()).unwrap(),
            0,
            "the run must have stopped rather than going on to write into the library it was switched to"
        );
    }

    #[test]
    fn search_returns_the_page_and_its_total() {
        let (_library, app) = app_with_library();
        import(&app, &folder_of_images(3));

        let result = now(search(
            SearchRequest {
                limit: 2,
                ..everything()
            },
            app.state(),
        ))
        .unwrap();

        assert_eq!(result.total, 3);
        assert_eq!(result.images.len(), 2);
    }

    #[test]
    fn search_reports_a_file_that_vanished_since_it_was_imported() {
        let (library, app) = app_with_library();
        import(&app, &folder_of_images(1));
        let id = ids_in_library(&app).remove(0);
        let paths = crate::library::LibraryPaths {
            root: library.path().to_path_buf(),
        };
        std::fs::remove_file(paths.image_path(&id, "png")).unwrap();

        let result = search_all(&app);

        assert!(
            result.images[0].missing,
            "the flag must be re-checked on the page being returned"
        );
    }

    #[test]
    fn image_counts_groups_the_import_under_local() {
        let (_library, app) = app_with_library();
        import(&app, &folder_of_images(2));

        let counts = now(image_counts(app.state())).unwrap();

        assert_eq!(counts.total, 2);
        assert_eq!(counts.local, 2);
        assert_eq!(counts.extension, 0);
    }

    #[test]
    fn trash_restore_and_delete_forever_reach_the_library_through_the_commands() {
        let (_library, app) = app_with_library();
        import(&app, &folder_of_images(3));
        let ids = ids_in_library(&app);

        now(trash_images(vec![ids[0].clone()], app.state())).unwrap();
        assert_eq!(now(trash_count(app.state())).unwrap(), 1);
        assert!(!ids_in_library(&app).contains(&ids[0]));

        now(restore_images(vec![ids[0].clone()], app.state())).unwrap();
        assert_eq!(now(trash_count(app.state())).unwrap(), 0);
        assert!(ids_in_library(&app).contains(&ids[0]));

        now(trash_images(vec![ids[1].clone()], app.state())).unwrap();
        let report = now(delete_forever(vec![ids[1].clone()], app.state())).unwrap();
        assert_eq!(report.deleted, 1);
        assert_eq!(now(trash_count(app.state())).unwrap(), 0);
        assert_eq!(
            status_of(&app).image_count,
            2,
            "ids[0] restored and ids[2] untouched; only ids[1] is gone",
        );
    }

    #[test]
    fn empty_trash_reaches_the_library_through_the_command() {
        let (_library, app) = app_with_library();
        import(&app, &folder_of_images(2));
        let ids = ids_in_library(&app);
        now(trash_images(ids.clone(), app.state())).unwrap();

        let report = now(empty_trash(app.state())).unwrap();

        assert_eq!(report.deleted, 2);
        assert_eq!(now(trash_count(app.state())).unwrap(), 0);
    }

    #[test]
    fn the_trash_commands_need_a_library_before_they_answer() {
        let app = mock_app();

        let errors = [
            now(trash_images(vec!["a".to_string()], app.state())).unwrap_err(),
            now(restore_images(vec!["a".to_string()], app.state())).unwrap_err(),
            now(delete_forever(vec!["a".to_string()], app.state())).unwrap_err(),
            now(empty_trash(app.state())).unwrap_err(),
            now(trash_count(app.state())).unwrap_err(),
        ];

        for error in errors {
            assert!(matches!(error, AppError::NoLibrary), "{error:?}");
        }
    }

    fn tag(app: &App, id: &str, tags: &[&str]) -> ImageRecord {
        now(update_tags(
            id.to_string(),
            tags.iter().map(|tag| (*tag).to_string()).collect(),
            app.state(),
        ))
        .unwrap()
    }

    #[test]
    fn an_edit_answers_with_the_row_it_changed() {
        let (_library, app) = app_with_library();
        import(&app, &folder_of_images(1));
        let id = ids_in_library(&app).remove(0);

        let tagged = tag(&app, &id, &["cat", "rating:e"]);
        assert_eq!(tagged.tags, vec!["cat".to_string()]);
        assert_eq!(tagged.rating.as_deref(), Some("e"));

        let rated = now(set_rating(id.clone(), Some("s".to_string()), app.state())).unwrap();
        assert_eq!(rated.rating.as_deref(), Some("s"));

        let cleared = now(set_rating(id, None, app.state())).unwrap();
        assert_eq!(cleared.rating, None);
    }

    #[test]
    fn update_facts_answers_with_the_row_it_changed_and_refuses_a_bad_address() {
        let (_library, app) = app_with_library();
        import(&app, &folder_of_images(1));
        let id = ids_in_library(&app).remove(0);

        let edit = FactsEdit {
            page_title: Some("a title".to_string()),
            page_url: Some("https://x.com/alice/status/9".to_string()),
            image_url: None,
        };
        let record = now(update_facts(id.clone(), edit, app.state())).unwrap();
        assert_eq!(record.page_title.as_deref(), Some("a title"));
        assert_eq!(record.account.as_deref(), Some("alice"));

        let bad = FactsEdit {
            page_title: None,
            page_url: Some("not a url".to_string()),
            image_url: None,
        };
        let error = now(update_facts(id, bad, app.state())).unwrap_err();
        assert!(matches!(error, AppError::BadRequest(_)), "{error:?}");
    }

    #[test]
    fn suggestions_and_counts_read_the_open_library() {
        let (_library, app) = app_with_library();
        import(&app, &folder_of_images(2));
        let ids = ids_in_library(&app);
        tag(&app, &ids[0], &["cat"]);
        tag(&app, &ids[1], &["cat", "cathedral", "rating:s"]);

        let suggested = now(tag_suggestions("cat".to_string(), 8, app.state())).unwrap();
        let names: Vec<&str> = suggested.iter().map(|tag| tag.name.as_str()).collect();
        assert_eq!(names, vec!["cat", "cathedral"], "most used first");

        let counts = now(tag_counts(everything(), app.state())).unwrap();
        assert_eq!(counts.tags.first().map(|tag| tag.count), Some(2));
        assert_eq!(counts.ratings.s, 1);
        assert_eq!(counts.ratings.unrated, 1);
    }

    #[test]
    fn an_edit_naming_no_image_is_not_found() {
        let (_library, app) = app_with_library();

        for error in [
            now(update_tags(
                "no-such-id".to_string(),
                vec!["cat".to_string()],
                app.state(),
            ))
            .unwrap_err(),
            now(set_rating(
                "no-such-id".to_string(),
                Some("s".to_string()),
                app.state(),
            ))
            .unwrap_err(),
            now(update_facts(
                "no-such-id".to_string(),
                FactsEdit {
                    page_title: Some("title".to_string()),
                    page_url: None,
                    image_url: None,
                },
                app.state(),
            ))
            .unwrap_err(),
        ] {
            assert!(matches!(error, AppError::NotFound(_)), "{error:?}");
        }
    }

    /// Design D11: the value comes from this app's own UI, so one that is not a
    /// rating is a bug here rather than a foreign library's row.
    #[test]
    fn a_rating_outside_the_alphabet_is_refused() {
        let (_library, app) = app_with_library();
        import(&app, &folder_of_images(1));
        let id = ids_in_library(&app).remove(0);

        let error = now(set_rating(id, Some("safe".to_string()), app.state())).unwrap_err();

        assert!(matches!(error, AppError::BadRequest(_)), "{error:?}");
    }

    #[test]
    fn the_tag_commands_need_a_library_before_they_answer() {
        let app = mock_app();

        let errors = [
            now(update_tags("a".to_string(), vec![], app.state())).unwrap_err(),
            now(set_rating("a".to_string(), None, app.state())).unwrap_err(),
            now(tag_suggestions("cat".to_string(), 8, app.state())).unwrap_err(),
            now(tag_counts(everything(), app.state())).unwrap_err(),
            now(tag_vocabulary(app.state())).unwrap_err(),
            now(set_tag_category(
                "a".to_string(),
                TagCategory::Artist,
                app.state(),
            ))
            .unwrap_err(),
            now(set_tag_pinned("a".to_string(), true, app.state())).unwrap_err(),
        ];

        for error in errors {
            assert!(matches!(error, AppError::NoLibrary), "{error:?}");
        }
    }

    #[test]
    fn search_ids_matches_the_ids_search_would_page() {
        let (_library, app) = app_with_library();
        import(&app, &folder_of_images(5));

        let req = SearchRequest {
            limit: 2,
            offset: 1,
            ..everything()
        };
        let paged = now(search(req.clone(), app.state())).unwrap();
        let ids = now(search_ids(req, app.state())).unwrap();

        assert_eq!(
            ids,
            paged
                .images
                .iter()
                .map(|image| image.id.clone())
                .collect::<Vec<_>>(),
        );
    }

    #[test]
    fn matching_ids_answers_the_subset_the_request_still_matches() {
        let (_library, app) = app_with_library();
        import(&app, &folder_of_images(5));
        let ids = ids_in_library(&app);

        let candidates = vec![ids[0].clone(), ids[1].clone(), "no-such-id".to_string()];
        let matched = now(matching_ids(everything(), candidates, app.state())).unwrap();

        assert_eq!(matched.len(), 2);
        assert!(matched.contains(&ids[0]));
        assert!(matched.contains(&ids[1]));
    }

    #[test]
    fn search_position_agrees_with_the_row_search_ids_pages_it_at() {
        let (_library, app) = app_with_library();
        import(&app, &folder_of_images(5));
        let ids = ids_in_library(&app);
        let third = ids[2].clone();

        let position = now(search_position(everything(), third, app.state())).unwrap();

        assert_eq!(position, Some(2));

        let missing = now(search_position(
            everything(),
            "no-such-id".to_string(),
            app.state(),
        ))
        .unwrap();
        assert_eq!(missing, None);
    }

    #[test]
    fn apply_edit_reaches_every_selected_image_through_the_command() {
        let (_library, app) = app_with_library();
        import(&app, &folder_of_images(2));
        let ids = ids_in_library(&app);

        let records = now(apply_edit(
            ids.clone(),
            TagEditSpec {
                add: vec!["cat".to_string()],
                ..Default::default()
            },
            app.state(),
        ))
        .unwrap();

        assert!(
            records
                .iter()
                .all(|image| image.tags.contains(&"cat".to_string())),
            "{records:?}",
        );
        let result = search_all(&app);
        assert!(
            result
                .images
                .iter()
                .all(|image| image.tags.contains(&"cat".to_string())),
            "{:?}",
            result.images,
        );
    }

    #[test]
    fn bulk_set_rating_reaches_every_selected_image_through_the_command() {
        let (_library, app) = app_with_library();
        import(&app, &folder_of_images(2));
        let ids = ids_in_library(&app);

        now(bulk_set_rating(
            ids.clone(),
            Some("e".to_string()),
            app.state(),
        ))
        .unwrap();

        let result = search_all(&app);
        assert!(
            result
                .images
                .iter()
                .all(|image| image.rating.as_deref() == Some("e")),
            "{:?}",
            result.images,
        );
    }

    #[test]
    fn selection_tag_counts_reaches_the_open_library_through_the_command() {
        let (_library, app) = app_with_library();
        import(&app, &folder_of_images(2));
        let ids = ids_in_library(&app);
        tag(&app, &ids[0], &["cat"]);
        tag(&app, &ids[1], &["cat"]);

        let counts = now(selection_tag_counts(ids, 10, None, app.state())).unwrap();

        assert_eq!(
            counts.first().map(|tag| (tag.name.as_str(), tag.count)),
            Some(("cat", 2)),
        );
    }

    #[test]
    fn export_zip_writes_the_selected_originals_through_the_command() {
        let (_library, app) = app_with_library();
        import(&app, &folder_of_images(2));
        let ids = ids_in_library(&app);
        let out = tempfile::tempdir().unwrap();
        let path = out.path().join("export.zip");

        let report = now(export_zip(
            ids.clone(),
            path.display().to_string(),
            0,
            app.handle().clone(),
            app.state(),
        ))
        .unwrap();

        assert_eq!(report.written, 2);
        assert!(report.missing.is_empty());
        assert!(path.is_file());
    }

    /// The owner's Human update on task 6.1 ("selection-and-bulk"): a
    /// UUID-only name has no order, and every entry carried the crate's
    /// `1980-01-01` default mtime regardless of when the image was
    /// captured. This pins the fix through the command: a tagged image's
    /// entry is named `<id> <tag1> <tag2>…<ext>` (tags alphabetical, the
    /// image's stored order), an untagged one keeps `<id>.<ext>`, and every
    /// entry's mtime is the image's own `captured_at`, not the default.
    #[test]
    fn export_zip_names_entries_by_id_and_tags_and_dates_them_by_captured_at() {
        let (_library, app) = app_with_library();
        import(&app, &folder_of_images(2));
        let ids = ids_in_library(&app);
        tag(&app, &ids[0], &["skirt_lift", "kani_biimu"]);
        let captured_at = search_all(&app)
            .images
            .into_iter()
            .find(|image| image.id == ids[0])
            .unwrap()
            .captured_at;
        let out = tempfile::tempdir().unwrap();
        let path = out.path().join("export.zip");

        now(export_zip(
            ids.clone(),
            path.display().to_string(),
            0,
            app.handle().clone(),
            app.state(),
        ))
        .unwrap();

        let mut archive = zip::ZipArchive::new(std::fs::File::open(&path).unwrap()).unwrap();
        // Tags come back alphabetical (the image's stored order,
        // `ingest::load_records`'s `ORDER BY tags.name`), not insertion order.
        let tagged_entry = archive
            .by_name(&format!("{} kani_biimu skirt_lift.png", ids[0]))
            .expect("a tagged image's entry is named `<id> <tag1> <tag2>….<ext>`");
        let expected = time::OffsetDateTime::from_unix_timestamp(captured_at.div_euclid(1000))
            .unwrap()
            .date();
        let mtime = tagged_entry.last_modified().unwrap();
        assert_eq!(
            (mtime.year(), mtime.month(), mtime.day()),
            (
                expected.year() as u16,
                u8::from(expected.month()),
                expected.day()
            ),
            "the entry's mtime must be the image's captured_at, not the zip default",
        );
        assert_ne!(
            mtime,
            zip::DateTime::default(),
            "captured_at is well past 1980, so the entry must not fall back to the crate default",
        );
        drop(tagged_entry);

        assert!(
            archive.by_name(&format!("{}.png", ids[1])).is_ok(),
            "an untagged image keeps the `<id>.<ext>` name",
        );
    }

    #[test]
    fn the_new_bulk_and_export_commands_need_a_library_before_they_answer() {
        let app = mock_app();

        let errors = [
            now(search_ids(everything(), app.state())).unwrap_err(),
            now(apply_edit(
                vec!["a".to_string()],
                TagEditSpec::default(),
                app.state(),
            ))
            .unwrap_err(),
            now(bulk_set_rating(vec!["a".to_string()], None, app.state())).unwrap_err(),
            now(selection_tag_counts(
                vec!["a".to_string()],
                10,
                None,
                app.state(),
            ))
            .unwrap_err(),
            now(export_zip(
                vec!["a".to_string()],
                "/tmp/export.zip".to_string(),
                0,
                app.handle().clone(),
                app.state(),
            ))
            .unwrap_err(),
        ];

        for error in errors {
            assert!(matches!(error, AppError::NoLibrary), "{error:?}");
        }
    }

    #[test]
    fn thumbnail_path_generates_the_file_on_demand() {
        let (_library, app) = app_with_library();
        import(&app, &folder_of_images(1));
        let id = ids_in_library(&app).remove(0);
        let path = PathBuf::from(now(thumbnail_path(id.clone(), app.state())).unwrap());
        std::fs::remove_file(&path).unwrap();

        let again = now(thumbnail_path(id, app.state())).unwrap();

        assert_eq!(PathBuf::from(&again), path);
        assert!(path.is_file());
    }

    #[test]
    fn a_thumbnail_for_an_unknown_image_is_not_found() {
        let (_library, app) = app_with_library();

        let error = now(thumbnail_path("no-such-id".to_string(), app.state())).unwrap_err();

        assert!(matches!(error, AppError::NotFound(_)), "{error:?}");
    }

    #[test]
    fn commands_needing_a_library_say_so_before_one_is_open() {
        let app = mock_app();

        let error = now(search(everything(), app.state())).unwrap_err();

        assert!(matches!(error, AppError::NoLibrary), "{error:?}");
    }

    #[test]
    fn import_outcomes_name_a_file_that_is_not_an_image() {
        let (_library, app) = app_with_library();
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("notes.txt"), b"not an image").unwrap();

        let report = import(&app, &dir);

        assert_eq!(report.imported, 0);
        assert_eq!(report.items.len(), 1);
        assert_ne!(report.items[0].status, ImportStatus::Imported);
    }

    /// The webview loads every thumbnail from the dotted `.thumbs/`, and Tauri's
    /// fs scope sets `require_literal_leading_dot` on unix, so a recursive grant
    /// over the library root matches nothing under `.thumbs/` — every thumbnail
    /// in the grid comes back 403 with nothing in the Rust log to show for it.
    /// Naming the directory in the pattern is the fix, so that is what is
    /// asserted: the mock runtime's scope does not apply the dotfile rule, so
    /// asserting `is_allowed` here would pass against the broken grant too.
    #[test]
    fn the_asset_scope_names_the_dotted_thumbnail_directory() {
        let (library, app) = app_with_library();
        let root = library.path().display().to_string();

        let patterns: Vec<String> = app
            .handle()
            .asset_protocol_scope()
            .allowed_patterns()
            .iter()
            .map(|pattern| pattern.as_str().to_string())
            .collect();

        assert!(
            patterns.iter().any(|p| p.contains("/.thumbs/")),
            "no pattern reaches .thumbs: {patterns:?}",
        );
        assert!(
            patterns.iter().any(|p| p.contains("/images/")),
            "no pattern reaches images: {patterns:?}",
        );
        assert!(
            !patterns.iter().any(|p| *p == format!("{root}/**")),
            "granting the root exposes library.sqlite and inbox: {patterns:?}",
        );
    }

    /// `setup()` reopens the remembered path exactly this way. It used to pass
    /// `CreateIfMissing`, so a library folder the user had deleted, renamed or
    /// not yet mounted was recreated empty and reported open — the images were
    /// simply gone from the app's point of view, and new captures landed in the
    /// replacement.
    #[test]
    fn reopening_a_library_folder_that_is_gone_leaves_it_closed_and_missing() {
        let dir = tempfile::tempdir().unwrap();
        let gone = dir.path().join("was-a-library");
        let app = mock_app();
        let state = app.state::<AppState>();
        lock(&state.settings).library_path = Some(gone.clone());

        let result = open_into_state(app.handle(), &state, &gone, OpenMode::ExistingOnly);

        assert!(result.is_err(), "a library that is not there must not open");
        assert!(!gone.exists(), "and must not be conjured up at that path");

        let status = status_of(&app);
        assert!(!status.opened);
        assert_eq!(status.missing_path, Some(gone.display().to_string()));
    }

    /// Design D13. No unit test can see the app's main thread being free — the
    /// mock runtime has no run loop — so what is pinned is the other half: the
    /// command really does wait for the mutex, and answers once it is released
    /// rather than failing or racing past it. The freedom of the main thread
    /// follows from the wait happening on a blocking thread, which the shape of
    /// `off_main_thread` is what guarantees.
    #[test]
    fn a_search_answers_after_the_thread_holding_the_library_lets_go() {
        let (_library, app) = app_with_library();
        import(&app, &folder_of_images(1));
        let library = app.state::<AppState>().library.clone();
        let released = Arc::new(AtomicBool::new(false));

        let (locked, is_locked) = std::sync::mpsc::channel();
        let holder = {
            let released = released.clone();
            std::thread::spawn(move || {
                with_library(&library, |_| {
                    locked.send(()).unwrap();
                    std::thread::sleep(Duration::from_millis(300));
                    released.store(true, Ordering::SeqCst);
                    Ok(())
                })
                .unwrap();
            })
        };
        is_locked.recv().unwrap();

        let result = search_all(&app);

        assert!(
            released.load(Ordering::SeqCst),
            "the search answered out of a library another thread was holding",
        );
        assert_eq!(result.total, 1);
        holder.join().unwrap();
    }

    fn new_rule(name: &str, pattern: &str, tags: &[&str]) -> crate::model::RuleInput {
        crate::model::RuleInput {
            id: None,
            name: name.to_string(),
            pattern: pattern.to_string(),
            is_regex: false,
            tags: tags.iter().map(|tag| (*tag).to_string()).collect(),
            enabled: true,
        }
    }

    #[test]
    fn rules_list_upsert_and_delete_reach_the_open_library_through_the_commands() {
        let (_library, app) = app_with_library();

        let created = now(rules_upsert(
            new_rule("pixiv", "pixiv", &["pixiv"]),
            app.state(),
        ))
        .unwrap();
        let listed = now(rules_list(app.state())).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].rule, created);
        assert_eq!(listed[0].pattern_error, None);

        now(rules_delete(created.id, app.state())).unwrap();
        assert!(now(rules_list(app.state())).unwrap().is_empty());
    }

    /// Design D6: the command surfaces the engine's reason, and creates
    /// nothing.
    #[test]
    fn rules_upsert_returns_the_refusal_reason() {
        let (_library, app) = app_with_library();

        let error = now(rules_upsert(
            crate::model::RuleInput {
                is_regex: true,
                ..new_rule("broken", "(unterminated", &["x"])
            },
            app.state(),
        ))
        .unwrap_err();

        assert!(matches!(error, AppError::BadRequest(_)), "{error:?}");
        assert!(now(rules_list(app.state())).unwrap().is_empty());
    }

    #[test]
    fn rules_run_reaches_the_open_library_and_reports_progress() {
        let (_library, app) = app_with_library();
        now(rules_upsert(
            new_rule("pixiv", "pixiv", &["pixiv"]),
            app.state(),
        ))
        .unwrap();
        import(&app, &folder_of_images(1));
        let ticks = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let seen = ticks.clone();
        app.listen(RULES_PROGRESS_EVENT, move |event| {
            let progress: ExportProgress = serde_json::from_str(event.payload()).unwrap();
            lock(&seen).push(progress);
        });

        let report = now(rules_run(app.handle().clone(), app.state())).unwrap();

        assert_eq!(report.examined, 1);
        let ticks = lock(&ticks).clone();
        assert_eq!(
            ticks.last().map(|last| (last.done, last.total)),
            Some((1, 1)),
        );
    }

    #[test]
    fn the_rules_commands_need_a_library_before_they_answer() {
        let app = mock_app();

        for error in [
            now(rules_list(app.state())).unwrap_err(),
            now(rules_upsert(new_rule("a", "a", &["a"]), app.state())).unwrap_err(),
            now(rules_delete("a".to_string(), app.state())).unwrap_err(),
            now(rules_run(app.handle().clone(), app.state())).unwrap_err(),
            now(rules_export("/tmp/rules.json".to_string(), app.state())).unwrap_err(),
            now(rules_import("/tmp/rules.json".to_string(), app.state())).unwrap_err(),
        ] {
            assert!(matches!(error, AppError::NoLibrary), "{error:?}");
        }
    }

    #[test]
    fn export_written_to_a_temp_path_re_imports_with_everything_skipped() {
        let (_library, app) = app_with_library();
        now(rules_upsert(
            new_rule("pixiv", "pixiv", &["pixiv"]),
            app.state(),
        ))
        .unwrap();
        let out = tempfile::tempdir().unwrap();
        let path = out.path().join("rules.json");

        now(rules_export(path.display().to_string(), app.state())).unwrap();
        let report = now(rules_import(path.display().to_string(), app.state())).unwrap();

        assert_eq!(report.imported, 0);
        assert_eq!(report.skipped, 1);
        assert_eq!(now(rules_list(app.state())).unwrap().len(), 1);
    }

    #[test]
    fn importing_a_missing_path_errors_without_touching_the_library() {
        let (_library, app) = app_with_library();
        now(rules_upsert(
            new_rule("pixiv", "pixiv", &["pixiv"]),
            app.state(),
        ))
        .unwrap();

        let error = now(rules_import("/no/such/file.json".to_string(), app.state())).unwrap_err();

        assert!(matches!(error, AppError::Io(_)), "{error:?}");
        assert_eq!(now(rules_list(app.state())).unwrap().len(), 1);
    }

    #[test]
    fn collection_list_starts_with_favorites_through_the_command() {
        let (_library, app) = app_with_library();

        let listed = now(collection_list(app.state())).unwrap();

        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, "favorites");
        assert_eq!(listed[0].name, "Favorites");
    }

    #[test]
    fn collection_create_rename_and_delete_reach_the_open_library_through_the_commands() {
        let (_library, app) = app_with_library();

        let created = now(collection_create("To upload".to_string(), app.state())).unwrap();
        assert_eq!(created.slug, "to_upload");
        assert_eq!(now(collection_list(app.state())).unwrap().len(), 2);

        let renamed = now(collection_rename(
            created.id.clone(),
            "Queue".to_string(),
            app.state(),
        ))
        .unwrap();
        assert_eq!(renamed.id, created.id);
        assert_eq!(renamed.name, "Queue");

        now(collection_delete(created.id, app.state())).unwrap();
        assert_eq!(now(collection_list(app.state())).unwrap().len(), 1);
    }

    fn new_stamp(name: &str, text: &str) -> StampInput {
        StampInput {
            id: None,
            name: name.to_string(),
            text: text.to_string(),
        }
    }

    #[test]
    fn stamps_list_upsert_and_delete_reach_the_open_library_through_the_commands() {
        let (_library, app) = app_with_library();

        let created = now(stamps_upsert(new_stamp("Cat", "cat animal"), app.state())).unwrap();
        let listed = now(stamps_list(app.state())).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0], created);

        now(stamps_delete(created.id, app.state())).unwrap();
        assert!(now(stamps_list(app.state())).unwrap().is_empty());
    }

    /// Design D3: the command surfaces the store's refusal reason, and
    /// creates nothing.
    #[test]
    fn stamps_upsert_returns_the_refusal_reason() {
        let (_library, app) = app_with_library();

        let error = now(stamps_upsert(new_stamp("Cat", "   "), app.state())).unwrap_err();

        assert!(matches!(error, AppError::BadRequest(_)), "{error:?}");
        assert!(now(stamps_list(app.state())).unwrap().is_empty());
    }

    #[test]
    fn the_stamps_commands_need_a_library_before_they_answer() {
        let app = mock_app();

        for error in [
            now(stamps_list(app.state())).unwrap_err(),
            now(stamps_upsert(new_stamp("a", "a"), app.state())).unwrap_err(),
            now(stamps_delete("a".to_string(), app.state())).unwrap_err(),
        ] {
            assert!(matches!(error, AppError::NoLibrary), "{error:?}");
        }
    }

    #[test]
    fn tag_vocabulary_starts_empty_and_lists_a_categorised_tag_through_the_command() {
        let (_library, app) = app_with_library();
        import(&app, &folder_of_images(1));
        let ids = ids_in_library(&app);

        assert!(now(tag_vocabulary(app.state())).unwrap().is_empty());

        tag(&app, &ids[0], &["artist:kantoku"]);

        let entries = now(tag_vocabulary(app.state())).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "kantoku");
        assert_eq!(entries[0].category, TagCategory::Artist);
    }

    #[test]
    fn set_tag_category_and_set_tag_pinned_reach_the_open_library_through_the_commands() {
        let (_library, app) = app_with_library();
        import(&app, &folder_of_images(1));
        let ids = ids_in_library(&app);
        tag(&app, &ids[0], &["azur_lane"]);

        let entries = now(set_tag_category(
            "azur_lane".to_string(),
            TagCategory::Copyright,
            app.state(),
        ))
        .unwrap();
        assert_eq!(entries[0].category, TagCategory::Copyright);

        let entries = now(set_tag_pinned("azur_lane".to_string(), true, app.state())).unwrap();
        assert!(entries[0].pinned);

        let entries = now(set_tag_pinned("azur_lane".to_string(), false, app.state())).unwrap();
        assert_eq!(entries[0].category, TagCategory::Copyright);
        assert!(!entries[0].pinned);
    }

    #[test]
    fn set_tag_category_for_an_unknown_tag_is_refused_through_the_command() {
        let (_library, app) = app_with_library();

        let error = now(set_tag_category(
            "nobody".to_string(),
            TagCategory::Artist,
            app.state(),
        ))
        .unwrap_err();

        assert!(matches!(error, AppError::NotFound(_)), "{error:?}");
    }

    #[test]
    fn collection_add_and_remove_reach_every_selected_image_through_the_commands() {
        let (_library, app) = app_with_library();
        import(&app, &folder_of_images(2));
        let ids = ids_in_library(&app);

        let added = now(collection_add(
            ids.clone(),
            "favorites".to_string(),
            app.state(),
        ))
        .unwrap();

        assert_eq!(added.len(), 2);
        assert!(
            added
                .iter()
                .all(|record| record.collections == vec!["favorites".to_string()]),
            "{added:?}",
        );

        let removed = now(collection_remove(ids, "favorites".to_string(), app.state())).unwrap();

        assert!(
            removed.iter().all(|record| record.collections.is_empty()),
            "{removed:?}",
        );
    }

    #[test]
    fn the_collections_commands_need_a_library_before_they_answer() {
        let app = mock_app();

        for error in [
            now(collection_list(app.state())).unwrap_err(),
            now(collection_create("Queue".to_string(), app.state())).unwrap_err(),
            now(collection_rename(
                "id".to_string(),
                "Queue".to_string(),
                app.state(),
            ))
            .unwrap_err(),
            now(collection_delete("id".to_string(), app.state())).unwrap_err(),
            now(collection_add(vec![], "id".to_string(), app.state())).unwrap_err(),
            now(collection_remove(vec![], "id".to_string(), app.state())).unwrap_err(),
        ] {
            assert!(matches!(error, AppError::NoLibrary), "{error:?}");
        }
    }

    #[test]
    fn the_note_round_trips_through_the_commands() {
        let (_library, app) = app_with_library();

        assert_eq!(now(note_get(app.state())).unwrap(), Note::default());

        now(note_set("still to sort".to_string(), app.state())).unwrap();

        assert_eq!(now(note_get(app.state())).unwrap().content, "still to sort");
    }

    #[test]
    fn the_note_commands_need_a_library_before_they_answer() {
        let app = mock_app();

        for error in [
            now(note_get(app.state())).unwrap_err(),
            now(note_set(String::new(), app.state())).unwrap_err(),
        ] {
            assert!(matches!(error, AppError::NoLibrary), "{error:?}");
        }
    }

    /// The panel's fold is a preference, so it has to survive the process, not
    /// just the running state (`notes` design D13).
    #[test]
    fn the_notes_fold_reaches_the_state_and_the_store() {
        let app = mock_app();

        let answered = set_notes_collapsed(true, app.handle().clone(), app.state()).unwrap();

        assert!(answered.notes_collapsed);
        assert!(app_settings(app.state()).notes_collapsed);
        assert!(settings::load(app.handle()).notes_collapsed);
    }

    /// The collections section's fold is a preference, so it has to survive
    /// the process too, the same as the notes panel's own test.
    #[test]
    fn the_collections_fold_reaches_the_state_and_the_store() {
        let app = mock_app();

        let answered = set_collections_collapsed(true, app.handle().clone(), app.state()).unwrap();

        assert!(answered.collections_collapsed);
        assert!(app_settings(app.state()).collections_collapsed);
        assert!(settings::load(app.handle()).collections_collapsed);
    }

    // ---- damage, the backfill and the rebuild (`library-sidecars` tasks
    // 2.8, 2.9) ----

    /// Bytes overwritten well past the header, so `db::open`'s `quick_check`
    /// refuses the file as `LibraryCorrupt` rather than `NotADatabase` — the
    /// same corruption `db.rs`'s own test exercises, reused here to drive the
    /// command layer's response to it.
    fn corrupt_db(path: &Path) {
        let mut bytes = std::fs::read(path).unwrap();
        assert!(
            bytes.len() > 4200,
            "the schema must have grown past one page"
        );
        for byte in &mut bytes[4096..4200] {
            *byte ^= 0xff;
        }
        std::fs::write(path, bytes).unwrap();
    }

    /// `library-sidecars` task 2.8: reopening a damaged remembered library
    /// leaves `damagedPath` set and `missingPath` null.
    #[test]
    fn a_damaged_remembered_library_leaves_damaged_path_set_and_missing_path_null() {
        let (dir, app) = app_with_library();
        *lock(&app.state::<AppState>().library) = None;
        corrupt_db(&dir.path().join("library.sqlite"));

        let state = app.state::<AppState>();
        let error = open_into_state(
            &app.handle().clone(),
            &state,
            dir.path(),
            OpenMode::ExistingOnly,
        )
        .unwrap_err();
        assert!(
            matches!(error, AppError::LibraryCorrupt { .. }),
            "{error:?}"
        );

        let status = status_of(&app);
        assert_eq!(status.damaged_path, Some(dir.path().display().to_string()));
        assert_eq!(status.missing_path, None);
        assert_eq!(status.newer_path, None);
    }

    /// `library-sidecars` task 2.8: a folder that is simply gone leaves
    /// `missingPath` set and `damagedPath` null.
    #[test]
    fn a_folder_that_is_simply_gone_leaves_missing_path_set_and_damaged_path_null() {
        let (dir, app) = app_with_library();
        let gone = dir.path().to_path_buf();
        *lock(&app.state::<AppState>().library) = None;
        drop(dir);

        let state = app.state::<AppState>();
        open_into_state(&app.handle().clone(), &state, &gone, OpenMode::ExistingOnly).unwrap_err();

        let status = status_of(&app);
        assert_eq!(status.missing_path, Some(gone.display().to_string()));
        assert_eq!(status.damaged_path, None);
        assert_eq!(status.newer_path, None);
    }

    /// `library-sidecars` task 2.8: a library written by a newer build sets
    /// `newerPath` and neither of the other two — it is not missing, and
    /// `library-recovery`'s "A library from a newer build" says it must not
    /// be treated as damaged either, but it still has to name its folder:
    /// with all three null the start screen offers "choose a library folder"
    /// and never mentions the library the user already has.
    #[test]
    fn a_schema_too_new_library_sets_newer_path_only() {
        let (dir, app) = app_with_library();
        {
            let library = app.state::<AppState>().library.clone();
            let guard = library
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            guard
                .as_ref()
                .unwrap()
                .conn
                .pragma_update(None, "user_version", 99i64)
                .unwrap();
        }
        *lock(&app.state::<AppState>().library) = None;

        let state = app.state::<AppState>();
        let error = open_into_state(
            &app.handle().clone(),
            &state,
            dir.path(),
            OpenMode::ExistingOnly,
        )
        .unwrap_err();
        assert!(matches!(error, AppError::SchemaTooNew { .. }), "{error:?}");

        let status = status_of(&app);
        assert_eq!(status.newer_path, Some(dir.path().display().to_string()));
        assert_eq!(status.missing_path, None);
        assert_eq!(status.damaged_path, None);
    }

    /// `library-sidecars` task 2.8: opening a healthy library clears all
    /// three — proven by leaving a stale failure in `open_failure` from a
    /// previous attempt against the same path.
    #[test]
    fn opening_a_healthy_library_clears_missing_damaged_and_newer_path() {
        let dir = tempfile::tempdir().unwrap();
        let app = mock_app();
        *lock(&app.state::<AppState>().open_failure) =
            Some((dir.path().to_path_buf(), crate::OpenFailureKind::Corrupt));

        open(&app, dir.path()).unwrap();

        let status = status_of(&app);
        assert_eq!(status.missing_path, None);
        assert_eq!(status.damaged_path, None);
        assert_eq!(status.newer_path, None);
    }

    /// `library-sidecars` task 2.8: opening a library with sidecars missing
    /// emits `library:sidecars` progress and does not block the command that
    /// opened it — the pass runs on its own thread, detached from the
    /// `open_library` call.
    #[test]
    fn opening_a_library_with_missing_sidecars_emits_progress_without_blocking() {
        let dir = tempfile::tempdir().unwrap();
        {
            let library = Library::open_or_create(dir.path()).unwrap();
            for id in ["a", "b", "c"] {
                crate::ingest::store_image(
                    &library,
                    crate::ingest::IngestInput {
                        id,
                        bytes: &png_bytes(2, 2),
                        source: crate::model::ImageSource::Local,
                        source_ref: None,
                        image_url: None,
                        page_url: None,
                        page_title: None,
                        adapter: None,
                        rating: None,
                        tags: &[],
                        captured_at: 0,
                        file_modified_at: None,
                        deleted_at: None,
                    },
                )
                .unwrap();
                std::fs::remove_file(crate::sidecar::path(&library.paths, id)).unwrap();
            }
        }
        let app = mock_app();
        let (sender, receiver) = mpsc::channel();
        app.listen(SIDECARS_PROGRESS_EVENT, move |event| {
            let progress: SidecarsProgress = serde_json::from_str(event.payload()).unwrap();
            let _ = sender.send(progress);
        });

        let status = open(&app, dir.path()).unwrap();
        assert!(
            status.opened,
            "the command must return without waiting for the pass"
        );

        let mut last = None;
        while last.map(|p: SidecarsProgress| p.done) != Some(3) {
            last = Some(
                receiver
                    .recv_timeout(A_LOCK_IS_NOT_COMING)
                    .expect("the backfill must emit progress"),
            );
        }
        assert_eq!(last, Some(SidecarsProgress { done: 3, total: 3 }));
        for id in ["a", "b", "c"] {
            assert!(
                crate::sidecar::path(&Library::open_existing(dir.path()).unwrap().paths, id)
                    .is_file()
            );
        }
    }

    /// `library-sidecars` task 2.9: rebuilding the open library closes it and
    /// leaves nothing open.
    #[test]
    fn rebuilding_the_open_library_closes_it_and_leaves_nothing_open() {
        let (dir, app) = app_with_library();
        import(&app, &folder_of_images(2));

        let report = now(rebuild_library(
            dir.path().display().to_string(),
            app.handle().clone(),
            app.state(),
        ))
        .unwrap();

        assert_eq!(report.images, 2);
        assert!(!status_of(&app).opened, "rebuild must leave nothing open");
    }

    /// `library-sidecars` task 2.9: rebuilding a path that is not open works
    /// with no library open at all.
    #[test]
    fn rebuilding_a_path_that_is_not_open_works_with_no_library_open() {
        let dir = tempfile::tempdir().unwrap();
        {
            let library = Library::open_or_create(dir.path()).unwrap();
            crate::ingest::store_image(
                &library,
                crate::ingest::IngestInput {
                    id: "a",
                    bytes: &png_bytes(2, 2),
                    source: crate::model::ImageSource::Local,
                    source_ref: None,
                    image_url: None,
                    page_url: None,
                    page_title: None,
                    adapter: None,
                    rating: None,
                    tags: &[],
                    captured_at: 0,
                    file_modified_at: None,
                    deleted_at: None,
                },
            )
            .unwrap();
        }
        let app = mock_app();

        let report = now(rebuild_library(
            dir.path().display().to_string(),
            app.handle().clone(),
            app.state(),
        ))
        .unwrap();

        assert_eq!(report.images, 1);
        assert!(!status_of(&app).opened);
    }

    /// `import-pause-cancel` design D7 again, for the third door onto it: a
    /// rebuild tears the library down and renames a database onto its folder,
    /// so a run left going would write the rest of its rows into the database
    /// about to be moved aside — rows the rebuilt library would then be
    /// missing, whatever the import's own report said.
    #[test]
    fn rebuilding_wakes_a_paused_import_and_ends_it() {
        let (dir, app) = app_with_library();
        let images = folder_of_images(3);
        let control = Arc::new(import::ImportControl::default());
        control.pause();
        *lock(&app.state::<AppState>().import_control) = Some(control.clone());

        let library = app.state::<AppState>().library.clone();
        let paths = vec![images.path().to_path_buf()];
        let run_control = control.clone();
        let (first_item_done, confirm_first_item) = mpsc::channel();
        let run = std::thread::spawn(move || {
            import::import_paths(&library, &paths, &run_control, &mut |progress| {
                if progress.done == 1 {
                    let _ = first_item_done.send(());
                }
            })
        });
        confirm_first_item
            .recv_timeout(A_LOCK_IS_NOT_COMING)
            .unwrap();

        let report = now(rebuild_library(
            dir.path().display().to_string(),
            app.handle().clone(),
            app.state(),
        ))
        .unwrap();

        let (finished, import_report) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = finished.send(run.join().unwrap());
        });
        let import_report = import_report
            .recv_timeout(A_LOCK_IS_NOT_COMING)
            .expect("a rebuild must wake the paused run rather than hang it")
            .expect("a cancelled run still answers with a report, not an error");

        assert!(import_report.cancelled);
        assert_eq!(
            report.images,
            i64::from(import_report.imported),
            "the rebuilt library holds exactly what the import had landed"
        );
    }

    /// `library-sidecars` task 2.9: the progress event reaches a listener.
    #[test]
    fn rebuild_library_ticks_progress_to_a_listener() {
        let (dir, app) = app_with_library();
        import(&app, &folder_of_images(3));
        let ticks = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let seen = ticks.clone();
        app.listen(REBUILD_PROGRESS_EVENT, move |event| {
            let progress: RebuildProgress = serde_json::from_str(event.payload()).unwrap();
            lock(&seen).push(progress);
        });

        let report = now(rebuild_library(
            dir.path().display().to_string(),
            app.handle().clone(),
            app.state(),
        ))
        .unwrap();

        assert_eq!(report.images, 3);
        let ticks = lock(&ticks).clone();
        assert_eq!(
            ticks.last(),
            Some(&RebuildProgress { done: 3, total: 3 }),
            "the last tick must show the rebuild finished: {ticks:?}"
        );
    }

    // ---- booru sites and upload (`booru-upload` tasks 1.5, 2.5, 2.6) ----

    mod booru_tests {
        use serde_json::json;
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        use super::*;
        use crate::booru::credentials::InMemoryCredentials;
        use crate::model::{BooruUploadForm, CommentaryOutcome, PostRef, UploadStep};
        use crate::test_support::mock_app_with_credentials;

        fn app_with_library_and_credentials(
            credentials: std::sync::Arc<dyn Credentials>,
        ) -> (tempfile::TempDir, App) {
            let dir = tempfile::tempdir().unwrap();
            let app = mock_app_with_credentials(credentials);
            open(&app, dir.path()).unwrap();
            (dir, app)
        }

        fn upload_form(tags: &[&str], commentary_title: &str) -> BooruUploadForm {
            BooruUploadForm {
                tags: tags.iter().map(|tag| (*tag).to_string()).collect(),
                rating: "s".to_string(),
                source: "https://example.test/p".to_string(),
                artist: String::new(),
                commentary_title: commentary_title.to_string(),
                commentary_body: String::new(),
            }
        }

        fn posts_of(app: &App, image_id: &str) -> Vec<PostRef> {
            with_library(&app.state::<AppState>().library, |library| {
                ingest::require_record(&library.conn, image_id)
            })
            .unwrap()
            .posts
        }

        /// The MD5 lookup that precedes every upload, answered "no such
        /// post" (design D12).
        async fn no_post_holds_the_file(server: &MockServer) {
            Mock::given(method("GET"))
                .and(path("/posts.json"))
                .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
                .mount(server)
                .await;
        }

        /// The lookup, `/uploads.json` and `/uploads/9.json` mocked to a
        /// completed processing state; `POST /posts.json` left to each test
        /// so a failure there, or past it, can still be driven.
        async fn upload_and_processing_ok(server: &MockServer) {
            no_post_holds_the_file(server).await;
            Mock::given(method("POST"))
                .and(path("/uploads.json"))
                .respond_with(ResponseTemplate::new(201).set_body_json(json!({ "id": 9 })))
                .mount(server)
                .await;
            Mock::given(method("GET"))
                .and(path("/uploads/9.json"))
                .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                    "status": "completed",
                    "upload_media_assets": [{ "id": 77, "media_asset_id": 4001 }],
                })))
                .mount(server)
                .await;
        }

        #[test]
        fn saving_a_site_then_listing_round_trips_it() {
            let (_dir, app) = app_with_library();

            let created = now(booru_site_save(
                None,
                "Danbooru".to_string(),
                "https://danbooru.donmai.us".to_string(),
                "alice".to_string(),
                Some("secret-key".to_string()),
                app.state(),
            ))
            .unwrap();

            assert_eq!(created.id, "danbooru-donmai-us");
            assert_eq!(now(booru_site_list(app.state())).unwrap(), vec![created]);
        }

        #[test]
        fn deleting_a_site_removes_it_and_its_credential() {
            let (_dir, app) = app_with_library();
            let site = now(booru_site_save(
                None,
                "Danbooru".to_string(),
                "https://danbooru.donmai.us".to_string(),
                "alice".to_string(),
                Some("secret-key".to_string()),
                app.state(),
            ))
            .unwrap();

            now(booru_site_delete(site.id.clone(), app.state())).unwrap();

            assert!(now(booru_site_list(app.state())).unwrap().is_empty());
            let credentials = app.state::<AppState>().credentials.clone();
            assert_eq!(
                credentials.get("danbooru.donmai.us", "alice").unwrap(),
                None
            );
        }

        #[test]
        fn test_connection_reports_the_three_outcomes() {
            let (_dir, app) = app_with_library();

            now(async {
                let server = MockServer::start().await;
                Mock::given(method("GET"))
                    .and(path("/profile.json"))
                    .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "id": 1 })))
                    .mount(&server)
                    .await;
                let working = booru_site_save(
                    None,
                    "Working".to_string(),
                    server.uri(),
                    "alice".to_string(),
                    Some("secret-key".to_string()),
                    app.state(),
                )
                .await
                .unwrap();
                assert_eq!(
                    booru_site_test(working.id, app.state()).await.unwrap(),
                    BooruConnectionTest::Connected
                );

                // A second server, so the two sites do not share a base
                // address (`booru-sites`: two sites cannot share one).
                let server2 = MockServer::start().await;
                Mock::given(method("GET"))
                    .and(path("/profile.json"))
                    .respond_with(ResponseTemplate::new(401))
                    .mount(&server2)
                    .await;
                let wrong_key = booru_site_save(
                    None,
                    "Wrong key".to_string(),
                    server2.uri(),
                    "alice".to_string(),
                    Some("wrong-key".to_string()),
                    app.state(),
                )
                .await
                .unwrap();
                assert_eq!(
                    booru_site_test(wrong_key.id, app.state()).await.unwrap(),
                    BooruConnectionTest::CredentialRejected
                );
            });
        }

        #[test]
        fn test_connection_reports_unreachable_for_an_address_nothing_answers_on() {
            let (_dir, app) = app_with_library();

            now(async {
                // A server that was bound and then dropped: its address no
                // longer has anything listening on it.
                let server = MockServer::start().await;
                let uri = server.uri();
                drop(server);

                let site = booru_site_save(
                    None,
                    "Nowhere".to_string(),
                    uri,
                    "alice".to_string(),
                    Some("secret-key".to_string()),
                    app.state(),
                )
                .await
                .unwrap();

                let outcome = booru_site_test(site.id, app.state()).await.unwrap();
                assert!(matches!(outcome, BooruConnectionTest::Unreachable { .. }));
            });
        }

        #[test]
        fn a_successful_upload_records_a_post_and_the_image_shows_it() {
            let (_dir, app) = app_with_library();
            import(&app, &folder_of_images(1));
            let id = ids_in_library(&app)[0].clone();

            now(async {
                let server = MockServer::start().await;
                upload_and_processing_ok(&server).await;
                Mock::given(method("POST"))
                    .and(path("/posts.json"))
                    .respond_with(ResponseTemplate::new(201).set_body_json(json!({ "id": 555 })))
                    .mount(&server)
                    .await;
                let site = booru_site_save(
                    None,
                    "Test booru".to_string(),
                    server.uri(),
                    "alice".to_string(),
                    Some("secret-key".to_string()),
                    app.state(),
                )
                .await
                .unwrap();

                let outcome = booru_upload(
                    id.clone(),
                    site.id.clone(),
                    upload_form(&["1girl"], ""),
                    app.state(),
                )
                .await
                .unwrap();

                match outcome {
                    BooruUploadOutcome::Posted { post, commentary } => {
                        assert_eq!(post.site, site.id);
                        assert_eq!(post.remote_id, "555");
                        assert_eq!(commentary, CommentaryOutcome::Skipped);
                    }
                    BooruUploadOutcome::Failed { error } => {
                        panic!("expected success, got {error:?}")
                    }
                }
            });

            let posts = posts_of(&app, &id);
            assert_eq!(posts.len(), 1);
            assert_eq!(posts[0].remote_id, "555");
        }

        #[test]
        fn a_missing_credential_fails_before_any_request_is_made() {
            let (_dir, app) = app_with_library();
            import(&app, &folder_of_images(1));
            let id = ids_in_library(&app)[0].clone();

            let error = now(async {
                // Nothing is listening here; a request reaching the network
                // would hang or refuse the connection rather than answer, so a
                // fast `AppError::Credential` is proof this was never sent.
                let site = booru_site_save(
                    None,
                    "No key yet".to_string(),
                    "http://127.0.0.1:1".to_string(),
                    "alice".to_string(),
                    None,
                    app.state(),
                )
                .await
                .unwrap();

                booru_upload(
                    id.clone(),
                    site.id,
                    upload_form(&["1girl"], ""),
                    app.state(),
                )
                .await
            })
            .unwrap_err();

            assert!(matches!(error, AppError::Credential { .. }), "{error:?}");
            assert!(posts_of(&app, &id).is_empty());
        }

        #[test]
        fn a_refusing_credential_store_fails_the_upload_before_any_request_is_made() {
            let credentials =
                std::sync::Arc::new(InMemoryCredentials::refusing("keychain is locked"));
            let (_dir, app) = app_with_library_and_credentials(credentials);
            import(&app, &folder_of_images(1));
            let id = ids_in_library(&app)[0].clone();

            let error = now(async {
                let site = booru_site_save(
                    None,
                    "Locked".to_string(),
                    "http://127.0.0.1:1".to_string(),
                    "alice".to_string(),
                    Some("secret-key".to_string()),
                    app.state(),
                )
                .await;
                // Saving the site's own settings must not depend on the
                // credential store (`booru-sites` design D7): only the key
                // write fails.
                assert!(matches!(site.unwrap_err(), AppError::Credential { .. }));

                let saved = booru_site_list(app.state()).await.unwrap().remove(0);
                booru_upload(
                    id.clone(),
                    saved.id,
                    upload_form(&["1girl"], ""),
                    app.state(),
                )
                .await
            })
            .unwrap_err();

            assert!(matches!(error, AppError::Credential { .. }), "{error:?}");
            assert!(posts_of(&app, &id).is_empty());
        }

        #[test]
        fn a_refused_upload_records_nothing() {
            let (_dir, app) = app_with_library();
            import(&app, &folder_of_images(1));
            let id = ids_in_library(&app)[0].clone();

            let outcome = now(async {
                let server = MockServer::start().await;
                no_post_holds_the_file(&server).await;
                Mock::given(method("POST"))
                    .and(path("/uploads.json"))
                    .respond_with(
                        ResponseTemplate::new(422)
                            .set_body_json(json!({ "message": "duplicate of post #1" })),
                    )
                    .mount(&server)
                    .await;
                let site = booru_site_save(
                    None,
                    "Test booru".to_string(),
                    server.uri(),
                    "alice".to_string(),
                    Some("secret-key".to_string()),
                    app.state(),
                )
                .await
                .unwrap();

                booru_upload(
                    id.clone(),
                    site.id,
                    upload_form(&["1girl"], ""),
                    app.state(),
                )
                .await
                .unwrap()
            });

            match outcome {
                BooruUploadOutcome::Failed { error } => {
                    assert_eq!(error.step, UploadStep::CreateUpload)
                }
                BooruUploadOutcome::Posted { .. } => panic!("expected a failure"),
            }
            assert!(posts_of(&app, &id).is_empty());
        }

        #[test]
        fn a_processing_timeout_records_nothing() {
            let (_dir, app) = app_with_library();
            import(&app, &folder_of_images(1));
            let id = ids_in_library(&app)[0].clone();

            let outcome = now(async {
                let server = MockServer::start().await;
                no_post_holds_the_file(&server).await;
                Mock::given(method("POST"))
                    .and(path("/uploads.json"))
                    .respond_with(ResponseTemplate::new(201).set_body_json(json!({ "id": 9 })))
                    .mount(&server)
                    .await;
                Mock::given(method("GET"))
                    .and(path("/uploads/9.json"))
                    .respond_with(
                        ResponseTemplate::new(200).set_body_json(json!({ "status": "processing" })),
                    )
                    .mount(&server)
                    .await;
                let site = booru_site_save(
                    None,
                    "Test booru".to_string(),
                    server.uri(),
                    "alice".to_string(),
                    Some("secret-key".to_string()),
                    app.state(),
                )
                .await
                .unwrap();

                booru_upload(
                    id.clone(),
                    site.id,
                    upload_form(&["1girl"], ""),
                    app.state(),
                )
                .await
                .unwrap()
            });

            match outcome {
                BooruUploadOutcome::Failed { error } => {
                    assert_eq!(error.step, UploadStep::AwaitProcessing);
                    assert_eq!(error.remote_ref, Some("9".to_string()));
                }
                BooruUploadOutcome::Posted { .. } => panic!("expected a failure"),
            }
            assert!(posts_of(&app, &id).is_empty());
        }

        #[test]
        fn a_refused_post_creation_records_nothing() {
            let (_dir, app) = app_with_library();
            import(&app, &folder_of_images(1));
            let id = ids_in_library(&app)[0].clone();

            let outcome = now(async {
                let server = MockServer::start().await;
                upload_and_processing_ok(&server).await;
                Mock::given(method("POST"))
                    .and(path("/posts.json"))
                    .respond_with(
                        ResponseTemplate::new(422)
                            .set_body_json(json!({ "message": "invalid rating" })),
                    )
                    .mount(&server)
                    .await;
                let site = booru_site_save(
                    None,
                    "Test booru".to_string(),
                    server.uri(),
                    "alice".to_string(),
                    Some("secret-key".to_string()),
                    app.state(),
                )
                .await
                .unwrap();

                booru_upload(
                    id.clone(),
                    site.id,
                    upload_form(&["1girl"], ""),
                    app.state(),
                )
                .await
                .unwrap()
            });

            match outcome {
                BooruUploadOutcome::Failed { error } => {
                    assert_eq!(error.step, UploadStep::CreatePost)
                }
                BooruUploadOutcome::Posted { .. } => panic!("expected a failure"),
            }
            assert!(posts_of(&app, &id).is_empty());
        }

        #[test]
        fn a_commentary_failure_still_leaves_the_post_row() {
            let (_dir, app) = app_with_library();
            import(&app, &folder_of_images(1));
            let id = ids_in_library(&app)[0].clone();

            let outcome = now(async {
                let server = MockServer::start().await;
                upload_and_processing_ok(&server).await;
                Mock::given(method("POST"))
                    .and(path("/posts.json"))
                    .respond_with(ResponseTemplate::new(201).set_body_json(json!({ "id": 555 })))
                    .mount(&server)
                    .await;
                Mock::given(method("PUT"))
                    .and(path("/posts/555/artist_commentary/create_or_update.json"))
                    .respond_with(
                        ResponseTemplate::new(422)
                            .set_body_json(json!({ "message": "title too long" })),
                    )
                    .mount(&server)
                    .await;
                let site = booru_site_save(
                    None,
                    "Test booru".to_string(),
                    server.uri(),
                    "alice".to_string(),
                    Some("secret-key".to_string()),
                    app.state(),
                )
                .await
                .unwrap();

                booru_upload(
                    id.clone(),
                    site.id,
                    upload_form(&["1girl"], "a title"),
                    app.state(),
                )
                .await
                .unwrap()
            });

            match outcome {
                BooruUploadOutcome::Posted { post, commentary } => {
                    assert_eq!(post.remote_id, "555");
                    assert_eq!(
                        commentary,
                        CommentaryOutcome::Failed {
                            message: "title too long".to_string()
                        }
                    );
                }
                BooruUploadOutcome::Failed { error } => panic!("expected success, got {error:?}"),
            }
            assert_eq!(posts_of(&app, &id).len(), 1);
        }

        #[test]
        fn an_upload_with_no_tags_or_no_rating_is_refused_without_touching_the_network() {
            let (_dir, app) = app_with_library();
            import(&app, &folder_of_images(1));
            let id = ids_in_library(&app)[0].clone();
            let site = now(booru_site_save(
                None,
                "Test booru".to_string(),
                "http://127.0.0.1:1".to_string(),
                "alice".to_string(),
                Some("secret-key".to_string()),
                app.state(),
            ))
            .unwrap();

            let no_tags = now(booru_upload(
                id.clone(),
                site.id.clone(),
                BooruUploadForm {
                    rating: "s".to_string(),
                    ..upload_form(&[], "")
                },
                app.state(),
            ))
            .unwrap_err();
            assert!(matches!(no_tags, AppError::BadRequest(_)));

            let no_rating = now(booru_upload(
                id,
                site.id,
                BooruUploadForm {
                    rating: String::new(),
                    ..upload_form(&["1girl"], "")
                },
                app.state(),
            ))
            .unwrap_err();
            assert!(matches!(no_rating, AppError::BadRequest(_)));
        }
    }
}
