//! The remembered library path and listener port, kept in the app config dir
//! rather than in the library, so a copied library carries no machine's paths
//! (design D6).

use std::path::{Path, PathBuf};

use serde_json::Value as JsonValue;
use tauri::{AppHandle, Runtime};
use tauri_plugin_store::StoreExt;

use crate::error::{AppError, Result};
use crate::from_tauri;
use crate::model::{
    CLICK_ZOOM_CEILING_DEFAULT, CLICK_ZOOM_CEILING_MAX, CLICK_ZOOM_CEILING_MIN, DEFAULT_PORT,
    GRID_TILE_DEFAULT, GRID_TILE_MAX, GRID_TILE_MIN, Theme,
};

/// Resolved against the app config dir by tauri-plugin-store.
const SETTINGS_FILE: &str = "settings.json";

/// The store's keys. `settings.json` is a flat object, so these names are the
/// whole file format: rename one and every existing file reads as defaults.
const LIBRARY_PATH: &str = "libraryPath";
const PORT: &str = "port";
const THEME: &str = "theme";
const GRID_TILE_SIZE: &str = "gridTileSize";
const SHOW_TILE_TAGS: &str = "showTileTags";
const CLICK_ZOOM_CEILING_PERCENT: &str = "clickZoomCeilingPercent";
const RECENT_LIBRARIES: &str = "recentLibraries";
const NOTES_COLLAPSED: &str = "notesCollapsed";
const COLLECTIONS_COLLAPSED: &str = "collectionsCollapsed";
const OPEN_LAST_ON_LAUNCH: &str = "openLastOnLaunch";

/// How many folders the recent list keeps (design D4). A bound, not a
/// measurement: the list is there to be clicked through, and a longer one is a
/// file browser. Raising it changes this and nothing else.
const RECENT_LIMIT: usize = 10;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    pub library_path: Option<PathBuf>,
    pub port: u16,
    pub theme: Theme,
    pub grid_tile_size: u32,
    /// Whether the grid's tag footer shows outside edit mode, where the mode
    /// forces it on regardless (`tile-tags-in-edit-mode` design D3). Kept
    /// beside `grid_tile_size`: a display preference of this machine, not of
    /// the library.
    pub show_tile_tags: bool,
    /// How far a click in the viewer may zoom, as a percent of the fit
    /// (`click-zoom-ceiling` design D1).
    pub click_zoom_ceiling_percent: u32,
    /// Absolute paths, most recent first, each folder once (design D4).
    pub recent_libraries: Vec<PathBuf>,
    /// Whether the sidebar's notes panel is folded away (`notes` design D13).
    /// A machine's display preference, so it belongs here rather than in the
    /// library the note's own text lives in.
    pub notes_collapsed: bool,
    /// Whether the sidebar's collections section is folded away
    /// (`browse-feedback` design D4), copied from `notes_collapsed`'s line.
    pub collections_collapsed: bool,
    /// Whether `setup` reopens `library_path` automatically at launch
    /// (`launch-screen` design D3). On by default so an existing settings
    /// file, which has never written this key, keeps today's behaviour.
    /// Off leaves the start screen showing the recent list instead.
    pub open_last_on_launch: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            library_path: None,
            port: DEFAULT_PORT,
            theme: Theme::default(),
            grid_tile_size: GRID_TILE_DEFAULT,
            // Off: edit mode already forces the footer on, so a fresh
            // profile's browse grid stays as dense as it was before this
            // setting existed.
            show_tile_tags: false,
            click_zoom_ceiling_percent: CLICK_ZOOM_CEILING_DEFAULT,
            recent_libraries: Vec::new(),
            // Expanded: a panel nobody asked to hide is a panel the user has
            // not seen yet.
            notes_collapsed: false,
            collections_collapsed: false,
            open_last_on_launch: true,
        }
    }
}

impl Settings {
    /// Put `path` at the front of the recent list, once, dropping the oldest
    /// past the cap.
    pub fn remember_recent(&mut self, path: &Path) {
        self.forget_recent(path);
        self.recent_libraries.insert(0, path.to_path_buf());
        self.recent_libraries.truncate(RECENT_LIMIT);
    }

    /// Deduplication is by the exact path: two spellings of one folder are two
    /// entries, which is visible and harmless, where normalising would have to
    /// resolve symlinks on a volume that may not be mounted.
    pub fn forget_recent(&mut self, path: &Path) {
        self.recent_libraries.retain(|known| known != path);
    }
}

impl From<&Settings> for crate::model::AppSettings {
    fn from(settings: &Settings) -> Self {
        crate::model::AppSettings {
            theme: settings.theme,
            grid_tile_size: settings.grid_tile_size,
            show_tile_tags: settings.show_tile_tags,
            click_zoom_ceiling_percent: settings.click_zoom_ceiling_percent,
            notes_collapsed: settings.notes_collapsed,
            collections_collapsed: settings.collections_collapsed,
            open_last_on_launch: settings.open_last_on_launch,
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
        // hand must not cost the user the other fields.
        port: store
            .get(PORT)
            .as_ref()
            .and_then(JsonValue::as_u64)
            .and_then(|port| u16::try_from(port).ok())
            .unwrap_or(defaults.port),
        theme: store
            .get(THEME)
            .and_then(|theme| serde_json::from_value(theme).ok())
            .unwrap_or(defaults.theme),
        // A size outside the range reads as the default rather than being
        // clamped: `set_grid_tile_size` is the only writer that clamps, so
        // anything out of range here was hand-edited and is not a preference.
        grid_tile_size: store
            .get(GRID_TILE_SIZE)
            .as_ref()
            .and_then(JsonValue::as_u64)
            .and_then(|size| u32::try_from(size).ok())
            .filter(|size| (GRID_TILE_MIN..=GRID_TILE_MAX).contains(size))
            .unwrap_or(defaults.grid_tile_size),
        show_tile_tags: store
            .get(SHOW_TILE_TAGS)
            .as_ref()
            .and_then(JsonValue::as_bool)
            .unwrap_or(defaults.show_tile_tags),
        // Same reasoning as `grid_tile_size`: `set_click_zoom_ceiling_percent`
        // is the only writer that clamps, so anything out of range here was
        // hand-edited and is not a preference.
        click_zoom_ceiling_percent: store
            .get(CLICK_ZOOM_CEILING_PERCENT)
            .as_ref()
            .and_then(JsonValue::as_u64)
            .and_then(|percent| u32::try_from(percent).ok())
            .filter(|percent| (CLICK_ZOOM_CEILING_MIN..=CLICK_ZOOM_CEILING_MAX).contains(percent))
            .unwrap_or(defaults.click_zoom_ceiling_percent),
        recent_libraries: store
            .get(RECENT_LIBRARIES)
            .as_ref()
            .and_then(JsonValue::as_array)
            .map(|entries| {
                entries
                    .iter()
                    .filter_map(JsonValue::as_str)
                    .map(PathBuf::from)
                    .take(RECENT_LIMIT)
                    .collect()
            })
            .unwrap_or(defaults.recent_libraries),
        notes_collapsed: store
            .get(NOTES_COLLAPSED)
            .as_ref()
            .and_then(JsonValue::as_bool)
            .unwrap_or(defaults.notes_collapsed),
        collections_collapsed: store
            .get(COLLECTIONS_COLLAPSED)
            .as_ref()
            .and_then(JsonValue::as_bool)
            .unwrap_or(defaults.collections_collapsed),
        open_last_on_launch: store
            .get(OPEN_LAST_ON_LAUNCH)
            .as_ref()
            .and_then(JsonValue::as_bool)
            .unwrap_or(defaults.open_last_on_launch),
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
    // `model.rs`'s serde is the one spelling of the theme names, in the file as
    // on the wire; a unit variant has no failing branch to handle.
    store.set(
        THEME,
        serde_json::to_value(settings.theme).unwrap_or_default(),
    );
    store.set(GRID_TILE_SIZE, settings.grid_tile_size);
    store.set(SHOW_TILE_TAGS, settings.show_tile_tags);
    store.set(
        CLICK_ZOOM_CEILING_PERCENT,
        settings.click_zoom_ceiling_percent,
    );
    store.set(OPEN_LAST_ON_LAUNCH, settings.open_last_on_launch);
    let recent = settings
        .recent_libraries
        .iter()
        .map(|path| utf8(path))
        .collect::<Result<Vec<&str>>>()?;
    store.set(RECENT_LIBRARIES, recent);
    store.set(NOTES_COLLAPSED, settings.notes_collapsed);
    store.set(COLLECTIONS_COLLAPSED, settings.collections_collapsed);
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
            theme: Theme::Dark,
            grid_tile_size: 240,
            show_tile_tags: true,
            click_zoom_ceiling_percent: 350,
            recent_libraries: vec![
                PathBuf::from("/tmp/boorubox-library"),
                PathBuf::from("/tmp/older"),
            ],
            notes_collapsed: true,
            collections_collapsed: true,
            open_last_on_launch: false,
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
                ..Settings::default()
            },
        )
        .unwrap();
        save_to(writer.handle(), file, &Settings::default()).unwrap();
        let reader = mock_app();

        assert_eq!(load_from(reader.handle(), file).library_path, None);
    }

    /// The whole point of the per-field fallback: a theme nobody knows must not
    /// cost the user the library path stored beside it.
    #[test]
    fn a_theme_the_build_does_not_know_reads_as_system() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("settings.json");
        let file = file.to_str().unwrap();
        let writer = mock_app();
        save_to(
            writer.handle(),
            file,
            &Settings {
                library_path: Some(PathBuf::from("/tmp/kept")),
                theme: Theme::Dark,
                ..Settings::default()
            },
        )
        .unwrap();
        let store = writer.handle().store(file).unwrap();
        store.set(THEME, "chartreuse");
        store.save().unwrap();

        let loaded = load_from(mock_app().handle(), file);

        assert_eq!(loaded.theme, Theme::System);
        assert_eq!(loaded.library_path, Some(PathBuf::from("/tmp/kept")));
    }

    #[test]
    fn a_tile_size_outside_the_range_reads_as_the_default() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("settings.json");
        let file = file.to_str().unwrap();
        let writer = mock_app();
        let store = writer.handle().store(file).unwrap();
        store.set(GRID_TILE_SIZE, GRID_TILE_MAX + 1);
        store.save().unwrap();

        assert_eq!(
            load_from(mock_app().handle(), file).grid_tile_size,
            GRID_TILE_DEFAULT,
        );

        store.set(GRID_TILE_SIZE, "wide");
        store.save().unwrap();

        assert_eq!(
            load_from(mock_app().handle(), file).grid_tile_size,
            GRID_TILE_DEFAULT,
        );
    }

    /// The footer nobody has turned on yet is off: edit mode already forces
    /// it, so a file that predates this setting must not suddenly show
    /// footers in the browse grid too.
    #[test]
    fn a_file_without_the_show_tile_tags_key_reads_as_off() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("settings.json");
        let file = file.to_str().unwrap();
        let writer = mock_app();
        let store = writer.handle().store(file).unwrap();
        store.set(GRID_TILE_SIZE, GRID_TILE_DEFAULT);
        store.save().unwrap();

        assert!(!load_from(mock_app().handle(), file).show_tile_tags);
    }

    #[test]
    fn a_click_zoom_ceiling_outside_the_range_reads_as_the_default() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("settings.json");
        let file = file.to_str().unwrap();
        let writer = mock_app();
        let store = writer.handle().store(file).unwrap();
        store.set(CLICK_ZOOM_CEILING_PERCENT, CLICK_ZOOM_CEILING_MAX + 1);
        store.save().unwrap();

        assert_eq!(
            load_from(mock_app().handle(), file).click_zoom_ceiling_percent,
            CLICK_ZOOM_CEILING_DEFAULT,
        );

        store.set(CLICK_ZOOM_CEILING_PERCENT, "wide");
        store.save().unwrap();

        assert_eq!(
            load_from(mock_app().handle(), file).click_zoom_ceiling_percent,
            CLICK_ZOOM_CEILING_DEFAULT,
        );
    }

    /// The panel a file has never mentioned is the expanded one: the setting
    /// exists to remember a fold the user chose, not to open the app folded.
    #[test]
    fn a_file_without_the_notes_key_reads_as_expanded() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("settings.json");
        let file = file.to_str().unwrap();
        let writer = mock_app();
        let store = writer.handle().store(file).unwrap();
        store.set(GRID_TILE_SIZE, GRID_TILE_DEFAULT);
        store.save().unwrap();

        assert!(!load_from(mock_app().handle(), file).notes_collapsed);
    }

    /// The section nobody asked to hide is the expanded one, the same
    /// reasoning as the notes panel's own default.
    #[test]
    fn a_file_without_the_collections_key_reads_as_expanded() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("settings.json");
        let file = file.to_str().unwrap();
        let writer = mock_app();
        let store = writer.handle().store(file).unwrap();
        store.set(GRID_TILE_SIZE, GRID_TILE_DEFAULT);
        store.save().unwrap();

        assert!(!load_from(mock_app().handle(), file).collections_collapsed);
    }

    /// `launch-screen` design D3: a settings file written before this setting
    /// existed must keep reopening the remembered library, not silently
    /// switch the user to the start screen.
    #[test]
    fn a_file_without_the_open_last_on_launch_key_reads_as_on() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("settings.json");
        let file = file.to_str().unwrap();
        let writer = mock_app();
        let store = writer.handle().store(file).unwrap();
        store.set(GRID_TILE_SIZE, GRID_TILE_DEFAULT);
        store.save().unwrap();

        assert!(load_from(mock_app().handle(), file).open_last_on_launch);
    }

    #[test]
    fn remembering_a_library_puts_it_first_and_keeps_it_once() {
        let mut settings = Settings::default();

        settings.remember_recent(Path::new("/a"));
        settings.remember_recent(Path::new("/b"));
        settings.remember_recent(Path::new("/a"));

        assert_eq!(
            settings.recent_libraries,
            vec![PathBuf::from("/a"), PathBuf::from("/b")],
        );
    }

    #[test]
    fn the_recent_list_drops_the_oldest_past_the_cap() {
        let mut settings = Settings::default();

        for index in 0..RECENT_LIMIT + 2 {
            settings.remember_recent(Path::new(&format!("/library-{index}")));
        }

        assert_eq!(settings.recent_libraries.len(), RECENT_LIMIT);
        assert_eq!(
            settings.recent_libraries.first(),
            Some(&PathBuf::from(format!("/library-{}", RECENT_LIMIT + 1))),
        );
        assert!(
            !settings
                .recent_libraries
                .contains(&PathBuf::from("/library-0")),
            "the oldest entry must be the one that goes: {:?}",
            settings.recent_libraries,
        );
    }

    #[test]
    fn forgetting_an_entry_leaves_the_rest_in_order() {
        let mut settings = Settings::default();
        settings.remember_recent(Path::new("/a"));
        settings.remember_recent(Path::new("/b"));
        settings.remember_recent(Path::new("/c"));

        settings.forget_recent(Path::new("/b"));

        assert_eq!(
            settings.recent_libraries,
            vec![PathBuf::from("/c"), PathBuf::from("/a")],
        );
    }
}
