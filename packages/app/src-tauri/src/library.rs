//! The open library: its folder layout and the single SQLite connection every
//! write goes through (design D1).

use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::Connection;

use crate::db;
use crate::error::{AppError, Result};

const IMAGES_DIR: &str = "images";
const INBOX_DIR: &str = "inbox";
const THUMBS_DIR: &str = ".thumbs";
const DB_FILE: &str = "library.sqlite";
const PART_EXT: &str = "part";

/// One open library folder. Everything it needs lives inside `root`, so the
/// folder can be copied to another machine and opened there unchanged.
pub struct Library {
    pub root: PathBuf,
    pub conn: Connection,
}

/// `rusqlite::Connection` is `Send` but not `Sync`, so the mutex is what makes
/// D1's "one connection, one writer" true rather than merely intended.
pub type SharedLibrary = std::sync::Arc<std::sync::Mutex<Option<Library>>>;

impl Library {
    /// Open the library at `root`, creating the folder layout and the database
    /// if they are not there yet. Only for a folder the user just chose: use
    /// `open_existing` for a path that was merely remembered.
    pub fn open_or_create(root: &Path) -> Result<Library> {
        let library = Library {
            root: root.to_path_buf(),
            conn: create_layout_then_open(root)?,
        };
        library.sweep_inbox()?;
        Ok(library)
    }

    /// Open a library that is already there, refusing to bring one into being.
    ///
    /// `library.sqlite` must exist, not merely the folder: an unmounted volume
    /// or a not-yet-synced cloud folder can leave an empty directory at the
    /// path, and creating a library into it would strand the user's captures
    /// somewhere they are not looking while the real library sits elsewhere.
    /// Deleting this check puts that bug back.
    pub fn open_existing(root: &Path) -> Result<Library> {
        if !root.is_dir() || !root.join(DB_FILE).is_file() {
            return Err(AppError::NotFound(format!("library at {}", root.display())));
        }
        Library::open_or_create(root)
    }

    pub fn images_dir(&self) -> PathBuf {
        self.root.join(IMAGES_DIR)
    }

    pub fn inbox_dir(&self) -> PathBuf {
        self.root.join(INBOX_DIR)
    }

    pub fn thumbs_dir(&self) -> PathBuf {
        self.root.join(THUMBS_DIR)
    }

    pub fn db_path(&self) -> PathBuf {
        self.root.join(DB_FILE)
    }

    pub fn image_path(&self, id: &str, ext: &str) -> PathBuf {
        self.images_dir().join(format!("{id}.{ext}"))
    }

    /// Where an upload lands before it is fsynced and renamed into `images/`
    /// (design D4).
    pub fn part_path(&self, id: &str) -> PathBuf {
        self.inbox_dir().join(format!("{id}.{PART_EXT}"))
    }

    /// Rows the user still has, deleted ones excluded.
    pub fn image_count(&self) -> Result<i64> {
        let count = self.conn.query_row(
            "SELECT COUNT(*) FROM images WHERE deleted_at IS NULL",
            [],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    /// A crash between the part file and the rename leaves an orphan behind.
    /// Nothing ever reads a `.part` back — the request that wrote it is long
    /// gone and reported failed — so startup is the moment to drop them.
    fn sweep_inbox(&self) -> Result<()> {
        for entry in fs::read_dir(self.inbox_dir())? {
            let path = entry?.path();
            if path.extension().is_some_and(|ext| ext == PART_EXT) {
                fs::remove_file(path)?;
            }
        }
        Ok(())
    }
}

fn create_layout_then_open(root: &Path) -> Result<Connection> {
    for dir in [
        root.join(IMAGES_DIR),
        root.join(INBOX_DIR),
        root.join(THUMBS_DIR),
    ] {
        fs::create_dir_all(dir)?;
    }
    db::open(&root.join(DB_FILE))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_the_layout_in_an_empty_folder() {
        let dir = tempfile::tempdir().unwrap();
        let library = Library::open_or_create(dir.path()).unwrap();

        assert!(library.images_dir().is_dir());
        assert!(library.inbox_dir().is_dir());
        assert!(library.thumbs_dir().is_dir());
        assert!(library.db_path().is_file());
        assert_eq!(library.image_count().unwrap(), 0);
    }

    #[test]
    fn reopens_an_existing_folder() {
        let dir = tempfile::tempdir().unwrap();
        drop(Library::open_or_create(dir.path()).unwrap());

        let library = Library::open_or_create(dir.path()).unwrap();
        assert_eq!(library.image_count().unwrap(), 0);
    }

    #[test]
    fn sweeps_part_files_left_by_a_crash() {
        let dir = tempfile::tempdir().unwrap();
        drop(Library::open_or_create(dir.path()).unwrap());

        let inbox = dir.path().join(INBOX_DIR);
        fs::write(inbox.join("abc.part"), b"half an upload").unwrap();
        fs::write(inbox.join("keep.txt"), b"not ours to delete").unwrap();

        drop(Library::open_or_create(dir.path()).unwrap());

        assert!(!inbox.join("abc.part").exists());
        assert!(inbox.join("keep.txt").exists());
    }

    #[test]
    fn image_path_names_the_file_by_id_and_extension() {
        let dir = tempfile::tempdir().unwrap();
        let library = Library::open_or_create(dir.path()).unwrap();

        assert_eq!(
            library.image_path("abc", "png"),
            library.images_dir().join("abc.png")
        );
    }

    #[test]
    fn open_existing_refuses_a_folder_that_is_not_there_and_creates_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let gone = dir.path().join("gone");

        assert!(
            matches!(Library::open_existing(&gone), Err(AppError::NotFound(_))),
            "a path that is not there must not open",
        );
        assert!(!gone.exists(), "a missing library must not be conjured up");
    }

    #[test]
    fn open_existing_refuses_a_folder_without_a_database() {
        // What an unmounted volume or an unsynced cloud folder looks like.
        let dir = tempfile::tempdir().unwrap();

        assert!(
            matches!(
                Library::open_existing(dir.path()),
                Err(AppError::NotFound(_))
            ),
            "a folder with no library.sqlite must not open",
        );
        assert!(!dir.path().join(DB_FILE).exists());
    }

    #[test]
    fn open_existing_opens_a_library_that_is_there() {
        let dir = tempfile::tempdir().unwrap();
        drop(Library::open_or_create(dir.path()).unwrap());

        let library = Library::open_existing(dir.path()).unwrap();

        assert_eq!(library.image_count().unwrap(), 0);
    }
}
