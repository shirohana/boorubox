//! The remembered library path and listener port, kept in the app config dir
//! rather than in the library, so a copied library carries no machine's paths
//! (design D6).

use std::path::{Path, PathBuf};

use serde_json::Value as JsonValue;
use tauri::{AppHandle, Runtime};
use tauri_plugin_store::StoreExt;

use crate::error::{AppError, Result};
use crate::from_tauri;
use crate::model::DEFAULT_PORT;

/// Resolved against the app config dir by tauri-plugin-store.
const SETTINGS_FILE: &str = "settings.json";

/// The store's keys. `settings.json` is a flat object, so these two names are
/// the whole file format: rename one and every existing file reads as defaults.
const LIBRARY_PATH: &str = "libraryPath";
const PORT: &str = "port";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    pub library_path: Option<PathBuf>,
    pub port: u16,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            library_path: None,
            port: DEFAULT_PORT,
        }
    }
}

/// Read the stored settings, falling back to the defaults: a missing or
/// unreadable store must not stop the app from opening.
pub fn load<R: Runtime>(app: &AppHandle<R>) -> Settings {
    load_from(app, SETTINGS_FILE)
}

pub fn save<R: Runtime>(app: &AppHandle<R>, settings: &Settings) -> Result<()> {
    save_to(app, SETTINGS_FILE, settings)
}

/// `file` reaches the store as given, and an absolute path wins over the app
/// config dir — which is how the round-trip test below stays out of the real one.
fn load_from<R: Runtime>(app: &AppHandle<R>, file: &str) -> Settings {
    let Ok(store) = app.store(file) else {
        return Settings::default();
    };
    let defaults = Settings::default();
    Settings {
        library_path: store
            .get(LIBRARY_PATH)
            .as_ref()
            .and_then(JsonValue::as_str)
            .map(PathBuf::from),
        // Every field falls back on its own: a settings file someone edited by
        // hand must not cost the user the other field.
        port: store
            .get(PORT)
            .as_ref()
            .and_then(JsonValue::as_u64)
            .and_then(|port| u16::try_from(port).ok())
            .unwrap_or(defaults.port),
    }
}

fn save_to<R: Runtime>(app: &AppHandle<R>, file: &str, settings: &Settings) -> Result<()> {
    let store = app.store(file).map_err(from_tauri)?;
    match &settings.library_path {
        Some(path) => store.set(LIBRARY_PATH, utf8(path)?),
        None => {
            store.delete(LIBRARY_PATH);
        }
    }
    store.set(PORT, settings.port);
    store.save().map_err(from_tauri)
}

/// JSON has no form for a path that is not UTF-8, so such a path is refused
/// rather than remembered lossily: a mangled path reopens as a second, empty
/// library beside the user's real one.
fn utf8(path: &Path) -> Result<&str> {
    path.to_str().ok_or_else(|| {
        AppError::BadRequest(format!(
            "library path is not valid UTF-8: {}",
            path.display()
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::mock_app;

    #[test]
    fn settings_round_trip_through_the_file() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("settings.json");
        let file = file.to_str().unwrap();
        let settings = Settings {
            library_path: Some(PathBuf::from("/tmp/boorubox-library")),
            port: 51234,
        };

        // Written by one app and read back by another: sharing one app would
        // only prove the store's in-memory cache remembers what it was told.
        let writer = mock_app();
        save_to(writer.handle(), file, &settings).unwrap();
        let reader = mock_app();

        assert_eq!(load_from(reader.handle(), file), settings);
    }

    #[test]
    fn a_store_that_is_not_there_reads_as_the_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("never-written.json");
        let app = mock_app();

        assert_eq!(
            load_from(app.handle(), file.to_str().unwrap()),
            Settings::default()
        );
    }

    #[test]
    fn forgetting_the_library_path_leaves_the_key_out() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("settings.json");
        let file = file.to_str().unwrap();

        let writer = mock_app();
        save_to(
            writer.handle(),
            file,
            &Settings {
                library_path: Some(PathBuf::from("/tmp/gone")),
                port: DEFAULT_PORT,
            },
        )
        .unwrap();
        save_to(writer.handle(), file, &Settings::default()).unwrap();
        let reader = mock_app();

        assert_eq!(load_from(reader.handle(), file).library_path, None);
    }
}
