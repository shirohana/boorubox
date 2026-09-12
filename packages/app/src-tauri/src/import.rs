//! Local file and folder import: fresh ids, a copy into `images/`,
//! `source=local` and metadata read off the file (design D8).

use std::fs::Metadata;
use std::path::{Path, PathBuf};
use std::sync::{Condvar, Mutex};

use crate::db;
use crate::error::AppError;
use crate::ingest::{self, IngestInput};
use crate::library::{SharedLibrary, with_library};
use crate::model::{ImageSource, ImportOutcome, ImportProgress, ImportReport, ImportStatus};
use crate::thumbs;

/// `Running`, `Paused` or `Cancelled` (`import-pause-cancel` design D1). Never
/// goes back from `Cancelled` — a run stops once, and `pause`/`resume` after
/// that would just be racing a thread that is already on its way out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ControlState {
    Running,
    Paused,
    Cancelled,
}

/// One run's pause/resume/cancel handle (`import-pause-cancel` design D1).
///
/// A condvar rather than a polled flag: `checkpoint` must cost nothing while
/// parked and must wake the instant `resume` or `cancel` is called, not at the
/// end of some poll interval. Safe to park on precisely because of where
/// `checkpoint` is called — between items, with the library lock released —
/// so a paused run blocks nothing else (design D1).
pub struct ImportControl {
    state: Mutex<ControlState>,
    woken: Condvar,
}

impl Default for ImportControl {
    fn default() -> Self {
        ImportControl {
            state: Mutex::new(ControlState::Running),
            woken: Condvar::new(),
        }
    }
}

impl ImportControl {
    fn set(&self, state: ControlState) {
        *self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = state;
    }

    /// Holds a running run at the next `checkpoint`. A no-op once the run has
    /// already been cancelled — pausing a run that is on its way out must not
    /// resurrect it as paused.
    pub fn pause(&self) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if *state == ControlState::Running {
            *state = ControlState::Paused;
        }
    }

    /// Wakes a run parked at `checkpoint`. A no-op with nothing paused.
    pub fn resume(&self) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if *state == ControlState::Paused {
            *state = ControlState::Running;
            self.woken.notify_all();
        }
    }

    /// Ends the run at its next `checkpoint`, waking it first if it is parked
    /// there paused.
    pub fn cancel(&self) {
        self.set(ControlState::Cancelled);
        self.woken.notify_all();
    }

    /// Called between two items (design D2): blocks while the run is paused,
    /// and answers whether the run should carry on to the next item.
    pub fn checkpoint(&self) -> bool {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        loop {
            match *state {
                ControlState::Running => return true,
                ControlState::Cancelled => return false,
                ControlState::Paused => {
                    state = self
                        .woken
                        .wait(state)
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                }
            }
        }
    }
}

/// Import every decodable image under `paths`, recursing into folders, calling
/// `on_progress` as it goes so the UI can show a running count.
///
/// Blocking by design: the caller puts it on a thread and turns `on_progress`
/// into the `import:progress` event (design D12).
///
/// Takes the shared library and locks it one file at a time, never across the
/// loop: a search or a thumbnail asked for while a folder of a few hundred
/// files goes in has to be answered between two of them, or the window freezes
/// until the last file is in (design D13). The walk and `on_progress` run with
/// the library released.
///
/// The `Result` is the command surface's, not a way out of the loop — every
/// per-item failure is an outcome in the report, so one bad file never ends the
/// run. Do not reach for `?` inside the loop.
pub fn import_paths(
    library: &SharedLibrary,
    paths: &[PathBuf],
    control: &ImportControl,
    on_progress: &mut dyn FnMut(ImportProgress),
) -> crate::error::Result<ImportReport> {
    refuse_if_closed(library)?;

    // The walk runs to completion first because `total` has to be right in the
    // very first progress event: a bar that grows its own denominator reads as
    // an import that keeps finding more work. Enumerating a folder is cheap
    // next to decoding every image in it.
    let candidates = walk(paths);
    let total = candidates.len() as u32;

    let mut report = ImportReport::default();
    on_progress(progress(&report, total));
    for candidate in candidates {
        let outcome = match candidate {
            Candidate::File(path) => import_file(library, &path),
            Candidate::Decided(outcome) => outcome,
        };
        count(&mut report, outcome);
        on_progress(progress(&report, total));
        if is_last_item(&report, total) {
            break;
        }
        // Between items, with the library released (design D13): the one
        // point a paused or cancelled run may stop without freezing anything
        // else behind the library lock (`import-pause-cancel` design D2).
        if !control.checkpoint() {
            report.cancelled = true;
            break;
        }
    }
    Ok(report)
}

/// A closed library is one refusal the webview can show, not a report naming
/// every dropped file as failed. Asked before either importer's walk so
/// nothing is counted for a run that cannot store anything.
pub(crate) fn refuse_if_closed(library: &SharedLibrary) -> crate::error::Result<()> {
    with_library(library, |_| Ok(()))
}

/// One thing the walk found: a file to try, or an entry whose fate the walk
/// already knows. Both cost one report item, so `total` counts both.
enum Candidate {
    File(PathBuf),
    Decided(ImportOutcome),
}

fn walk(paths: &[PathBuf]) -> Vec<Candidate> {
    let mut found = Vec::new();
    for path in paths {
        collect(path, true, &mut found);
    }
    found
}

/// `named` marks a path the user handed us directly. Those are followed
/// whatever they are — pointing at a link to a folder is a way of asking for
/// that folder. Links found *inside* a folder are not followed: one pointing
/// back at an ancestor makes the walk unbounded, and the check that would rule
/// that out costs more than the feature is worth. Not following them is also
/// what bounds the recursion below.
fn collect(path: &Path, named: bool, found: &mut Vec<Candidate>) {
    if !named && is_symlink(path) {
        found.push(Candidate::Decided(skipped(
            path,
            "symbolic link, not followed",
        )));
        return;
    }
    match std::fs::metadata(path) {
        Ok(metadata) if metadata.is_dir() => collect_dir(path, found),
        Ok(metadata) if metadata.is_file() => found.push(Candidate::File(path.to_path_buf())),
        Ok(_) => found.push(Candidate::Decided(skipped(path, "not a regular file"))),
        Err(error) => found.push(Candidate::Decided(failed(path, error.to_string()))),
    }
}

fn collect_dir(dir: &Path, found: &mut Vec<Candidate>) {
    match read_sorted(dir) {
        // Sorted, so a drop of a hundred files reports them in the order the
        // user sees in their file manager rather than in inode order.
        Ok(entries) => {
            for entry in entries {
                collect(&entry, false, found);
            }
        }
        Err(error) => found.push(Candidate::Decided(failed(dir, error.to_string()))),
    }
}

fn read_sorted(dir: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut entries = std::fs::read_dir(dir)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<std::io::Result<Vec<PathBuf>>>()?;
    entries.sort();
    Ok(entries)
}

fn is_symlink(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok_and(|metadata| metadata.file_type().is_symlink())
}

/// Read the file and hand it to `ingest::store_image` with a fresh id. Phase 1
/// does not deduplicate, so the same file imported twice becomes two images
/// (spec `local-file-import`). The original is only ever read.
fn import_file(library: &SharedLibrary, path: &Path) -> ImportOutcome {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) => return failed(path, error.to_string()),
    };
    let file_modified_at = match std::fs::metadata(path) {
        Ok(metadata) => file_modified_at(&metadata),
        Err(error) => return failed(path, error.to_string()),
    };
    let id = uuid::Uuid::new_v4().to_string();
    let title = file_name(path);

    // The read, the decode-and-write and the thumbnail are three separate costs;
    // only the middle one needs the library, so only it is inside the lock.
    let stored = with_library(library, |library| {
        let ingested = ingest::store_image(
            library,
            IngestInput {
                id: &id,
                bytes: &bytes,
                source: ImageSource::Local,
                source_ref: None,
                image_url: None,
                page_url: None,
                page_title: Some(&title),
                adapter: None,
                rating: None,
                tags: &[],
                // The moment this import ran, not the file's own history
                // (design D11, `browse-polish`): capture time answers "when did
                // this arrive", and for a local import that is now.
                captured_at: db::now_ms(),
                file_modified_at,
                deleted_at: None,
            },
        )?;
        Ok((library.paths.clone(), ingested))
    });

    match stored {
        Ok((paths, ingested)) => {
            thumbs::warm_thumbnail(&paths, ingested.record());
            imported(path, ingested.record().id.clone())
        }
        // Undecodable is what the file is, not something that went wrong: the
        // text file in a dropped folder is skipped and named in the report,
        // never counted as a failure. `failed` is for a real attempt the
        // machine stopped — unreadable bytes, a full disk, a database error.
        Err(AppError::Decode(error)) => skipped(path, error.to_string()),
        Err(error) => failed(path, error.to_string()),
    }
}

/// The file's own modification time in epoch milliseconds, or `None` when the
/// platform cannot report one (design D11, `browse-polish`). No fallback to
/// now: unlike `captured_at`, this column is nullable, so a missing fact is
/// recorded as absent rather than invented.
fn file_modified_at(metadata: &Metadata) -> Option<i64> {
    let modified = metadata.modified().ok()?;
    Some(match modified.duration_since(std::time::UNIX_EPOCH) {
        Ok(since) => since.as_millis() as i64,
        Err(before) => -(before.duration().as_millis() as i64),
    })
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .unwrap_or(path.as_os_str())
        .to_string_lossy()
        .into_owned()
}

fn imported(path: &Path, id: String) -> ImportOutcome {
    ImportOutcome::imported(display(path), id)
}

fn skipped(path: &Path, reason: impl Into<String>) -> ImportOutcome {
    ImportOutcome::skipped(display(path), None, reason)
}

fn failed(path: &Path, reason: impl Into<String>) -> ImportOutcome {
    ImportOutcome::failed(display(path), None, reason)
}

fn display(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

/// Fold one outcome into the running report: keeps its counts and its
/// `items` list in step (design D6).
pub(crate) fn count(report: &mut ImportReport, outcome: ImportOutcome) {
    match outcome.status {
        ImportStatus::Imported => report.imported += 1,
        ImportStatus::Skipped => report.skipped += 1,
        ImportStatus::Failed => report.failed += 1,
    }
    report.items.push(outcome);
}

/// The `import:progress` payload for a report so far.
pub(crate) fn progress(report: &ImportReport, total: u32) -> ImportProgress {
    ImportProgress {
        done: report.items.len() as u32,
        total,
        imported: report.imported,
        skipped: report.skipped,
        failed: report.failed,
    }
}

/// Whether every item `total` promised has now been counted — the point past
/// which there is no next item for a checkpoint to guard. Shared by
/// `import_paths` and `bundle::import_bundle`, the same one place both call
/// their checkpoint (design D2): a pause or cancel landing after the last
/// item must not park, or mark `cancelled`, a run that actually finished —
/// `report.cancelled` is read by the UI as "there is more to resume."
pub(crate) fn is_last_item(report: &ImportReport, total: u32) -> bool {
    report.items.len() as u32 == total
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, mpsc};
    use std::time::Duration;

    use super::*;
    use crate::library::Library;
    use crate::model::ImageRecord;

    /// Long enough that a machine under load does not fail the test, short
    /// enough that a run that really is holding the library ends the suite
    /// instead of hanging it.
    const A_LOCK_IS_NOT_COMING: Duration = Duration::from_secs(5);

    fn library() -> (tempfile::TempDir, SharedLibrary) {
        let dir = tempfile::tempdir().unwrap();
        let library = Library::open_or_create(dir.path()).unwrap();
        (
            dir,
            SharedLibrary::new(std::sync::Mutex::new(Some(library))),
        )
    }

    fn image_count(library: &SharedLibrary) -> i64 {
        with_library(library, |library| library.image_count()).unwrap()
    }

    fn record(library: &SharedLibrary, id: &str) -> Option<ImageRecord> {
        with_library(library, |library| ingest::load_record(&library.conn, id)).unwrap()
    }

    fn thumbnail_of(library: &SharedLibrary, id: &str) -> PathBuf {
        with_library(library, |library| {
            Ok(thumbs::thumbnail_path(&library.paths, id))
        })
        .unwrap()
    }

    fn write_png(path: &Path, width: u32, height: u32) {
        image::RgbImage::new(width, height).save(path).unwrap();
    }

    fn png_bytes(width: u32, height: u32) -> Vec<u8> {
        let image = image::DynamicImage::ImageRgb8(image::RgbImage::new(width, height));
        let mut out = std::io::Cursor::new(Vec::new());
        image.write_to(&mut out, image::ImageFormat::Png).unwrap();
        out.into_inner()
    }

    /// The spec's mixed drop: two loose images, plus a folder holding three
    /// images and one text file.
    fn mixed_drop(root: &Path) -> Vec<PathBuf> {
        write_png(&root.join("a.png"), 8, 8);
        write_png(&root.join("b.png"), 8, 8);
        let nested = root.join("nested");
        std::fs::create_dir(&nested).unwrap();
        for name in ["c.png", "d.png", "e.png"] {
            write_png(&nested.join(name), 8, 8);
        }
        std::fs::write(nested.join("notes.txt"), b"not an image").unwrap();
        vec![root.join("a.png"), root.join("b.png"), nested]
    }

    fn run(library: &SharedLibrary, paths: &[PathBuf]) -> (ImportReport, Vec<ImportProgress>) {
        let control = ImportControl::default();
        let mut seen = Vec::new();
        let report = import_paths(library, paths, &control, &mut |progress| {
            seen.push(progress)
        })
        .unwrap();
        (report, seen)
    }

    fn items_with(report: &ImportReport, status: ImportStatus) -> Vec<&ImportOutcome> {
        report
            .items
            .iter()
            .filter(|item| item.status == status)
            .collect()
    }

    #[test]
    fn a_mixed_drop_imports_the_images_and_names_the_skipped_file() {
        let source = tempfile::tempdir().unwrap();
        let paths = mixed_drop(source.path());
        let (_dir, library) = library();

        let (report, _) = run(&library, &paths);

        assert_eq!(report.imported, 5, "{:?}", report.items);
        assert_eq!(report.skipped, 1);
        assert_eq!(report.failed, 0);
        let skipped = items_with(&report, ImportStatus::Skipped);
        assert_eq!(skipped.len(), 1);
        assert!(
            skipped[0].path.ends_with("notes.txt"),
            "the report must name the skipped file, got {:?}",
            skipped[0]
        );
        assert!(skipped[0].reason.is_some());
        assert_eq!(image_count(&library), 5);
    }

    #[test]
    fn every_imported_item_carries_its_new_id() {
        let source = tempfile::tempdir().unwrap();
        let paths = mixed_drop(source.path());
        let (_dir, library) = library();

        let (report, _) = run(&library, &paths);

        for item in items_with(&report, ImportStatus::Imported) {
            let id = item.id.as_deref().expect("imported item without an id");
            assert!(record(&library, id).is_some());
        }
    }

    #[test]
    fn the_same_file_twice_becomes_two_images() {
        let source = tempfile::tempdir().unwrap();
        let file = source.path().join("cat.png");
        write_png(&file, 8, 8);
        let (_dir, library) = library();

        let (first, _) = run(&library, std::slice::from_ref(&file));
        let (second, _) = run(&library, &[file]);

        assert_eq!((first.imported, second.imported), (1, 1));
        assert_ne!(first.items[0].id, second.items[0].id);
        assert_eq!(image_count(&library), 2);
    }

    #[test]
    fn the_original_is_untouched() {
        let source = tempfile::tempdir().unwrap();
        let file = source.path().join("cat.png");
        write_png(&file, 8, 8);
        let before = std::fs::read(&file).unwrap();
        let (_dir, library) = library();

        run(&library, std::slice::from_ref(&file));

        assert!(
            file.is_file(),
            "import must not move or delete the original"
        );
        assert_eq!(std::fs::read(&file).unwrap(), before);
    }

    #[test]
    fn metadata_comes_from_the_file() {
        let source = tempfile::tempdir().unwrap();
        let file = source.path().join("cat.png");
        write_png(&file, 40, 20);
        let mtime = file_modified_at(&std::fs::metadata(&file).unwrap());
        let (_dir, library) = library();

        let before = db::now_ms();
        let (report, _) = run(&library, &[file]);
        let after = db::now_ms();

        let id = report.items[0].id.as_deref().unwrap();
        let record = record(&library, id).unwrap();
        assert_eq!(record.page_title.as_deref(), Some("cat.png"));
        assert!(
            record.captured_at >= before && record.captured_at <= after,
            "capture time must be the import time, got {} outside [{before}, {after}]",
            record.captured_at
        );
        assert_eq!(
            record.file_modified_at, mtime,
            "modification time must be the file's own, apart from capture time"
        );
        assert_eq!(record.source, ImageSource::Local);
        assert_eq!((record.width, record.height), (40, 20));
        assert_eq!(record.mime, "image/png");
    }

    /// Spec `local-file-import` "A freshly imported file is at the front":
    /// capture time is the import moment (design D11), so an ancient file
    /// still lands ahead of an old capture in the default sort.
    #[test]
    fn a_file_years_old_still_sorts_to_the_front_of_a_newest_capture_first_search() {
        let source = tempfile::tempdir().unwrap();
        let old_file = source.path().join("old.png");
        write_png(&old_file, 8, 8);
        let years_ago =
            std::time::SystemTime::now() - std::time::Duration::from_secs(60 * 60 * 24 * 365 * 2);
        std::fs::File::open(&old_file)
            .unwrap()
            .set_modified(years_ago)
            .unwrap();

        let (_dir, library) = library();
        // Already in the library with a recent capture time — the point is
        // that the import just below outranks it despite its ancient mtime.
        with_library(&library, |lib| {
            ingest::store_image(
                lib,
                IngestInput {
                    id: "already-here",
                    bytes: &png_bytes(4, 4),
                    source: ImageSource::Local,
                    source_ref: None,
                    image_url: None,
                    page_url: None,
                    page_title: Some("already here"),
                    adapter: None,
                    rating: None,
                    tags: &[],
                    captured_at: db::now_ms() - 1,
                    file_modified_at: None,
                    deleted_at: None,
                },
            )
            .map(|_| ())
        })
        .unwrap();

        let (report, _) = run(&library, &[old_file]);
        let new_id = report.items[0].id.clone().unwrap();

        let req = crate::model::SearchRequest {
            query: crate::model::ParsedTagSearch::default(),
            text: String::new(),
            view: crate::model::SearchView::Library,
            sort: crate::model::Sort::default(),
            group: crate::model::GroupBy::default(),
            limit: 10,
            offset: 0,
        };
        let result = with_library(&library, |lib| crate::query::search(&lib.conn, &req)).unwrap();

        assert_eq!(
            result.images[0].id, new_id,
            "a fresh import must sort ahead of an old capture despite an ancient file mtime"
        );
    }

    #[test]
    fn progress_runs_to_the_report_it_returns() {
        let source = tempfile::tempdir().unwrap();
        let paths = mixed_drop(source.path());
        let (_dir, library) = library();

        let (report, seen) = run(&library, &paths);

        let total = report.items.len() as u32;
        assert_eq!(seen.len() as u32, total + 1, "one start plus one per item");
        for (step, progress) in seen.iter().enumerate() {
            assert_eq!(progress.done, step as u32);
            assert_eq!(progress.total, total);
        }
        let last = seen.last().unwrap();
        assert_eq!(
            (last.imported, last.skipped, last.failed),
            (report.imported, report.skipped, report.failed)
        );
    }

    /// The guarantee used to sit inside `ingest::store_image`; it moved out to
    /// keep the encode off the library lock (design D13), so it is pinned here,
    /// where it now happens.
    #[test]
    fn an_imported_image_is_thumbnailed_without_being_asked() {
        let source = tempfile::tempdir().unwrap();
        let file = source.path().join("cat.png");
        write_png(&file, 800, 600);
        let (_dir, library) = library();

        let (report, _) = run(&library, &[file]);

        let id = report.items[0].id.as_deref().unwrap();
        assert!(thumbnail_of(&library, id).is_file());
    }

    /// Design D13: a scroll that needs a thumbnail must be served between two
    /// files, not after the last one. The run is parked in its progress
    /// callback — which is outside the lock — and another thread has to be able
    /// to read the library while it waits there.
    #[test]
    fn the_library_is_free_between_two_files() {
        let source = tempfile::tempdir().unwrap();
        for name in ["a.png", "b.png", "c.png"] {
            write_png(&source.path().join(name), 8, 8);
        }
        let (_dir, library) = library();

        let (parked, is_parked) = mpsc::channel();
        let (release, go_on) = mpsc::channel();
        let running = library.clone();
        let paths = vec![source.path().to_path_buf()];
        let control = ImportControl::default();
        let run = std::thread::spawn(move || {
            import_paths(&running, &paths, &control, &mut |progress| {
                if progress.done == 1 {
                    parked.send(()).unwrap();
                    go_on.recv().unwrap();
                }
            })
        });
        is_parked.recv_timeout(A_LOCK_IS_NOT_COMING).unwrap();

        let (answered, answer) = mpsc::channel();
        let reader = library.clone();
        std::thread::spawn(move || {
            let _ = answered.send(image_count(&reader));
        });

        let counted = answer
            .recv_timeout(A_LOCK_IS_NOT_COMING)
            .expect("the run held the library across its loop");
        assert_eq!(counted, 1, "the first file is in and the rest are not");
        release.send(()).unwrap();
        assert_eq!(run.join().unwrap().unwrap().imported, 3);
    }

    #[test]
    fn a_run_with_no_library_open_is_one_refusal_rather_than_a_report_of_failures() {
        let source = tempfile::tempdir().unwrap();
        write_png(&source.path().join("a.png"), 8, 8);
        let closed = SharedLibrary::default();

        let control = ImportControl::default();
        let mut ticks = 0;
        let error = import_paths(
            &closed,
            &[source.path().to_path_buf()],
            &control,
            &mut |_| ticks += 1,
        )
        .unwrap_err();

        assert!(matches!(error, AppError::NoLibrary), "{error:?}");
        assert_eq!(ticks, 0, "nothing is counted for a run that cannot store");
    }

    #[test]
    fn a_folder_with_no_images_reports_nothing_rather_than_failing() {
        let source = tempfile::tempdir().unwrap();
        let (_dir, library) = library();

        let (report, _) = run(&library, &[source.path().to_path_buf()]);

        assert_eq!(report, ImportReport::default());
        assert_eq!(image_count(&library), 0);
    }

    #[test]
    fn an_unreadable_path_fails_that_item_only() {
        let source = tempfile::tempdir().unwrap();
        write_png(&source.path().join("a.png"), 8, 8);
        let (_dir, library) = library();

        let paths = vec![source.path().join("gone.png"), source.path().join("a.png")];
        let (report, _) = run(&library, &paths);

        assert_eq!((report.imported, report.failed), (1, 1));
        assert!(
            items_with(&report, ImportStatus::Failed)[0]
                .path
                .ends_with("gone.png")
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_link_inside_a_folder_is_named_and_not_followed() {
        let source = tempfile::tempdir().unwrap();
        write_png(&source.path().join("a.png"), 8, 8);
        // A link back to the folder holding it: following links would walk here
        // forever.
        std::os::unix::fs::symlink(source.path(), source.path().join("loop")).unwrap();
        let (_dir, library) = library();

        let (report, _) = run(&library, &[source.path().to_path_buf()]);

        assert_eq!((report.imported, report.skipped), (1, 1));
        assert!(
            items_with(&report, ImportStatus::Skipped)[0]
                .path
                .ends_with("loop")
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_link_the_user_named_is_followed() {
        let source = tempfile::tempdir().unwrap();
        let images = source.path().join("images");
        std::fs::create_dir(&images).unwrap();
        write_png(&images.join("a.png"), 8, 8);
        let link = source.path().join("link-to-images");
        std::os::unix::fs::symlink(&images, &link).unwrap();
        let (_dir, library) = library();

        let (report, _) = run(&library, &[link]);

        assert_eq!(report.imported, 1);
    }

    /// `import-pause-cancel` task 1.1.
    #[test]
    fn a_cancelled_control_stops_the_very_next_checkpoint() {
        let control = ImportControl::default();
        control.cancel();

        assert!(
            !control.checkpoint(),
            "a control cancelled before the first check must not carry on"
        );
    }

    /// `import-pause-cancel` task 1.1: `resume` must wake a thread genuinely
    /// parked in `checkpoint`, not merely return quickly because the state
    /// happened to change first — proven with a channel a `sleep` cannot
    /// stand in for.
    #[test]
    fn resume_wakes_a_thread_parked_at_the_checkpoint() {
        let control = Arc::new(ImportControl::default());
        // Paused before the thread exists: the thread's own `checkpoint` call
        // is guaranteed to observe `Paused`, never a race against `pause`.
        control.pause();

        let parked = control.clone();
        let (done, carried_on) = mpsc::channel();
        std::thread::spawn(move || {
            done.send(parked.checkpoint()).unwrap();
        });

        control.resume();

        assert!(
            carried_on.recv_timeout(A_LOCK_IS_NOT_COMING).unwrap(),
            "resume must wake the parked checkpoint and tell it to carry on"
        );
    }

    /// `import-pause-cancel` task 1.1: a cancel delivered to a parked thread
    /// wakes it too, and tells it to stop rather than carry on.
    #[test]
    fn a_cancel_delivered_while_parked_wakes_it_and_stops_it() {
        let control = Arc::new(ImportControl::default());
        control.pause();

        let parked = control.clone();
        let (done, carried_on) = mpsc::channel();
        std::thread::spawn(move || {
            done.send(parked.checkpoint()).unwrap();
        });

        control.cancel();

        assert!(
            !carried_on.recv_timeout(A_LOCK_IS_NOT_COMING).unwrap(),
            "a parked checkpoint woken by cancel must say to stop"
        );
    }

    /// `import-pause-cancel` task 1.2: the item in flight when Cancel is
    /// pressed finishes and is counted; nothing after it is.
    #[test]
    fn a_run_cancelled_after_the_first_item_reports_and_keeps_exactly_one() {
        let source = tempfile::tempdir().unwrap();
        for name in ["a.png", "b.png", "c.png"] {
            write_png(&source.path().join(name), 8, 8);
        }
        let (_dir, library) = library();
        let control = ImportControl::default();

        let report = import_paths(
            &library,
            &[source.path().to_path_buf()],
            &control,
            &mut |progress| {
                if progress.done == 1 {
                    control.cancel();
                }
            },
        )
        .unwrap();

        assert_eq!(report.items.len(), 1, "{:?}", report.items);
        assert_eq!(report.imported, 1);
        assert!(report.cancelled);
        assert_eq!(
            image_count(&library),
            1,
            "the files after the stop are neither imported nor reported"
        );
    }

    /// `import-pause-cancel` should-fix: the checkpoint only runs when there
    /// is a next item to check before. A cancel landing the instant the last
    /// item finishes must not mark a run that actually completed as
    /// `cancelled` — the UI reads that field as "there is more to resume."
    #[test]
    fn cancelling_exactly_when_the_last_item_finishes_does_not_mark_the_run_cancelled() {
        let source = tempfile::tempdir().unwrap();
        write_png(&source.path().join("a.png"), 8, 8);
        let (_dir, library) = library();
        let control = ImportControl::default();

        let report = import_paths(
            &library,
            &[source.path().to_path_buf()],
            &control,
            &mut |progress| {
                if progress.done == progress.total {
                    control.cancel();
                }
            },
        )
        .unwrap();

        assert_eq!(report.imported, 1);
        assert!(
            !report.cancelled,
            "a cancel landing after the last item must not mark a finished run cancelled"
        );
    }
}
