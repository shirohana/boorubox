//! The `#[tauri::command]` surface (design D12) — argument marshalling only.
//! Every parameter name here is part of the contract: `tauri::command` derives
//! the JSON key the webview sends from it, so renaming one breaks `lib/api/`.
//!
//! The commands are generic over the runtime so the tests below can drive them
//! on `tauri::test`'s mock runtime; `generate_handler!` fills `R` in with the
//! real one.

use std::path::{Path, PathBuf};

use tauri::{AppHandle, Emitter, Manager, Runtime, State};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;

use crate::error::{AppError, Result};
use crate::library::{self, Library};
use crate::model::{
    AppSettings, GRID_TILE_MAX, GRID_TILE_MIN, ImageCounts, ImportReport, LibraryStatus,
    ListenerStatus, RecentLibrary, SearchRequest, SearchResult, Theme,
};
use crate::settings::Settings;
use crate::{
    AppState, VERSION, from_tauri, http, import, ingest, lock, maintenance, query, settings, thumbs,
};

/// Progress while `import_paths` runs. The webview subscribes under this name;
/// change it here and the import screen sits at zero until the run finishes.
const IMPORT_PROGRESS_EVENT: &str = "import:progress";

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
        Some(path) => open_and_remember(&app, &state, &path),
        None => status(&state),
    }
}

#[tauri::command]
pub fn open_library<R: Runtime>(
    path: String,
    app: AppHandle<R>,
    state: State<'_, AppState>,
) -> Result<LibraryStatus> {
    open_and_remember(&app, &state, Path::new(&path))
}

#[tauri::command]
pub fn library_status(state: State<'_, AppState>) -> Result<LibraryStatus> {
    status(&state)
}

/// Close the open library and forget the remembered path, so the next launch
/// starts on the start screen rather than reporting a folder as missing — which
/// is what `status` would derive from a path with no library open (design D3).
/// The folder is not forgotten: it is the first entry of the recent list.
#[tauri::command]
pub fn close_library<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
) -> Result<LibraryStatus> {
    // Dropping the `Library` closes its connection. Nothing may hold a second
    // one on that database (§7).
    *lock(&state.library) = None;
    write_settings(&app, &state, |settings| settings.library_path = None)?;
    status(&state)
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
    let status = rebind_listener(&state, port).await;
    write_settings(&app, &state, |settings| settings.port = port)?;
    Ok(status)
}

/// Stop whatever is listening and bind `port`, leaving the outcome in the state.
/// A port that will not bind is a state, not an error: the app opens, and keeps
/// working, either way. `setup` binds the first listener through this too.
pub async fn rebind_listener(state: &AppState, port: u16) -> ListenerStatus {
    // Taken out of the state before the `await`, not inside an `if let`: the
    // guard would otherwise be held across it, and a `MutexGuard` is not `Send`.
    let running = lock(&state.listener_shutdown).take();
    if let Some(handle) = running {
        handle.stop().await;
    }

    let (status, handle) = http::start(state.http_state(), port).await;
    *lock(&state.listener_shutdown) = handle;
    *lock(&state.listener) = status.clone();
    status
}

/// Show the open library's folder in the file manager, so it can be backed up
/// or copied (docs/requirements.md §6).
#[tauri::command]
pub fn reveal_library<R: Runtime>(app: AppHandle<R>, state: State<'_, AppState>) -> Result<()> {
    let root = http::with_library(&state.library, |library| Ok(library.root.clone()))?;
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
pub fn search(req: SearchRequest, state: State<'_, AppState>) -> Result<SearchResult> {
    http::with_library(&state.library, |library| {
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
        })
    })
}

#[tauri::command]
pub fn image_counts(state: State<'_, AppState>) -> Result<ImageCounts> {
    http::with_library(&state.library, maintenance::image_counts)
}

#[tauri::command]
pub fn drop_image_record(id: String, state: State<'_, AppState>) -> Result<LibraryStatus> {
    http::with_library(&state.library, |library| {
        maintenance::drop_image_record(library, &id)
    })?;
    status(&state)
}

/// Absolute path; the thumbnail is generated if it is not there yet.
#[tauri::command]
pub fn thumbnail_path(id: String, state: State<'_, AppState>) -> Result<String> {
    http::with_library(&state.library, |library| {
        let record = ingest::load_record(&library.conn, &id)?
            .ok_or_else(|| AppError::NotFound(format!("image {id}")))?;
        Ok(thumbs::ensure_thumbnail(library, &record)?
            .display()
            .to_string())
    })
}

/// Emits `import:progress` while it runs.
#[tauri::command]
pub async fn import_paths<R: Runtime>(
    paths: Vec<String>,
    app: AppHandle<R>,
    state: State<'_, AppState>,
) -> Result<ImportReport> {
    let paths: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
    // The library is reached through its `Arc`, not through `State`: the closure
    // below outlives this call's borrow of the managed state.
    let library = state.library.clone();

    // `import::import_paths` decodes and copies every file it is given, one at a
    // time behind the library mutex. Off the async runtime's worker threads, or
    // a dropped folder freezes the window it is reporting progress to.
    tauri::async_runtime::spawn_blocking(move || {
        http::with_library(&library, |library| {
            import::import_paths(library, &paths, &mut |progress| {
                // A dropped tick is a progress bar that skips a number; the
                // import itself is unaffected, so it is not worth failing over.
                let _ = app.emit(IMPORT_PROGRESS_EVENT, progress);
            })
        })
    })
    .await
    .map_err(from_tauri)?
}

/// Open `path`, remember it, and let the webview read images out of it.
///
/// The path is stored only once the folder has actually opened: `status` reads a
/// remembered path with no open library as `missing_path`, so storing it earlier
/// would name a perfectly good folder as missing.
fn open_and_remember<R: Runtime>(
    app: &AppHandle<R>,
    state: &AppState,
    path: &Path,
) -> Result<LibraryStatus> {
    open_into_state(app, state, path, OpenMode::CreateIfMissing)?;
    write_settings(app, state, |settings| {
        settings.library_path = Some(path.to_path_buf());
        settings.remember_recent(path);
    })?;
    status(state)
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
pub fn open_into_state<R: Runtime>(
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
    *lock(&state.library) = Some(library);
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
    for directory in [library.images_dir(), library.thumbs_dir()] {
        scope
            .allow_directory(&directory, true)
            .map_err(from_tauri)?;
    }
    Ok(())
}

fn status(state: &AppState) -> Result<LibraryStatus> {
    let library = lock(&state.library);
    let open = library.as_ref();
    let remembered = lock(&state.settings).library_path.clone();

    Ok(LibraryStatus {
        opened: open.is_some(),
        library_path: open.map(|library| library.root.display().to_string()),
        // Derived, not stored: a remembered path with no library open is exactly
        // the path that would not open, which the spec keeps until the user
        // picks another. A second field would have to be cleared by hand
        // wherever a library opens, and one day would not be.
        missing_path: match open {
            Some(_) => None,
            None => remembered.map(|path| path.display().to_string()),
        },
        image_count: open.map_or(Ok(0), Library::image_count)?,
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
    use tauri::Listener;
    use tauri::test::MockRuntime;

    use super::*;
    use crate::http::test_support::{ask_for_status, free_port, nothing_answers_on, png_bytes};
    use crate::model::{GRID_TILE_DEFAULT, ImportProgress, ImportStatus, ParsedTagSearch};
    use crate::test_support::mock_app;

    type App = tauri::App<MockRuntime>;

    fn everything() -> SearchRequest {
        SearchRequest {
            query: ParsedTagSearch::default(),
            text: String::new(),
            include_deleted: false,
            limit: 100,
            offset: 0,
        }
    }

    /// An app with a library open in a temp folder. The `TempDir` comes back
    /// with it: dropping it deletes the library.
    fn app_with_library() -> (tempfile::TempDir, App) {
        let dir = tempfile::tempdir().unwrap();
        let app = mock_app();
        open_library(
            dir.path().display().to_string(),
            app.handle().clone(),
            app.state(),
        )
        .unwrap();
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
        tauri::async_runtime::block_on(import_paths(
            vec![dir.path().display().to_string()],
            app.handle().clone(),
            app.state(),
        ))
        .unwrap()
    }

    fn ids_in_library(app: &App) -> Vec<String> {
        search(everything(), app.state())
            .unwrap()
            .images
            .into_iter()
            .map(|image| image.id)
            .collect()
    }

    #[test]
    fn open_library_creates_the_folder_and_reports_it_open() {
        let dir = tempfile::tempdir().unwrap();
        let app = mock_app();

        let status = open_library(
            dir.path().display().to_string(),
            app.handle().clone(),
            app.state(),
        )
        .unwrap();

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

        let status = library_status(app.state()).unwrap();

        assert!(!status.opened);
        assert_eq!(status.library_path, None);
        assert_eq!(status.missing_path, None);
    }

    #[test]
    fn a_remembered_path_that_did_not_open_is_reported_as_missing() {
        let app = mock_app();
        let gone = PathBuf::from("/nonexistent/boorubox/library");
        lock(&app.state::<AppState>().settings).library_path = Some(gone.clone());

        let status = library_status(app.state()).unwrap();

        assert!(!status.opened);
        assert_eq!(status.missing_path, Some(gone.display().to_string()));
    }

    #[test]
    fn opening_a_library_clears_a_missing_path() {
        let dir = tempfile::tempdir().unwrap();
        let app = mock_app();
        lock(&app.state::<AppState>().settings).library_path =
            Some(PathBuf::from("/nonexistent/boorubox/library"));

        let status = open_library(
            dir.path().display().to_string(),
            app.handle().clone(),
            app.state(),
        )
        .unwrap();

        assert_eq!(status.missing_path, None);
        assert_eq!(status.library_path, Some(dir.path().display().to_string()));
    }

    #[test]
    fn a_fresh_profile_reads_the_default_preferences() {
        let app = mock_app();

        let settings = app_settings(app.state());

        assert_eq!(settings.theme, Theme::System);
        assert_eq!(settings.grid_tile_size, GRID_TILE_DEFAULT);
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

    /// The `library-switching` spec's "Two libraries seen": the list is what
    /// the start screen offers, so its order is the requirement.
    #[test]
    fn switching_from_one_library_to_another_lists_the_newest_first() {
        let first = tempfile::tempdir().unwrap();
        let second = tempfile::tempdir().unwrap();
        let app = mock_app();

        for dir in [&first, &second] {
            open_library(
                dir.path().display().to_string(),
                app.handle().clone(),
                app.state(),
            )
            .unwrap();
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

        let status = close_library(app.handle().clone(), app.state()).unwrap();

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
        open_library(
            gone.display().to_string(),
            app.handle().clone(),
            app.state(),
        )
        .unwrap();
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

        let error = open_library(
            blocked.join("library").display().to_string(),
            app.handle().clone(),
            app.state(),
        )
        .unwrap_err();

        let status = library_status(app.state()).unwrap();
        assert!(status.opened, "the working library must survive: {error:?}");
        assert_eq!(status.library_path, Some(dir.path().display().to_string()));
        assert_eq!(recent_libraries(app.state()).len(), 1);
    }

    /// The reveal itself opens a file manager window, so only the refusal is
    /// testable here; the rest is the manual pass (task 5.4).
    #[test]
    fn revealing_the_library_folder_needs_one_to_be_open() {
        let app = mock_app();

        let error = reveal_library(app.handle().clone(), app.state()).unwrap_err();

        assert!(matches!(error, AppError::NoLibrary), "{error:?}");
    }

    fn set_port(app: &App, port: u16) -> ListenerStatus {
        tauri::async_runtime::block_on(set_listener_port(port, app.handle().clone(), app.state()))
            .unwrap()
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
        assert_eq!(library_status(app.state()).unwrap().listener.port, now);
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
            library_status(app.state()).unwrap().opened,
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

    #[test]
    fn search_returns_the_page_and_its_total() {
        let (_library, app) = app_with_library();
        import(&app, &folder_of_images(3));

        let result = search(
            SearchRequest {
                limit: 2,
                ..everything()
            },
            app.state(),
        )
        .unwrap();

        assert_eq!(result.total, 3);
        assert_eq!(result.images.len(), 2);
    }

    #[test]
    fn search_reports_a_file_that_vanished_since_it_was_imported() {
        let (library, app) = app_with_library();
        import(&app, &folder_of_images(1));
        let id = ids_in_library(&app).remove(0);
        std::fs::remove_file(library.path().join("images").join(format!("{id}.png"))).unwrap();

        let result = search(everything(), app.state()).unwrap();

        assert!(
            result.images[0].missing,
            "the flag must be re-checked on the page being returned"
        );
    }

    #[test]
    fn image_counts_groups_the_import_under_local() {
        let (_library, app) = app_with_library();
        import(&app, &folder_of_images(2));

        let counts = image_counts(app.state()).unwrap();

        assert_eq!(counts.total, 2);
        assert_eq!(counts.local, 2);
        assert_eq!(counts.extension, 0);
    }

    #[test]
    fn dropping_a_record_returns_the_status_with_the_new_count() {
        let (_library, app) = app_with_library();
        import(&app, &folder_of_images(2));
        let id = ids_in_library(&app).remove(0);

        let status = drop_image_record(id, app.state()).unwrap();

        assert_eq!(status.image_count, 1);
        assert!(status.opened);
    }

    #[test]
    fn thumbnail_path_generates_the_file_on_demand() {
        let (_library, app) = app_with_library();
        import(&app, &folder_of_images(1));
        let id = ids_in_library(&app).remove(0);
        let path = PathBuf::from(thumbnail_path(id.clone(), app.state()).unwrap());
        std::fs::remove_file(&path).unwrap();

        let again = thumbnail_path(id, app.state()).unwrap();

        assert_eq!(PathBuf::from(&again), path);
        assert!(path.is_file());
    }

    #[test]
    fn a_thumbnail_for_an_unknown_image_is_not_found() {
        let (_library, app) = app_with_library();

        let error = thumbnail_path("no-such-id".to_string(), app.state()).unwrap_err();

        assert!(matches!(error, AppError::NotFound(_)), "{error:?}");
    }

    #[test]
    fn commands_needing_a_library_say_so_before_one_is_open() {
        let app = mock_app();

        let error = search(everything(), app.state()).unwrap_err();

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

        let status = library_status(app.state()).unwrap();
        assert!(!status.opened);
        assert_eq!(status.missing_path, Some(gone.display().to_string()));
    }
}
