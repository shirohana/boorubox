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
    work(guard.as_ref())
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
        library.relayout_to_buckets()?;
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

    /// Move files sitting flat under `images/` or `.thumbs/` — the shape a
    /// library made before the sharded layout (design D1) left them in — into
    /// their buckets (design D4). Idempotent: a library already sharded costs
    /// one `read_dir` of each near-empty top level, since `read_dir` never
    /// descends into a bucket.
    fn relayout_to_buckets(&self) -> Result<()> {
        let paths = &self.paths;
        relayout_flat_files(&paths.images_dir(), |stem, ext| {
            Some(paths.image_path(stem, ext))
        })?;
        relayout_flat_files(&paths.thumbs_dir(), |stem, ext| {
            // A flat file under `.thumbs/` with any other extension is not one
            // of ours — `thumbnail_path` only ever names a `.jpg` — and must
            // not be renamed onto a `.jpg` it never had.
            (ext == "jpg").then(|| crate::thumbs::thumbnail_path(paths, stem))
        })
    }
}

/// Move every regular file directly under `dir` to where `dest_for` (its stem
/// and extension, `None` if the entry is not this directory's to bucket) says
/// it belongs, creating the destination's bucket first (design D3). A stray
/// `.part` — a thumbnail encode that crashed mid-write — is removed rather
/// than moved; an inbox write's `.part` lives under `inbox/` and never reaches
/// here, but the check costs nothing to keep on both directories this walks.
///
/// Every failure here — the entry, the bucket `mkdir`, the rename — is
/// swallowed rather than propagated: a Dropbox "conflicted copy", an iCloud
/// placeholder, or (design D4 did not anticipate this) a stray file that
/// happens to be named exactly one id's two-character bucket prefix, blocking
/// `create_dir_all` for every id sharding into it, must cost that one file its
/// migration, not the library its ability to open. A file already sitting at
/// its destination — a library caught mid-migration, or a conflicted copy
/// restored flat — is left alone rather than overwritten.
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
        if dest.exists() {
            continue;
        }
        if let Some(bucket) = dest.parent()
            && fs::create_dir_all(bucket).is_err()
        {
            continue;
        }
        let _ = fs::rename(&path, &dest);
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

    /// `images/<a1>/<b2>/<id>.<ext>` (design D1, D2): the path `ImageRecord.file`
    /// carries, so it is a `/`-joined string, not a `PathBuf` — the webview
    /// reads this same value with no path module of its own (design D2).
    /// Filesystem paths (`image_path`, `thumbnail_path`) never build on this;
    /// both instead push `shard_dirs`' components directly, the one thing the
    /// two representations share.
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

/// The bucket directories `id` shards into (design D1, Danbooru's shape):
/// the first two characters, then the next two. An id shorter than four
/// characters uses what it has, omitting a level it cannot fill rather than
/// emitting one with an empty name (`a` -> `["a"]`, `abc` -> `["ab", "c"]`).
/// Shared by `LibraryPaths::relative_image_path` and `thumbs::thumbnail_path`,
/// the one definition of the bucket rule.
///
/// FIXME: this never validates `id` — a `/` or `..` inside one would escape
/// the bucket it should be confined to, and nothing stops an id built from
/// characters no filesystem accepts in a name. Slicing by character (below)
/// only keeps this from panicking on one; it validates nothing. The right
/// place for that check is the capture door (`ingest::store_image`), before an
/// id ever reaches a path; not built yet (design D1's non-goal).
pub(crate) fn shard_dirs(id: &str) -> Vec<&str> {
    // Byte indices of character boundaries, never byte offsets: an id is
    // arbitrary and unvalidated (the `FIXME` above), and `id[..2]` panics the
    // instant one non-ASCII character puts a byte boundary mid-character —
    // which the relayout in `Library::relayout_to_buckets` would then hit on
    // every stem in the directory, not just the one bad id.
    let a1_end = char_boundary(id, 2);
    let b2_end = char_boundary(id, 4);
    [&id[..a1_end], &id[a1_end..b2_end]]
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
            library
                .paths
                .images_dir()
                .join("ab")
                .join("c")
                .join("abc.png"),
            "never built by joining a `/`-string onto root (design D2's mixed-separator risk)"
        );
    }

    #[test]
    fn a_uuid_id_shards_two_levels_deep() {
        assert_eq!(
            LibraryPaths::relative_image_path("a1b2c3d4-e5f6-7890-abcd-ef1234567890", "jpg"),
            "images/a1/b2/a1b2c3d4-e5f6-7890-abcd-ef1234567890.jpg"
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
    fn a_three_character_id_shards_a_two_and_a_one_character_bucket() {
        assert_eq!(
            LibraryPaths::relative_image_path("abc", "png"),
            "images/ab/c/abc.png"
        );
    }

    /// A byte-index slice (`id[..2]`) panics the moment a two-character
    /// prefix lands mid-character; design D1 says "characters", and the id
    /// arrives from the capture listener with nothing checking its charset.
    #[test]
    fn a_non_ascii_id_shards_by_character_not_byte() {
        assert_eq!(
            LibraryPaths::relative_image_path("日本語テスト", "png"),
            "images/日本/語テ/日本語テスト.png"
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
        assert_eq!(record.file, "images/ab/c/abc.png");
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
}
