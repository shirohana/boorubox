//! Local file and folder import: fresh ids, a copy into `images/`,
//! `source=local` and metadata read off the file (design D8).

use std::fs::Metadata;
use std::path::{Path, PathBuf};

use crate::db;
use crate::error::AppError;
use crate::ingest::{self, IngestInput};
use crate::library::Library;
use crate::model::{ImageSource, ImportOutcome, ImportProgress, ImportReport, ImportStatus};

/// Import every decodable image under `paths`, recursing into folders, calling
/// `on_progress` as it goes so the UI can show a running count.
///
/// Blocking by design: the caller puts it on a thread and turns `on_progress`
/// into the `import:progress` event (design D12).
///
/// The `Result` is the command surface's, not a way out of the loop — every
/// per-item failure is an outcome in the report, so one bad file never ends the
/// run. Do not reach for `?` inside the loop.
pub fn import_paths(
    library: &Library,
    paths: &[PathBuf],
    on_progress: &mut dyn FnMut(ImportProgress),
) -> crate::error::Result<ImportReport> {
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
    }
    Ok(report)
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
fn import_file(library: &Library, path: &Path) -> ImportOutcome {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) => return failed(path, error.to_string()),
    };
    let captured_at = match std::fs::metadata(path) {
        Ok(metadata) => captured_at(&metadata),
        Err(error) => return failed(path, error.to_string()),
    };
    let id = uuid::Uuid::new_v4().to_string();
    let title = file_name(path);

    let stored = ingest::store_image(
        library,
        IngestInput {
            id: &id,
            bytes: &bytes,
            source: ImageSource::Local,
            source_ref: None,
            image_url: None,
            page_url: None,
            page_title: Some(&title),
            rating: None,
            tags: &[],
            captured_at,
        },
    );

    match stored {
        Ok(ingested) => imported(path, ingested.record().id.clone()),
        // Undecodable is what the file is, not something that went wrong: the
        // text file in a dropped folder is skipped and named in the report,
        // never counted as a failure. `failed` is for a real attempt the
        // machine stopped — unreadable bytes, a full disk, a database error.
        Err(AppError::Decode(error)) => skipped(path, error.to_string()),
        Err(error) => failed(path, error.to_string()),
    }
}

/// The file's modification time in epoch milliseconds. A platform that cannot
/// report one falls back to now: an import is worth more than an exact capture
/// time, and the row would otherwise claim 1970.
fn captured_at(metadata: &Metadata) -> i64 {
    let Ok(modified) = metadata.modified() else {
        return db::now_ms();
    };
    match modified.duration_since(std::time::UNIX_EPOCH) {
        Ok(since) => since.as_millis() as i64,
        Err(before) => -(before.duration().as_millis() as i64),
    }
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .unwrap_or(path.as_os_str())
        .to_string_lossy()
        .into_owned()
}

fn imported(path: &Path, id: String) -> ImportOutcome {
    ImportOutcome {
        path: display(path),
        status: ImportStatus::Imported,
        id: Some(id),
        reason: None,
    }
}

fn skipped(path: &Path, reason: impl Into<String>) -> ImportOutcome {
    ImportOutcome {
        path: display(path),
        status: ImportStatus::Skipped,
        id: None,
        reason: Some(reason.into()),
    }
}

fn failed(path: &Path, reason: impl Into<String>) -> ImportOutcome {
    ImportOutcome {
        path: display(path),
        status: ImportStatus::Failed,
        id: None,
        reason: Some(reason.into()),
    }
}

fn display(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn count(report: &mut ImportReport, outcome: ImportOutcome) {
    match outcome.status {
        ImportStatus::Imported => report.imported += 1,
        ImportStatus::Skipped => report.skipped += 1,
        ImportStatus::Failed => report.failed += 1,
    }
    report.items.push(outcome);
}

fn progress(report: &ImportReport, total: u32) -> ImportProgress {
    ImportProgress {
        done: report.items.len() as u32,
        total,
        imported: report.imported,
        skipped: report.skipped,
        failed: report.failed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn library() -> (tempfile::TempDir, Library) {
        let dir = tempfile::tempdir().unwrap();
        let library = Library::open_or_create(dir.path()).unwrap();
        (dir, library)
    }

    fn write_png(path: &Path, width: u32, height: u32) {
        image::RgbImage::new(width, height).save(path).unwrap();
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

    fn run(library: &Library, paths: &[PathBuf]) -> (ImportReport, Vec<ImportProgress>) {
        let mut seen = Vec::new();
        let report = import_paths(library, paths, &mut |progress| seen.push(progress)).unwrap();
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
        assert_eq!(library.image_count().unwrap(), 5);
    }

    #[test]
    fn every_imported_item_carries_its_new_id() {
        let source = tempfile::tempdir().unwrap();
        let paths = mixed_drop(source.path());
        let (_dir, library) = library();

        let (report, _) = run(&library, &paths);

        for item in items_with(&report, ImportStatus::Imported) {
            let id = item.id.as_deref().expect("imported item without an id");
            assert!(ingest::load_record(&library.conn, id).unwrap().is_some());
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
        assert_eq!(library.image_count().unwrap(), 2);
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
        let mtime = captured_at(&std::fs::metadata(&file).unwrap());
        let (_dir, library) = library();

        let (report, _) = run(&library, &[file]);

        let id = report.items[0].id.as_deref().unwrap();
        let record = ingest::load_record(&library.conn, id).unwrap().unwrap();
        assert_eq!(record.page_title.as_deref(), Some("cat.png"));
        assert_eq!(record.captured_at, mtime);
        assert_eq!(record.source, ImageSource::Local);
        assert_eq!((record.width, record.height), (40, 20));
        assert_eq!(record.mime, "image/png");
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

    #[test]
    fn a_folder_with_no_images_reports_nothing_rather_than_failing() {
        let source = tempfile::tempdir().unwrap();
        let (_dir, library) = library();

        let (report, _) = run(&library, &[source.path().to_path_buf()]);

        assert_eq!(report, ImportReport::default());
        assert_eq!(library.image_count().unwrap(), 0);
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
}
