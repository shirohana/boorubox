pub mod booru;
pub mod bundle;
pub mod collections;
pub mod commands;
pub mod db;
pub mod error;
pub mod export;
pub mod facts;
pub mod http;
pub mod import;
pub mod ingest;
pub mod library;
pub mod maintenance;
pub mod model;
pub mod notes;
pub mod query;
pub mod recover;
pub mod rules;
pub mod settings;
pub mod sidecar;
pub mod tags;
pub mod thumbs;
pub mod trash;

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use tauri::{AppHandle, Emitter, Manager, Runtime};

use crate::error::AppError;
use crate::library::SharedLibrary;
use crate::model::ListenerStatus;
use crate::settings::Settings;

/// The build's version. `GET /status` and `library_status` both report it, so
/// the extension and the webview can never disagree about which app answered.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Tauri managed state. The mutex around the library is what makes design D1's
/// "one connection, one writer" true rather than merely intended.
pub struct AppState {
    pub library: SharedLibrary,
    pub settings: Mutex<Settings>,
    pub listener: Mutex<ListenerStatus>,
    /// The running listener's off switch, so the port can be changed without a
    /// relaunch (design D6). `None` whenever nothing is listening.
    pub listener_shutdown: Mutex<Option<http::ListenerHandle>>,
    /// The running import's pause/resume/cancel handle (`import-pause-cancel`
    /// design D4), one slot for the one run Rust knows about at a time.
    /// `import_paths`/`import_bundle` install a fresh handle before the run
    /// and clear the slot when it returns, early or not, so a control signal
    /// pressed after a run has finished cancels nothing rather than the next
    /// run, and `import_pause`/`import_resume`/`import_cancel` are a no-op
    /// with nothing running.
    pub import_control: Mutex<Option<Arc<import::ImportControl>>>,
    /// The last outcome of `open_into_state`, against whichever path it tried
    /// (`library-sidecars` design D9): `None` on success, the path and why
    /// otherwise. `LibraryStatus.damaged_path` reads this rather than
    /// re-deriving it — repeating the open just to answer a status poll would
    /// mean a `quick_check` (`db.rs`) on every poll. Written by exactly the
    /// one function that can change the answer, on both outcomes, so nothing
    /// else has to remember to clear it.
    pub open_failure: Mutex<Option<(std::path::PathBuf, OpenFailureKind)>>,
    /// The path `open_remembered_library_and_listen` is still opening on its
    /// blocking thread (`launch-screen` design D1), `None` once that open has
    /// settled either way. `LibraryStatus.opening` mirrors this so the
    /// webview can name the folder before `state.library` is set.
    pub launch_opening: Mutex<Option<std::path::PathBuf>>,
    /// The OS credential store behind `booru::credentials::Credentials`
    /// (`booru-upload` design D7). Built once here rather than per call:
    /// `KeyringCredentials` itself has no state, but a trait object is what
    /// lets `test_support::mock_app` swap in an in-memory store, so no test
    /// ever reaches a real keychain.
    pub credentials: Arc<dyn booru::credentials::Credentials>,
}

impl Default for AppState {
    fn default() -> Self {
        AppState {
            // The stored settings replace these in `setup`; until then the
            // defaults are what a first launch would have written anyway.
            library: SharedLibrary::default(),
            settings: Mutex::new(Settings::default()),
            listener: Mutex::new(ListenerStatus::default()),
            listener_shutdown: Mutex::new(None),
            import_control: Mutex::new(None),
            open_failure: Mutex::new(None),
            launch_opening: Mutex::new(None),
            credentials: Arc::new(booru::credentials::KeyringCredentials),
        }
    }
}

/// Which kind of failure `AppState.open_failure` names (`library-sidecars`
/// design D9): `Corrupt` surfaces as `LibraryStatus.damaged_path`;
/// `SchemaTooNew` (a folder written by a newer build, `library-recovery`
/// spec's "A library from a newer build") surfaces as `newer_path`, a slot of
/// its own — it is not missing, and treating it as such would be wrong and
/// frightening for a folder sitting right there, but reporting nothing at all
/// loses the folder entirely; `Other` — a folder that really is gone,
/// unreadable, or anything else — keeps today's `missing_path` reading.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenFailureKind {
    Corrupt,
    SchemaTooNew,
    Other,
}

impl AppState {
    /// What the listener's handlers see. Built from this state, so a capture and
    /// a UI action never reach two different libraries — including after a
    /// rebind, which builds a second router over the same `SharedLibrary`.
    ///
    /// The app handle is here for one reason: a capture arrives with nothing on
    /// screen having asked for it, and only an event tells the webview to read
    /// the library again.
    pub fn http_state<R: Runtime>(&self, app: &AppHandle<R>) -> http::HttpState {
        let app = app.clone();
        http::HttpState {
            library: self.library.clone(),
            version: VERSION.to_string(),
            on_event: Arc::new(move |event| {
                // A dropped event is a grid that waits for the user's next
                // action; the image is stored either way, so it is not worth
                // failing the delivery the extension is still waiting on.
                let _ = match event {
                    http::CaptureEvent::Pending(meta) => {
                        app.emit(commands::CAPTURE_PENDING_EVENT, meta)
                    }
                    http::CaptureEvent::Withdrawn(withdrawn) => {
                        app.emit(commands::CAPTURE_WITHDRAWN_EVENT, withdrawn)
                    }
                    http::CaptureEvent::Stored(record) => {
                        app.emit(commands::CAPTURE_STORED_EVENT, record)
                    }
                };
            }),
        }
    }
}

/// Take one of `AppState`'s locks, recovering from poisoning instead of
/// propagating it. A panic under one of these leaves the value usable — they
/// hold plain data, or a connection whose open transaction rolled back — and
/// refusing every later command would be the larger outage.
pub fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// A failure inside Tauri itself — the plugin store, the asset-protocol scope —
/// as an `AppError`.
///
/// FIXME: `error.rs` has no variant for these, so they arrive as `Io`, whose
/// `Display` is its inner error verbatim and so loses only the variant. The
/// right shape is a `Tauri` variant there.
pub fn from_tauri(error: impl std::fmt::Display) -> AppError {
    AppError::Io(std::io::Error::other(error.to_string()))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        // `app-update` design D1: the plugin does check/download/verify/install
        // and the restart; we add no Rust commands of our own, the confirmation
        // lives in the webview which calls the guest bindings directly.
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .manage(AppState::default())
        .setup(|app| {
            open_remembered_library_and_listen(app);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::pick_library,
            commands::open_library,
            commands::library_status,
            commands::close_library,
            commands::rebuild_library,
            commands::recent_libraries,
            commands::forget_recent,
            commands::reveal_library,
            commands::set_listener_port,
            commands::app_settings,
            commands::set_theme,
            commands::set_grid_tile_size,
            commands::set_click_zoom_ceiling_percent,
            commands::set_open_last_on_launch,
            commands::search,
            commands::search_ids,
            commands::matching_ids,
            commands::search_position,
            commands::tag_counts,
            commands::update_tags,
            commands::set_rating,
            commands::update_facts,
            commands::tag_suggestions,
            commands::image_counts,
            commands::thumbnail_path,
            commands::import_paths,
            commands::import_bundle,
            commands::bundle_plan,
            commands::import_pause,
            commands::import_resume,
            commands::import_cancel,
            commands::bulk_update_tags,
            commands::bulk_set_rating,
            commands::selection_tag_counts,
            commands::export_zip,
            commands::trash_images,
            commands::restore_images,
            commands::delete_forever,
            commands::empty_trash,
            commands::trash_count,
            commands::rules_list,
            commands::rules_upsert,
            commands::rules_delete,
            commands::rules_run,
            commands::rules_export,
            commands::rules_import,
            commands::collection_list,
            commands::collection_create,
            commands::collection_rename,
            commands::collection_delete,
            commands::collection_add,
            commands::collection_remove,
            commands::note_get,
            commands::note_set,
            commands::set_notes_collapsed,
            commands::set_collections_collapsed,
            commands::booru_site_list,
            commands::booru_site_save,
            commands::booru_site_delete,
            commands::booru_site_test,
            commands::booru_upload,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Everything that must be true before the window appears: the capture
/// listener bound, and the last library's open under way if it still runs.
///
/// Nothing here can fail the launch, and nothing here waits for the library to
/// open: `setup` returns as soon as the listener is bound, so the window
/// paints before a large library finishes opening (`launch-screen` design
/// D1) rather than after. A library that will not open leaves the app on the
/// start screen with the path named (spec `library-folder`), and a port
/// already taken leaves the listener stopped with its reason on the settings
/// screen (spec `capture-ingest`, design D6).
fn open_remembered_library_and_listen(app: &tauri::App) {
    let handle = app.handle().clone();
    let state = app.state::<AppState>();
    let settings = settings::load(&handle);

    // Off: the path stays remembered (spec `library-folder`'s "The remembered
    // path SHALL be kept in both cases"), nothing opens, and no flag is set —
    // `status` reads as the plain not-opened case, which is what sends the
    // start screen to its recent list (design D3).
    if let Some(path) = settings.library_path.clone()
        && settings.open_last_on_launch
    {
        spawn_launch_open(handle.clone(), path);
    }

    tauri::async_runtime::block_on(commands::rebind_listener(&handle, &state, settings.port));
    *lock(&state.settings) = settings;
}

/// Open `path` on a blocking thread and clear `AppState.launch_opening` when
/// it settles, either way, emitting `LIBRARY_OPENED_EVENT` so the webview
/// re-reads `library_status` rather than being handed the outcome directly —
/// the same path a capture or a failure during the open already reports
/// through (`launch-screen` design D1).
fn spawn_launch_open(handle: tauri::AppHandle, path: std::path::PathBuf) {
    let state = handle.state::<AppState>();
    *lock(&state.launch_opening) = Some(path.clone());

    tauri::async_runtime::spawn_blocking(move || {
        let state = handle.state::<AppState>();
        // The refusal itself is not dropped with the `Result`: `open_into_state`
        // records it in `AppState.open_failure`, so `library_status` names this
        // folder as `damaged_path`, `newer_path` or `missing_path` depending on
        // why it would not open (design D9). `ExistingOnly` is what makes that
        // reachable — creating the folder here would report a healthy empty
        // library instead of a missing one.
        let _ = commands::open_into_state(&handle, &state, &path, commands::OpenMode::ExistingOnly);
        *lock(&state.launch_opening) = None;
        let _ = handle.emit(commands::LIBRARY_OPENED_EVENT, ());
    });
}

/// A headless app the command and settings tests can hand to the code under
/// test wherever it wants an `AppHandle`.
#[cfg(test)]
pub mod test_support {
    use std::sync::Arc;

    use tauri::test::{MockRuntime, mock_builder, mock_context, noop_assets};

    use crate::AppState;
    use crate::booru::credentials::{Credentials, InMemoryCredentials};

    /// An in-memory credential store, never the real keychain (`booru-upload`
    /// design D7's "no test touches a real keychain"). Use
    /// [`mock_app_with_credentials`] for a test that needs a specific store —
    /// a refusing one, or one pre-loaded with a key.
    pub fn mock_app() -> tauri::App<MockRuntime> {
        mock_app_with_credentials(Arc::new(InMemoryCredentials::default()))
    }

    pub fn mock_app_with_credentials(credentials: Arc<dyn Credentials>) -> tauri::App<MockRuntime> {
        // The mock context carries no identifier, and every app directory is the
        // platform root joined with it — so a store opened here would land on
        // the real `settings.json` and overwrite the user's own. Joining an
        // absolute path replaces the root, so an absolute identifier moves all
        // of them into a temp folder instead.
        let config_dir = tempfile::tempdir().expect("a temp config dir");
        let mut context = mock_context(noop_assets());
        context.config_mut().identifier = config_dir.path().display().to_string();

        mock_builder()
            .plugin(tauri_plugin_store::Builder::new().build())
            .manage(AppState {
                credentials,
                ..AppState::default()
            })
            // Managed so the folder outlives every store the app opens and is
            // deleted with the app.
            .manage(config_dir)
            .build(context)
            .expect("the mock app must build")
    }
}
