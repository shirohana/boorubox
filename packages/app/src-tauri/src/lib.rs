pub mod commands;
pub mod db;
pub mod error;
pub mod http;
pub mod import;
pub mod ingest;
pub mod library;
pub mod maintenance;
pub mod model;
pub mod query;
pub mod settings;
pub mod tags;
pub mod thumbs;

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
        }
    }
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
            commands::recent_libraries,
            commands::forget_recent,
            commands::reveal_library,
            commands::set_listener_port,
            commands::app_settings,
            commands::set_theme,
            commands::set_grid_tile_size,
            commands::search,
            commands::tag_counts,
            commands::update_tags,
            commands::set_rating,
            commands::tag_suggestions,
            commands::image_counts,
            commands::drop_image_record,
            commands::thumbnail_path,
            commands::import_paths,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Everything that must be true before the window appears: the last library
/// open if it still opens, and the capture listener bound.
///
/// Nothing here can fail the launch. A library that will not open leaves the
/// app on the start screen with the path named (spec `library-folder`), and a
/// port already taken leaves the listener stopped with its reason on the
/// settings screen (spec `capture-ingest`, design D6).
fn open_remembered_library_and_listen(app: &tauri::App) {
    let handle = app.handle().clone();
    let state = app.state::<AppState>();
    let settings = settings::load(&handle);

    if let Some(path) = &settings.library_path {
        // The path stays in settings whether or not it opened: `library_status`
        // reports a remembered path with no library open as `missing_path`, and
        // only picking another replaces it. `ExistingOnly` is what makes that
        // reachable — creating the folder here would report a healthy empty
        // library instead of a missing one.
        let _ = commands::open_into_state(&handle, &state, path, commands::OpenMode::ExistingOnly);
    }

    tauri::async_runtime::block_on(commands::rebind_listener(&handle, &state, settings.port));
    *lock(&state.settings) = settings;
}

/// A headless app the command and settings tests can hand to the code under
/// test wherever it wants an `AppHandle`.
#[cfg(test)]
pub mod test_support {
    use tauri::test::{MockRuntime, mock_builder, mock_context, noop_assets};

    use crate::AppState;

    pub fn mock_app() -> tauri::App<MockRuntime> {
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
            .manage(AppState::default())
            // Managed so the folder outlives every store the app opens and is
            // deleted with the app.
            .manage(config_dir)
            .build(context)
            .expect("the mock app must build")
    }
}
