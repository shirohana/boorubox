//! The open library: its folder layout and the single SQLite connection every
//! write goes through (design D1).

use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::Connection;

use crate::db;
use crate::error::{AppError, Result};
use crate::sidecar;

const IMAGES_DIR: &str = "images";
const INBOX_DIR: &str = "inbox";
const THUMBS_DIR: &str = ".thumbs";
const DB_FILE: &str = "library.sqlite";
const PART_EXT: &str = "part";

/// One open library folder. Everything it needs lives inside `root`, so the
/// folder can be copied to another machine and opened there unchanged.
pub struct Library {
    pub paths: LibraryPaths,
    pub conn: Connection,
}

/// Where a library folder keeps its files, without the connection that reads
/// them. Split out so work that only touches files — thumbnailing (design D13)
/// — can be carried past the mutex and done with the library released.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LibraryPaths {
    pub root: PathBuf,
}

/// `rusqlite::Connection` is `Send` but not `Sync`, so the mutex is what makes
/// D1's "one connection, one writer" true rather than merely intended.
pub type SharedLibrary = std::sync::Arc<std::sync::Mutex<Option<Library>>>;

/// Borrow the open library, or fail with `NoLibrary`. `work` is a plain closure
/// on purpose: `SharedLibrary` is a `std::sync::Mutex`, so nothing holding this
/// lock may `.await` (see `http::captures::store`).
pub fn with_library<T>(
    library: &SharedLibrary,
    work: impl FnOnce(&Library) -> Result<T>,
) -> Result<T> {
    with_library_if_open(library, |open| work(open.ok_or(AppError::NoLibrary)?))
}

/// Borrow whatever is open, `None` included. Only for the callers a closed
/// library is an answer to rather than a refusal — `library_status` reports a
/// closed library, it does not fail on one.
///
/// Every command and every capture reaches the database through here, which is
/// why this is where a rusqlite corrupt-class code becomes `LibraryCorrupt`
/// (`library-sidecars` design D8: the same message "wherever the damage
/// surfaces ... at open or mid-session"). `?` has already turned it into `Db`
/// by the time `work` returns, and this is the innermost frame that knows
/// which file it happened to.
pub fn with_library_if_open<T>(
    library: &SharedLibrary,
    work: impl FnOnce(Option<&Library>) -> Result<T>,
) -> Result<T> {
    // A poisoned mutex means some earlier request panicked while holding the
    // library. The connection survives that (an open transaction rolls back when
    // it drops), and refusing every later capture would be the larger outage.
    let guard = library
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    match (work(guard.as_ref()), guard.as_ref()) {
        (Err(error), Some(open)) => Err(error.or_corrupt(&open.paths.db_path())),
        (result, _) => result,
    }
}

/// Write a sidecar for every image that does not have one, and `library.json`
/// if it is absent (design D7): the repair "opening a library SHALL write a
/// describing file for every image that has none" runs from here, called by
/// `commands::open_into_state` on a blocking thread.
///
/// The one query takes the library once, briefly; the 25,000 stats after it do
/// not — `LibraryPaths` is `Clone` and a file check needs nothing else — and
/// each write takes it again for one image through `with_library` (Phase 1
/// D13's shape, the same `import::import_paths` and `rules::run` use), which
/// is what lets a search or a capture during the pass be answered between two
/// of them.
///
/// `root` is the path this pass was started for: every write checks the open
/// library's root still matches it, so a switch mid-pass (`pending-work`
/// spec's "Switching libraries mid-pass") stops the pass without writing into
/// a folder it no longer belongs to, rather than needing a control handle of
/// its own (design D7 — "this pass has no controls").
pub fn backfill_sidecars(
    library: &SharedLibrary,
    root: &Path,
    on_progress: &mut dyn FnMut(i64, i64),
) -> Result<()> {
    let (paths, ids) = with_library(library, |open| {
        Ok((open.paths.clone(), all_image_ids(&open.conn)?))
    })?;
    if paths.root != root {
        return Ok(());
    }

    let missing: Vec<&String> = ids
        .iter()
        .filter(|id| !sidecar::path(&paths, id).is_file())
        .collect();

    // A library already in step must show no tile at all (`pending-work`
    // spec's "A library already in step") — the webview's tile hides once
    // `done >= total` but shows for *any* event with `total > 0`, so even a
    // single `(0, 0)` tick here would flash one on every open. `total` is
    // therefore the count of rows that still lack a sidecar, and nothing at
    // all is emitted when that count is zero.
    let total = missing.len() as i64;
    if total > 0 {
        on_progress(0, total);
        for (index, id) in missing.iter().enumerate() {
            let stopped = with_library_if_open(library, |open| match open {
                Some(open) if open.paths.root == root => {
                    sidecar::write_one(&open.paths, &open.conn, id)?;
                    Ok(false)
                }
                // The library closed, or a switch moved it onto another root
                // (`open_into_state`'s D13's "tears a library down"): nothing
                // more of this pass belongs to write.
                _ => Ok(true),
            })?;
            if stopped {
                return Ok(());
            }
            on_progress(index as i64 + 1, total);
        }
    }

    write_library_json_if_missing(library, root)
}

/// `library.json` is written by the same pass when it is absent (design D7),
/// unconditionally on the sidecar count above: a library with every sidecar
/// already in place can still be missing its one library-level file, and this
/// repair carries no progress event of its own to gate on.
fn write_library_json_if_missing(library: &SharedLibrary, root: &Path) -> Result<()> {
    with_library_if_open(library, |open| match open {
        Some(open) if open.paths.root == root => {
            if !sidecar::library_path(&open.paths).is_file() {
                sidecar::write_library(&open.paths, &open.conn)?;
            }
            Ok(())
        }
        _ => Ok(()),
    })
}

/// Every id in the library, trashed rows included — shared with
/// `thumbs::regenerate_all` (`one-level-buckets` design D4), which needs the
/// same "everything, deleted or not" read `backfill_sidecars` already does: a
/// trashed image keeps its thumbnail until it is deleted forever.
pub(crate) fn all_image_ids(conn: &Connection) -> Result<Vec<String>> {
    let mut stmt = conn.prepare("SELECT id FROM images")?;
    let ids = stmt.query_map([], |row| row.get::<_, String>(0))?;
    Ok(ids.collect::<rusqlite::Result<_>>()?)
}

impl Library {
    /// Open the library at `root`, creating the folder layout and the database
    /// if they are not there yet. Only for a folder the user just chose: use
    /// `open_existing` for a path that was merely remembered.
    pub fn open_or_create(root: &Path) -> Result<Library> {
        let library = Library {
            paths: LibraryPaths {
                root: root.to_path_buf(),
            },
            conn: create_layout_then_open(root)?,
        };
        library.relayout_to_one_level()?;
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
        if !root.is_dir() || !database_path(root).is_file() {
            return Err(AppError::NotFound(format!("library at {}", root.display())));
        }
        Library::open_or_create(root)
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
        for entry in fs::read_dir(self.paths.inbox_dir())? {
            let path = entry?.path();
            if path.extension().is_some_and(|ext| ext == PART_EXT) {
                fs::remove_file(path)?;
            }
        }
        Ok(())
    }

    /// Move files under `images/` and `.thumbs/` into today's one-level
    /// bucket shape (`one-level-buckets` design D2), from either of the two
    /// shapes an existing library might be in: flat under the top level, or
    /// one level too deep at `<a1>/<b2>/`. Idempotent: one `read_dir` of
    /// `images/`, one of `.thumbs/`, and one `read_dir` per bucket — every
    /// file-or-directory check reads `DirEntry::file_type()` off the listing
    /// itself, no stat per file.
    fn relayout_to_one_level(&self) -> Result<()> {
        let paths = &self.paths;
        relayout_one_level(&paths.images_dir(), |stem, ext| {
            Some(paths.image_path(stem, ext))
        })?;
        relayout_one_level(&paths.thumbs_dir(), |stem, ext| {
            // A flat file under `.thumbs/` with any other extension is not one
            // of ours — `thumbnail_path` only ever names a `.jpg` — and must
            // not be renamed onto a `.jpg` it never had.
            (ext == "jpg").then(|| crate::thumbs::thumbnail_path(paths, stem))
        })
    }
}

/// `relayout_flat_files` reaches the bucket shape from a pre-shard library;
/// this reaches it from a two-level one, then does the same for `dir` itself
/// so a library in either stale shape lands in the one this change targets in
/// a single open (`one-level-buckets` design D2).
fn relayout_one_level(dir: &Path, dest_for: impl Fn(&str, &str) -> Option<PathBuf>) -> Result<()> {
    relayout_flat_files(dir, &dest_for)?;
    lift_second_level(dir)
}

/// Move every regular file sitting at `<a1>/<b2>/<name>` up to `<a1>/<name>`,
/// and remove `<b2>/` once nothing is left in it (design D2). The destination
/// is `<a1>/<name>` verbatim, the same rule `relayout_flat_files` follows: the
/// walk never recomputes a bucket from a stem, so a file already in the wrong
/// bucket stays there rather than being "fixed" into a path the record does
/// not expect. A `<b2>/` a file could not leave — a stale destination, a
/// nested directory, a rename that failed — is left for the next open to try
/// again, exactly as `relayout_flat_files` leaves a file it could not move.
fn lift_second_level(dir: &Path) -> Result<()> {
    let Ok(buckets) = fs::read_dir(dir) else {
        return Ok(());
    };
    for bucket in buckets {
        let Ok(bucket) = bucket else {
            continue;
        };
        let Ok(bucket_type) = bucket.file_type() else {
            continue;
        };
        if !bucket_type.is_dir() {
            continue;
        }
        let bucket = bucket.path();
        let Ok(subdirs) = fs::read_dir(&bucket) else {
            continue;
        };
        for subdir in subdirs {
            let Ok(subdir) = subdir else {
                continue;
            };
            let Ok(subdir_type) = subdir.file_type() else {
                continue;
            };
            if !subdir_type.is_dir() {
                continue;
            }
            let subdir = subdir.path();
            // A sync client (Dropbox, iCloud) can evict `<b2>/` between this
            // listing and `relayout_flat_files` reading it; that one bucket
            // is left for the next open to try again, the same swallow every
            // per-file failure inside `relayout_flat_files` gets, rather than
            // failing the whole library open over it.
            let _ = relayout_flat_files(&subdir, |stem, ext| {
                Some(bucket.join(format!("{stem}.{ext}")))
            });
            // The OS's own droppings (Finder's `.DS_Store`, Explorer's
            // `Thumbs.db`) reappear in any directory it is shown, not one
            // anything here wrote to; removed like a stray `.part`, or a
            // `<b2>/` that holds only one of these never empties and is
            // re-walked on every open.
            for name in [".DS_Store", "Thumbs.db"] {
                let _ = fs::remove_file(subdir.join(name));
            }
            let _ = fs::remove_dir(&subdir);
        }
    }
    Ok(())
}

/// Move every regular file directly under `dir` to where `dest_for` (its stem
/// and extension, `None` if the entry is not this directory's to bucket) says
/// it belongs, creating the destination's bucket first (design D3). A stray
/// `.part` — a thumbnail encode that crashed mid-write — is removed rather
/// than moved; an inbox write's `.part` lives under `inbox/` and never reaches
/// here, but the check costs nothing to keep on both directories this walks.
///
/// Every failure here — the entry, the bucket `mkdir`, the link or its
/// fallback rename — is swallowed rather than propagated: a Dropbox
/// "conflicted copy", an iCloud placeholder, or (design D4 did not
/// anticipate this) a stray file that happens to be named exactly one id's
/// two-character bucket prefix, blocking `create_dir_all` for every id
/// sharding into it, must cost that one file its migration, not the library
/// its ability to open. A file already sitting at its destination — a
/// library caught mid-migration, or a conflicted copy restored flat — is
/// left alone rather than overwritten.
fn relayout_flat_files(dir: &Path, dest_for: impl Fn(&str, &str) -> Option<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let Ok(path) = entry.map(|entry| entry.path()) else {
            continue;
        };
        if !path.is_file() {
            continue;
        }
        if path.extension().is_some_and(|ext| ext == PART_EXT) {
            let _ = fs::remove_file(&path);
            continue;
        }
        let (Some(stem), Some(ext)) = (
            path.file_stem().and_then(|stem| stem.to_str()),
            path.extension().and_then(|ext| ext.to_str()),
        ) else {
            continue;
        };
        let Some(dest) = dest_for(stem, ext) else {
            continue;
        };
        if let Some(bucket) = dest.parent()
            && fs::create_dir_all(bucket).is_err()
        {
            continue;
        }
        // A hard link, not a rename: `rename` overwrites its destination on
        // unix, so a file a cloud client lands at `dest` in the window
        // between a `dest.exists()` check and the move would be destroyed.
        // `hard_link` instead fails atomically with `AlreadyExists`, and that
        // file is left exactly as `dest.exists()` used to leave it.
        match fs::hard_link(&path, &dest) {
            Ok(()) => {
                let _ = fs::remove_file(&path);
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            // A filesystem with no hard links (exFAT, some network shares):
            // fall back to the rename this replaced, `dest.exists()` checked
            // first since a plain `rename` would otherwise overwrite it.
            Err(_) => {
                if !dest.exists() {
                    let _ = fs::rename(&path, &dest);
                }
            }
        }
    }
    Ok(())
}

impl LibraryPaths {
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
        database_path(&self.root)
    }

    /// Pushed component by component, like `thumbs::thumbnail_path` — never
    /// through `relative_image_path`'s `/`-joined string, which would splice a
    /// literal `/` into a `root` that is otherwise all native separators (a
    /// mixed-separator path in a user-visible report on Windows).
    pub fn image_path(&self, id: &str, ext: &str) -> PathBuf {
        let mut path = self.images_dir();
        path.extend(shard_dirs(id));
        path.push(format!("{id}.{ext}"));
        path
    }

    /// `images/<a1>/<id>.<ext>` (`one-level-buckets` design D1): the path
    /// `ImageRecord.file` carries, so it is a `/`-joined string, not a
    /// `PathBuf` — the webview reads this same value with no path module of
    /// its own (design D2). Filesystem paths (`image_path`, `thumbnail_path`)
    /// never build on this; both instead push `shard_dirs`' components
    /// directly, the one thing the two representations share.
    pub fn relative_image_path(id: &str, ext: &str) -> String {
        let mut components: Vec<&str> = vec![IMAGES_DIR];
        components.extend(shard_dirs(id));
        format!("{}/{id}.{ext}", components.join("/"))
    }

    /// Where an upload lands before it is fsynced and renamed into `images/`
    /// (design D4).
    pub fn part_path(&self, id: &str) -> PathBuf {
        self.inbox_dir().join(format!("{id}.{PART_EXT}"))
    }
}

/// The bucket directory `id` shards into (`one-level-buckets` design D1): one
/// level, the first two characters, nothing after them. An id shorter than
/// two characters uses what it has, omitting a level it
/// cannot fill rather than emitting one with an empty name (`a` -> `["a"]`,
/// `abc` -> `["ab"]`). Shared by `LibraryPaths::relative_image_path` and
/// `thumbs::thumbnail_path`, the one definition of the bucket rule.
///
/// FIXME: this never validates `id` — a `/` or `..` inside one would escape
/// the bucket it should be confined to, and nothing stops an id built from
/// characters no filesystem accepts in a name. Slicing by character (below)
/// only keeps this from panicking on one; it validates nothing. The right
/// place for that check is the capture door (`ingest::store_image`), before an
/// id ever reaches a path; not built yet (design D1's non-goal).
pub(crate) fn shard_dirs(id: &str) -> Vec<&str> {
    // A byte index of a character boundary, never a byte offset: an id is
    // arbitrary and unvalidated (the `FIXME` above), and `id[..2]` panics the
    // instant one non-ASCII character puts a byte boundary mid-character —
    // which the relayout in `Library::relayout_to_one_level` would then hit on
    // every stem in the directory, not just the one bad id.
    let a1_end = char_boundary(id, 2);
    [&id[..a1_end]]
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect()
}

/// The byte index at which `id`'s `nth` character starts, or `id.len()` if it
/// has fewer.
fn char_boundary(id: &str, nth: usize) -> usize {
    id.char_indices()
        .nth(nth)
        .map_or(id.len(), |(index, _)| index)
}

/// The database inside a library folder. The recent list asks this of folders
/// it has not opened, so the filename has one definition and an entry can never
/// be judged available by a name the loader does not use.
pub fn database_path(root: &Path) -> PathBuf {
    root.join(DB_FILE)
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

        assert!(library.paths.images_dir().is_dir());
        assert!(library.paths.inbox_dir().is_dir());
        assert!(library.paths.thumbs_dir().is_dir());
        assert!(library.paths.db_path().is_file());
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
    fn image_path_is_the_images_dir_with_the_shard_and_file_pushed_as_components() {
        let dir = tempfile::tempdir().unwrap();
        let library = Library::open_or_create(dir.path()).unwrap();

        assert_eq!(
            library.paths.image_path("abc", "png"),
            library.paths.images_dir().join("ab").join("abc.png"),
            "never built by joining a `/`-string onto root (design D2's mixed-separator risk)"
        );
    }

    #[test]
    fn a_uuid_id_shards_one_bucket_level() {
        assert_eq!(
            LibraryPaths::relative_image_path("a1b2c3d4-e5f6-7890-abcd-ef1234567890", "jpg"),
            "images/a1/a1b2c3d4-e5f6-7890-abcd-ef1234567890.jpg"
        );
    }

    #[test]
    fn a_one_character_id_uses_one_bucket_level() {
        assert_eq!(
            LibraryPaths::relative_image_path("a", "png"),
            "images/a/a.png"
        );
    }

    #[test]
    fn a_three_character_id_shards_its_first_two_characters() {
        assert_eq!(
            LibraryPaths::relative_image_path("abc", "png"),
            "images/ab/abc.png"
        );
    }

    /// A byte-index slice (`id[..2]`) panics the moment a two-character
    /// prefix lands mid-character; design D1 says "characters", and the id
    /// arrives from the capture listener with nothing checking its charset.
    #[test]
    fn a_non_ascii_id_shards_by_character_not_byte() {
        assert_eq!(
            LibraryPaths::relative_image_path("日本語テスト", "png"),
            "images/日本/日本語テスト.png"
        );
    }

    #[test]
    fn a_flat_image_moves_into_its_bucket_on_open_and_the_record_still_loads() {
        let dir = tempfile::tempdir().unwrap();
        {
            let library = Library::open_or_create(dir.path()).unwrap();
            let bytes = png_bytes();
            crate::ingest::store_image(
                &library,
                crate::ingest::IngestInput {
                    id: "abc",
                    bytes: &bytes,
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
            // Simulate a library from before the sharded layout: move the file
            // that `store_image` bucketed back out flat, the shape a real
            // pre-existing library was actually found in.
            let bucketed = library.paths.image_path("abc", "png");
            let flat = library.paths.root.join(IMAGES_DIR).join("abc.png");
            fs::rename(&bucketed, &flat).unwrap();
        }

        let library = Library::open_or_create(dir.path()).unwrap();

        let bucketed = library.paths.image_path("abc", "png");
        assert!(
            bucketed.is_file(),
            "the flat file must move into its bucket"
        );
        assert!(!library.paths.root.join(IMAGES_DIR).join("abc.png").exists());
        let record = crate::ingest::load_record(&library.conn, "abc")
            .unwrap()
            .unwrap();
        assert_eq!(record.file, "images/ab/abc.png");
    }

    /// The hard-link move (design D3, review of `f126c3f`): the destination
    /// carries the exact bytes and the flat copy is gone, the same outcome a
    /// `rename` gave — a TOCTOU race between the two is not portable to force
    /// in a test, so this only pins the happy path the link/remove pair must
    /// keep matching.
    #[test]
    fn a_flat_file_moved_into_its_bucket_is_byte_identical_and_the_flat_copy_is_gone() {
        let dir = tempfile::tempdir().unwrap();
        let library = Library::open_or_create(dir.path()).unwrap();
        let flat_images = library.paths.root.join(IMAGES_DIR);
        fs::write(flat_images.join("abc.png"), b"the original bytes").unwrap();
        drop(library);

        let library = Library::open_or_create(dir.path()).unwrap();

        let bucketed = library.paths.image_path("abc", "png");
        assert_eq!(fs::read(&bucketed).unwrap(), b"the original bytes");
        assert!(!flat_images.join("abc.png").exists());
    }

    #[test]
    fn a_flat_thumbnail_moves_into_its_bucket_and_a_stray_part_file_is_removed() {
        let dir = tempfile::tempdir().unwrap();
        let library = Library::open_or_create(dir.path()).unwrap();
        let flat_thumbs = library.paths.root.join(THUMBS_DIR);
        fs::write(flat_thumbs.join("abc.jpg"), b"a thumbnail").unwrap();
        fs::write(flat_thumbs.join("abc.jpg.part"), b"half a thumbnail").unwrap();
        drop(library);

        let library = Library::open_or_create(dir.path()).unwrap();

        let bucketed = crate::thumbs::thumbnail_path(&library.paths, "abc");
        assert!(bucketed.is_file());
        assert_eq!(fs::read(&bucketed).unwrap(), b"a thumbnail");
        assert!(!flat_thumbs.join("abc.jpg").exists());
        assert!(!flat_thumbs.join("abc.jpg.part").exists());
    }

    /// A flat file with any extension but `.jpg` is not a thumbnail this app
    /// ever wrote — `thumbnail_path` only ever names one — and must not be
    /// renamed onto a `.jpg` it never had.
    #[test]
    fn a_flat_non_jpg_file_under_thumbs_is_left_where_it_is() {
        let dir = tempfile::tempdir().unwrap();
        let library = Library::open_or_create(dir.path()).unwrap();
        let flat_thumbs = library.paths.root.join(THUMBS_DIR);
        fs::write(flat_thumbs.join("abc.png"), b"not a thumbnail").unwrap();
        drop(library);

        Library::open_or_create(dir.path()).unwrap();

        assert!(flat_thumbs.join("abc.png").is_file());
        assert_eq!(
            fs::read(flat_thumbs.join("abc.png")).unwrap(),
            b"not a thumbnail"
        );
    }

    /// A conflicted copy restored flat, or a library caught mid-migration:
    /// the bucketed file already there must survive, byte for byte, and the
    /// flat one is left rather than destroyed by an overwrite.
    #[test]
    fn a_flat_file_conflicting_with_an_already_bucketed_one_is_left_flat() {
        let dir = tempfile::tempdir().unwrap();
        let library = Library::open_or_create(dir.path()).unwrap();
        let bucketed = library.paths.image_path("abc", "png");
        fs::create_dir_all(bucketed.parent().unwrap()).unwrap();
        fs::write(&bucketed, b"the real file").unwrap();
        let flat = library.paths.root.join(IMAGES_DIR).join("abc.png");
        fs::write(&flat, b"a stale flat copy").unwrap();
        drop(library);

        Library::open_or_create(dir.path()).unwrap();

        assert_eq!(fs::read(&bucketed).unwrap(), b"the real file");
        assert_eq!(fs::read(&flat).unwrap(), b"a stale flat copy");
    }

    /// design D4 did not anticipate a flat file landing exactly on a bucket
    /// name: `create_dir_all("images/ab")` fails outright when `images/ab`
    /// already exists as this stray file, not a directory. That one id's
    /// migration must be the only casualty — the library still opens, and
    /// every other id still migrates.
    #[test]
    fn a_stray_file_blocking_one_bucket_does_not_stop_the_open_or_the_other_files() {
        let dir = tempfile::tempdir().unwrap();
        let library = Library::open_or_create(dir.path()).unwrap();
        let flat_images = library.paths.root.join(IMAGES_DIR);
        // "ab" is exactly the bucket name an id starting with those two
        // characters would need created under `images/`.
        fs::write(flat_images.join("ab"), b"not an image").unwrap();
        fs::write(flat_images.join("abcdef.png"), b"blocked by ab").unwrap();
        fs::write(flat_images.join("xyz999.png"), b"unrelated id").unwrap();
        drop(library);

        let library = Library::open_or_create(dir.path()).unwrap();

        assert!(
            flat_images.join("abcdef.png").is_file(),
            "its bucket could not be created, so it is left flat rather than lost"
        );
        assert!(
            library.paths.image_path("xyz999", "png").is_file(),
            "an unrelated id must still migrate"
        );
    }

    #[test]
    fn opening_a_library_already_bucketed_moves_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let library = Library::open_or_create(dir.path()).unwrap();
        let bytes = png_bytes();
        crate::ingest::store_image(
            &library,
            crate::ingest::IngestInput {
                id: "abc",
                bytes: &bytes,
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
        crate::thumbs::warm_thumbnail(
            &library.paths,
            &crate::ingest::load_record(&library.conn, "abc")
                .unwrap()
                .unwrap(),
        );
        let before = tree(dir.path());
        drop(library);

        Library::open_or_create(dir.path()).unwrap();

        assert_eq!(
            tree(dir.path()),
            before,
            "an already-sharded library is untouched"
        );
    }

    /// Where `id` sits under the two-level layout `one-level-buckets` design
    /// D2 lifts a library out of: `<base>/<a1>/<b2>/<id>.<ext>`, `<a1>` and
    /// `<b2>` split the same way `shard_dirs` splits the one level it keeps.
    fn two_level_path(base: &Path, id: &str, ext: &str) -> PathBuf {
        base.join(&id[..2])
            .join(&id[2..4])
            .join(format!("{id}.{ext}"))
    }

    #[test]
    fn a_two_level_library_lifts_its_files_one_level_on_open() {
        let dir = tempfile::tempdir().unwrap();
        let id = "a1b2c3d4";
        {
            let library = Library::open_or_create(dir.path()).unwrap();
            store(&library, id);
            let record = crate::ingest::load_record(&library.conn, id)
                .unwrap()
                .unwrap();
            crate::thumbs::warm_thumbnail(&library.paths, &record);

            // Simulate a library from before this change: move the image, its
            // sidecar and its thumbnail one level deeper than `open_or_create`
            // just bucketed them, the shape a real pre-existing library was
            // actually found in.
            for (bucketed, two_level) in [
                (
                    library.paths.image_path(id, "png"),
                    two_level_path(&library.paths.images_dir(), id, "png"),
                ),
                (
                    sidecar::path(&library.paths, id),
                    two_level_path(&library.paths.images_dir(), id, "json"),
                ),
                (
                    crate::thumbs::thumbnail_path(&library.paths, id),
                    two_level_path(&library.paths.thumbs_dir(), id, "jpg"),
                ),
            ] {
                fs::create_dir_all(two_level.parent().unwrap()).unwrap();
                fs::rename(&bucketed, &two_level).unwrap();
            }
        }

        let library = Library::open_or_create(dir.path()).unwrap();

        assert!(library.paths.image_path(id, "png").is_file());
        assert!(sidecar::path(&library.paths, id).is_file());
        assert!(crate::thumbs::thumbnail_path(&library.paths, id).is_file());
        assert!(!library.paths.images_dir().join("a1").join("b2").exists());
        assert!(!library.paths.thumbs_dir().join("a1").join("b2").exists());
        let record = crate::ingest::load_record(&library.conn, id)
            .unwrap()
            .unwrap();
        assert_eq!(record.file, format!("images/a1/{id}.png"));

        let before = tree(dir.path());
        Library::open_or_create(dir.path()).unwrap();
        assert_eq!(tree(dir.path()), before, "a second open moves nothing");
    }

    /// A library caught mid-migration, or a conflicted copy restored at the
    /// two-level path: the bucketed file already there must survive, byte for
    /// byte, and the two-level copy is left — `b2` stays — rather than
    /// destroyed by an overwrite.
    #[test]
    fn a_file_already_at_its_destination_is_left_alone() {
        let dir = tempfile::tempdir().unwrap();
        let library = Library::open_or_create(dir.path()).unwrap();
        let id = "abcdxyz9";
        let bucketed = library.paths.image_path(id, "png");
        fs::create_dir_all(bucketed.parent().unwrap()).unwrap();
        fs::write(&bucketed, b"the bucketed file").unwrap();
        let two_level = two_level_path(&library.paths.images_dir(), id, "png");
        fs::create_dir_all(two_level.parent().unwrap()).unwrap();
        fs::write(&two_level, b"a stale two-level copy").unwrap();
        drop(library);

        Library::open_or_create(dir.path()).unwrap();

        assert_eq!(fs::read(&bucketed).unwrap(), b"the bucketed file");
        assert_eq!(fs::read(&two_level).unwrap(), b"a stale two-level copy");
        assert!(
            two_level.parent().unwrap().is_dir(),
            "b2 must stay: the file it holds could not move"
        );
    }

    #[test]
    fn a_stray_part_under_b2_is_removed() {
        let dir = tempfile::tempdir().unwrap();
        let library = Library::open_or_create(dir.path()).unwrap();
        let id = "abcdxyz9";
        let b2 = two_level_path(&library.paths.images_dir(), id, "png")
            .parent()
            .unwrap()
            .to_path_buf();
        fs::create_dir_all(&b2).unwrap();
        fs::write(b2.join(format!("{id}.png.part")), b"half a copy").unwrap();
        drop(library);

        Library::open_or_create(dir.path()).unwrap();

        assert!(
            !b2.exists(),
            "b2 must be removed once emptied of its stray part"
        );
    }

    /// The OS's own droppings must not keep `b2` from emptying: without this,
    /// a `b2` Finder or Explorer has ever shown never empties and is
    /// re-walked on every open.
    #[test]
    fn a_b2_holding_only_os_droppings_is_removed() {
        let dir = tempfile::tempdir().unwrap();
        let library = Library::open_or_create(dir.path()).unwrap();
        let id = "abcdxyz9";
        let b2 = two_level_path(&library.paths.images_dir(), id, "png")
            .parent()
            .unwrap()
            .to_path_buf();
        fs::create_dir_all(&b2).unwrap();
        fs::write(b2.join(".DS_Store"), b"finder metadata").unwrap();
        fs::write(b2.join("Thumbs.db"), b"explorer metadata").unwrap();
        drop(library);

        Library::open_or_create(dir.path()).unwrap();

        assert!(
            !b2.exists(),
            "b2 must be removed once its only contents are the OS's own droppings"
        );
    }

    /// design D2's "a stray directory" case: `b2` holding a subdirectory is
    /// not something `relayout_flat_files` will ever move — it walks files
    /// only — so `b2` is left rather than silently dropped along with what it
    /// holds.
    ///
    /// The other half of `lift_second_level`'s swallow — `b2` itself going
    /// unreadable between this listing and `relayout_flat_files` reading it,
    /// the sync-client-eviction case its call site comments — has no
    /// portable test: `chmod`ing a directory unreadable is unix-only and
    /// root-dependent, and a `b2` that is a file rather than a directory (the
    /// only cross-platform way to break `read_dir`) is caught earlier by the
    /// `file_type().is_dir()` check above and never reaches the call at all.
    #[test]
    fn a_non_empty_b2_is_left() {
        let dir = tempfile::tempdir().unwrap();
        let library = Library::open_or_create(dir.path()).unwrap();
        let id = "abcdxyz9";
        let b2 = two_level_path(&library.paths.images_dir(), id, "png")
            .parent()
            .unwrap()
            .to_path_buf();
        fs::create_dir_all(b2.join("stray")).unwrap();
        drop(library);

        Library::open_or_create(dir.path()).unwrap();

        assert!(b2.is_dir(), "a b2 holding a subdirectory is not empty");
        assert!(b2.join("stray").is_dir());
    }

    fn png_bytes() -> Vec<u8> {
        let image = image::DynamicImage::ImageRgb8(image::RgbImage::new(2, 2));
        let mut out = std::io::Cursor::new(Vec::new());
        image.write_to(&mut out, image::ImageFormat::Png).unwrap();
        out.into_inner()
    }

    /// Every file path under `root`, relative to it and sorted, so two trees
    /// can be compared regardless of `read_dir`'s order.
    fn tree(root: &Path) -> Vec<PathBuf> {
        fn walk(dir: &Path, root: &Path, out: &mut Vec<PathBuf>) {
            for entry in fs::read_dir(dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    walk(&path, root, out);
                } else {
                    out.push(path.strip_prefix(root).unwrap().to_path_buf());
                }
            }
        }
        let mut out = Vec::new();
        walk(root, root, &mut out);
        out.sort();
        out
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

    fn shared(library: Library) -> SharedLibrary {
        std::sync::Arc::new(std::sync::Mutex::new(Some(library)))
    }

    fn store(library: &Library, id: &str) {
        crate::ingest::store_image(
            library,
            crate::ingest::IngestInput {
                id,
                bytes: &png_bytes(),
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

    /// `library-sidecars` design D8: damage met mid-session — a query failing
    /// with a corrupt-class code long after the library opened — is reported
    /// as `LibraryCorrupt` naming the open library's own database, not as the
    /// bare `Db` error `?` produced, which says nothing the user could act on.
    #[test]
    fn a_corrupt_class_failure_inside_a_command_names_the_open_library_as_damaged() {
        let dir = tempfile::tempdir().unwrap();
        let library = Library::open_or_create(dir.path()).unwrap();
        let db_path = library.paths.db_path();
        let shared = shared(library);

        let error = with_library(&shared, |_| -> Result<()> {
            Err(AppError::Db(rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error {
                    code: rusqlite::ErrorCode::DatabaseCorrupt,
                    extended_code: 11,
                },
                Some("database disk image is malformed".to_string()),
            )))
        })
        .unwrap_err();

        assert!(
            matches!(&error, AppError::LibraryCorrupt { path } if path == &db_path),
            "unexpected error: {error:?}"
        );
    }

    /// `library-sidecars` task 2.7: a library whose sidecars were deleted gets
    /// exactly those written again, and a second run writes nothing further.
    #[test]
    fn a_library_whose_sidecars_were_deleted_gets_exactly_those_written_again() {
        let dir = tempfile::tempdir().unwrap();
        let library = Library::open_or_create(dir.path()).unwrap();
        for id in ["a", "b", "c"] {
            store(&library, id);
        }
        let paths = library.paths.clone();
        std::fs::remove_file(sidecar::path(&paths, "a")).unwrap();
        std::fs::remove_file(sidecar::path(&paths, "c")).unwrap();
        let root = paths.root.clone();
        let shared = shared(library);

        let mut ticks = Vec::new();
        backfill_sidecars(&shared, &root, &mut |done, total| ticks.push((done, total))).unwrap();

        for id in ["a", "b", "c"] {
            assert!(sidecar::path(&paths, id).is_file());
        }
        assert_eq!(ticks.last(), Some(&(2, 2)));

        let a_bytes = std::fs::read(sidecar::path(&paths, "a")).unwrap();
        let mut second_ticks = Vec::new();
        backfill_sidecars(&shared, &root, &mut |done, total| {
            second_ticks.push((done, total))
        })
        .unwrap();
        assert_eq!(
            second_ticks,
            Vec::<(i64, i64)>::new(),
            "a library already in step must emit no library:sidecars event at all, \
             not even a (0, 0) tick — the webview shows a tile for any event with total > 0"
        );
        assert_eq!(
            std::fs::read(sidecar::path(&paths, "a")).unwrap(),
            a_bytes,
            "an already-written sidecar must not be rewritten"
        );
    }

    /// A library whose sidecars are all already there emits no
    /// `library:sidecars` event at all — not a `(0, 0)` tick — since the
    /// webview's tile shows for any event with `total > 0` and would
    /// otherwise flash on every single open (constraint from the webview
    /// reviewer on tasks 2.7/2.8).
    #[test]
    fn a_library_already_in_step_emits_no_sidecars_event() {
        let dir = tempfile::tempdir().unwrap();
        let library = Library::open_or_create(dir.path()).unwrap();
        for id in ["a", "b"] {
            store(&library, id);
        }
        let root = library.paths.root.clone();
        let shared = shared(library);

        let mut ticks = Vec::new();
        backfill_sidecars(&shared, &root, &mut |done, total| ticks.push((done, total))).unwrap();

        assert!(
            ticks.is_empty(),
            "nothing was missing, so nothing should have been ticked: {ticks:?}"
        );
    }

    /// `library-sidecars` design D3, D7: `library.json` is written by the same
    /// pass when it is absent.
    #[test]
    fn a_library_json_missing_at_open_is_written_by_the_pass() {
        let dir = tempfile::tempdir().unwrap();
        let library = Library::open_or_create(dir.path()).unwrap();
        store(&library, "a");
        let paths = library.paths.clone();
        assert!(
            !sidecar::library_path(&paths).is_file(),
            "nothing has written library.json yet"
        );
        let root = paths.root.clone();
        let shared = shared(library);

        backfill_sidecars(&shared, &root, &mut |_, _| {}).unwrap();

        assert!(sidecar::library_path(&paths).is_file());
    }

    /// `library-sidecars` task 2.7 / `pending-work` spec "Switching libraries
    /// mid-pass": a pass told to run against a library that has since been
    /// switched stops without writing into the new folder.
    #[test]
    fn a_pass_stops_without_writing_when_the_open_library_has_switched() {
        let dir = tempfile::tempdir().unwrap();
        let library = Library::open_or_create(dir.path()).unwrap();
        store(&library, "a");
        let started_for = library.paths.root.clone();
        let shared = shared(library);

        // The switch this pass must notice: a different library is now open
        // in the same slot, the shape `open_into_state` leaves behind.
        let other_dir = tempfile::tempdir().unwrap();
        let other = Library::open_or_create(other_dir.path()).unwrap();
        *shared.lock().unwrap() = Some(other);

        backfill_sidecars(&shared, &started_for, &mut |_, _| {}).unwrap();

        assert!(
            !sidecar::library_path(&LibraryPaths {
                root: other_dir.path().to_path_buf()
            })
            .is_file(),
            "the pass must not write into the folder that is open now"
        );
    }

    /// `library-sidecars` task 2.7: a 500-row library's pass leaves every
    /// sidecar matching what `sidecar::write_for` would have written.
    #[test]
    fn a_five_hundred_row_librarys_pass_matches_what_write_for_would_write() {
        let dir = tempfile::tempdir().unwrap();
        let library = Library::open_or_create(dir.path()).unwrap();
        let ids: Vec<String> = (0..500).map(|index| format!("id-{index:04}")).collect();
        for id in &ids {
            store(&library, id);
        }
        let paths = library.paths.clone();
        for id in &ids {
            std::fs::remove_file(sidecar::path(&paths, id)).unwrap();
        }
        let root = paths.root.clone();
        let shared = shared(library);

        backfill_sidecars(&shared, &root, &mut |_, _| {}).unwrap();

        // What `sidecar::write_for` would have produced for the same ids and
        // rows, byte for byte — the pass writes through `write_one`, and this
        // is the guarantee that the two never disagree.
        with_library(&shared, |open| {
            for id in &ids {
                let written = std::fs::read(sidecar::path(&open.paths, id)).unwrap();
                let record = crate::ingest::load_record(&open.conn, id).unwrap().unwrap();
                let expected = serde_json::to_vec_pretty(&sidecar::Sidecar::from(&record)).unwrap();
                assert_eq!(
                    written, expected,
                    "{id} does not match what write_for would write"
                );
            }
            Ok(())
        })
        .unwrap();
    }
}
