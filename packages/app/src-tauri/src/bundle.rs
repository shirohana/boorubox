//! Legacy bundle import (`legacy-bundle-import` design D1–D6, D8): read the
//! extension's exported SQLite parts and map their rows onto the schema this
//! app ships. `import.rs` is the model for the run shape — a blocking
//! function, `import:progress` ticks, one `ImportReport`.
//!
//! The mapping lives here alone (design D2): nothing outside this file knows
//! the legacy column names (`imageUrl`, `savedAt`, `isDeleted`, …).

use std::path::{Path, PathBuf};

use rusqlite::{Connection, OpenFlags, Row};

use crate::db;
use crate::error::Result;
use crate::import;
use crate::ingest::{self, IngestInput, Ingested};
use crate::library::{SharedLibrary, with_library};
use crate::model::{ImageSource, ImportOutcome, ImportProgress, ImportReport};
use crate::thumbs;

/// One row of a bundle part's `images` table, read as-is; `import_row` maps it
/// onto an [`IngestInput`]. `is_deleted` is `Option`: the legacy column is
/// nullable with `DEFAULT 0`, and a `NULL` means the same as `0` — the
/// extension never set it, not that deletion is somehow unknown.
struct BundleRow {
    id: String,
    image_url: String,
    page_url: String,
    page_title: Option<String>,
    saved_at: i64,
    tags_json: Option<String>,
    is_deleted: Option<bool>,
    rating: Option<String>,
    blob: Vec<u8>,
}

/// What opening a part found, before any of its rows is imported: either it
/// answered a row count, or it is one failed item naming the file (design D5).
enum PartPlan {
    Failed(PathBuf, String),
    Open(PathBuf, Connection, i64),
}

impl PartPlan {
    /// How many report items this part is worth — its row count, or one for
    /// itself when it never opened. `total` is this summed over every part,
    /// read before any row (design D5).
    fn work(&self) -> u32 {
        match self {
            PartPlan::Failed(..) => 1,
            PartPlan::Open(_, _, count) => *count as u32,
        }
    }
}

/// Import every row of `files`, streaming each part from its own read-only
/// connection, calling `on_progress` as it goes (mirrors [`import::import_paths`]).
///
/// Blocking by design, for the same reason as `import_paths`: the caller puts
/// it on a thread and turns `on_progress` into the `import:progress` event.
/// The library is locked one row at a time, never across the loop, so a
/// search asked for mid-run is answered between two rows.
pub fn import_bundle(
    library: &SharedLibrary,
    files: &[PathBuf],
    on_progress: &mut dyn FnMut(ImportProgress),
) -> Result<ImportReport> {
    import::refuse_if_closed(library)?;

    let plans: Vec<PartPlan> = sorted_parts(files).into_iter().map(open_part).collect();
    let total: u32 = plans.iter().map(PartPlan::work).sum();

    let mut report = ImportReport::default();
    on_progress(import::progress(&report, total));
    for plan in plans {
        match plan {
            PartPlan::Failed(path, reason) => {
                import::count(&mut report, failed_file(&path, reason));
                on_progress(import::progress(&report, total));
            }
            PartPlan::Open(path, conn, _) => {
                let source_ref = source_ref_of(&path);
                // Streamed straight off the part's own statement, one row at a
                // time: collecting every row (blob included) into a `Vec`
                // first would peak well above the bundle's own size for a
                // 200-row part of multi-megabyte images. Holding this
                // statement across `import_row`'s `with_library` call below is
                // safe — a different connection, on a different file, guarded
                // by no lock this app takes.
                let mut stmt = prepare_rows(&conn)?;
                let rows = stmt.query_map([], row_from_sql)?;
                for (index, row) in rows.enumerate() {
                    // A row rusqlite itself could not read (a column typed
                    // differently than expected) is that row's own failure,
                    // never a reason to drop every row after it.
                    let outcome = match row {
                        Ok(row) => import_row(library, source_ref.as_deref(), row),
                        Err(error) => failed_file(&path, format!("row {index}: {error}")),
                    };
                    import::count(&mut report, outcome);
                    on_progress(import::progress(&report, total));
                }
            }
        }
    }
    Ok(report)
}

/// `files` in the order the run processes them: by `part<N>` first, then by
/// name (design D5) — the order a re-run resumes skips in must not depend on
/// the order the file picker handed them over in. A file named twice is
/// imported once — `files` is what the picker selected, not a guarantee it
/// selected each part only once.
fn sorted_parts(files: &[PathBuf]) -> Vec<PathBuf> {
    let mut files = files.to_vec();
    files.sort_by(|a, b| part_sort_key(a).cmp(&part_sort_key(b)));
    files.dedup();
    files
}

/// Numbered parts first, in `part<N>` order; a numberless single-part
/// `database.db` sorts after all of them, by name — never ahead of
/// `database-part1of…db` by an accident of `Option`'s own ordering.
fn part_sort_key(path: &Path) -> (u32, u32, &Path) {
    match part_number(path) {
        Some(n) => (0, n, path),
        None => (1, 0, path),
    }
}

/// The `<N>` out of a `database-part<N>of<M>.db` file name, matching the
/// `-part<N>of<M>` shape specifically — not just any file whose name happens
/// to contain "part" somewhere in it.
fn part_number(path: &Path) -> Option<u32> {
    let name = path.file_stem()?.to_str()?;
    let (number, rest) = leading_digits(name.rsplit_once("-part")?.1);
    if number.is_empty() {
        return None;
    }
    let (of, _name) = leading_digits(rest.strip_prefix("of")?);
    if of.is_empty() {
        return None;
    }
    number.parse().ok()
}

fn leading_digits(text: &str) -> (&str, &str) {
    let end = text
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(text.len());
    text.split_at(end)
}

/// Open `path` read-only and confirm it answers an `images` table. Every way a
/// part can be unusable — not absolute (the command's own contract), not a
/// SQLite file at all, or one with no `images` table — surfaces as the same
/// `PartPlan::Failed` here, so a bad entry among `files` becomes one failed
/// item naming it rather than aborting the run (design D5).
fn open_part(path: PathBuf) -> PartPlan {
    if !path.is_absolute() {
        return PartPlan::Failed(path, "not an absolute path".to_string());
    }
    let opened = open_readonly(&path).and_then(|conn| row_count(&conn).map(|count| (conn, count)));
    match opened {
        Ok((conn, count)) => PartPlan::Open(path, conn, count),
        Err(error) => PartPlan::Failed(path, error.to_string()),
    }
}

fn open_readonly(path: &Path) -> Result<Connection> {
    Ok(Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?)
}

fn row_count(conn: &Connection) -> Result<i64> {
    Ok(conn.query_row("SELECT COUNT(*) FROM images", [], |row| row.get(0))?)
}

const ROW_COLUMNS: &str =
    "id, imageUrl, pageUrl, pageTitle, savedAt, tags, isDeleted, rating, blob";

/// The `SELECT` every row of `conn`'s `images` table streams from, in
/// `rowid` order (design D5) — the order a re-run resumes skips in.
fn prepare_rows(conn: &Connection) -> Result<rusqlite::Statement<'_>> {
    Ok(conn.prepare(&format!("SELECT {ROW_COLUMNS} FROM images ORDER BY rowid"))?)
}

fn row_from_sql(row: &Row) -> rusqlite::Result<BundleRow> {
    Ok(BundleRow {
        id: row.get(0)?,
        image_url: row.get(1)?,
        page_url: row.get(2)?,
        page_title: row.get(3)?,
        saved_at: row.get(4)?,
        tags_json: row.get(5)?,
        is_deleted: row.get(6)?,
        rating: row.get(7)?,
        blob: row.get(8)?,
    })
}

/// `source_ref` for every row of `path`'s part: the bundle folder's name, the
/// parent directory of the part file (design D1) — `None` when the path has
/// no parent to name.
fn source_ref_of(path: &Path) -> Option<String> {
    path.parent()
        .and_then(Path::file_name)
        .map(|name| name.to_string_lossy().into_owned())
}

/// Map one row onto [`IngestInput`] and store it (design D1). Every
/// `store_image` error — including an undecodable blob — is `failed` here,
/// not `skipped`: unlike a local import's undecodable file, a bundle row is a
/// real image the export already had, so its own failure to decode is a real
/// attempt the machine stopped, not a benign non-image found in a folder.
fn import_row(library: &SharedLibrary, source_ref: Option<&str>, row: BundleRow) -> ImportOutcome {
    let tags = match parse_tags(row.tags_json.as_deref()) {
        Ok(tags) => tags,
        Err(reason) => return failed_row(&row, reason),
    };
    // The import's own moment, not `updatedAt`: D1 records why — `updatedAt`
    // means "last edited" and is present on a small minority of rows, while
    // the trash view orders by `deleted_at`, so the import's moment is the one
    // honest timestamp every deleted row can carry. A `NULL` `isDeleted` reads
    // as not deleted, the same as `0` — the column is nullable and the
    // extension never treated the absence of a value as a third state.
    let deleted_at = row.is_deleted.unwrap_or(false).then(db::now_ms);

    let stored = with_library(library, |library| {
        let ingested = ingest::store_image(
            library,
            IngestInput {
                id: &row.id,
                bytes: &row.blob,
                source: ImageSource::LegacyBundle,
                source_ref,
                image_url: Some(&row.image_url),
                page_url: Some(&row.page_url),
                page_title: row.page_title.as_deref(),
                adapter: None,
                rating: row.rating.as_deref(),
                tags: &tags,
                captured_at: row.saved_at,
                file_modified_at: None,
                deleted_at,
            },
        )?;
        Ok((library.paths.clone(), ingested))
    });

    match stored {
        Ok((paths, Ingested::Created(record))) => {
            thumbs::warm_thumbnail(&paths, &record);
            imported_row(&row, record.id.clone())
        }
        Ok((_, Ingested::Existing(_))) => skipped_row(&row, "id already in the library"),
        Err(error) => failed_row(&row, error.to_string()),
    }
}

/// The bundle's `tags` column: a JSON array of strings, or absent. Anything
/// else is this row's own failure (task 1.3's "unreadable tags value"), never
/// the whole part's.
fn parse_tags(raw: Option<&str>) -> std::result::Result<Vec<String>, String> {
    match raw {
        None => Ok(Vec::new()),
        Some(text) => serde_json::from_str::<Vec<String>>(text)
            .map_err(|error| format!("tags column is not a JSON array of strings: {error}")),
    }
}

fn imported_row(row: &BundleRow, id: String) -> ImportOutcome {
    ImportOutcome::imported(row.page_url.clone(), id)
}

fn skipped_row(row: &BundleRow, reason: impl Into<String>) -> ImportOutcome {
    ImportOutcome::skipped(row.page_url.clone(), Some(row.id.clone()), reason)
}

fn failed_row(row: &BundleRow, reason: impl Into<String>) -> ImportOutcome {
    ImportOutcome::failed(row.page_url.clone(), Some(row.id.clone()), reason)
}

fn failed_file(path: &Path, reason: impl Into<String>) -> ImportOutcome {
    ImportOutcome::failed(path.display().to_string(), None, reason)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::Library;
    use crate::model::{ImportStatus, ParsedTagSearch, SearchRequest, SearchView, Sort};

    const FIXTURE: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/fixtures/legacy-bundle/database.db"
    );

    fn library() -> (tempfile::TempDir, SharedLibrary) {
        let dir = tempfile::tempdir().unwrap();
        let library = Library::open_or_create(dir.path()).unwrap();
        (
            dir,
            SharedLibrary::new(std::sync::Mutex::new(Some(library))),
        )
    }

    /// Every row the bundle stored, trashed or not — what task 1.3 means by
    /// "the library holds N images": `Library::image_count` excludes the trash
    /// (`trash` design D2), and the deleted row is still held, just trashed.
    fn total_count(library: &SharedLibrary) -> i64 {
        with_library(library, |library| {
            Ok(library
                .conn
                .query_row("SELECT COUNT(*) FROM images", [], |row| row.get(0))?)
        })
        .unwrap()
    }

    fn run(library: &SharedLibrary, files: &[PathBuf]) -> ImportReport {
        import_bundle(library, files, &mut |_| {}).unwrap()
    }

    fn search(library: &SharedLibrary, view: SearchView, query: ParsedTagSearch) -> Vec<String> {
        with_library(library, |library| {
            crate::query::search(
                &library.conn,
                &SearchRequest {
                    query,
                    text: String::new(),
                    view,
                    sort: Sort::default(),
                    group: Default::default(),
                    limit: 100,
                    offset: 0,
                },
            )
        })
        .unwrap()
        .images
        .into_iter()
        .map(|record| record.id)
        .collect()
    }

    fn items_with(report: &ImportReport, status: ImportStatus) -> Vec<&ImportOutcome> {
        report
            .items
            .iter()
            .filter(|item| item.status == status)
            .collect()
    }

    #[test]
    fn a_clean_import_stores_three_images_and_fails_the_corrupt_one() {
        let (_dir, library) = library();

        let report = run(&library, &[PathBuf::from(FIXTURE)]);

        assert_eq!(
            (report.imported, report.skipped, report.failed),
            (3, 0, 1),
            "{:?}",
            report.items
        );
        assert_eq!(total_count(&library), 3);
        let failed = items_with(&report, ImportStatus::Failed);
        assert_eq!(failed.len(), 1);
        assert!(
            failed[0]
                .reason
                .as_deref()
                .is_some_and(|reason| !reason.is_empty()),
            "the failed item must carry a decode reason: {:?}",
            failed[0]
        );

        let deleted = with_library(&library, |library| {
            ingest::load_record(&library.conn, "04612bb6-eacd-4171-8833-5bf6175a5ab2")
        })
        .unwrap()
        .unwrap();
        assert_eq!(deleted.source_ref.as_deref(), Some("legacy-bundle"));
        assert!(deleted.deleted_at.is_some());
    }

    #[test]
    fn re_importing_the_same_bundle_skips_everything_already_stored() {
        let (_dir, library) = library();
        run(&library, &[PathBuf::from(FIXTURE)]);

        let report = run(&library, &[PathBuf::from(FIXTURE)]);

        assert_eq!(
            (report.imported, report.skipped, report.failed),
            (0, 3, 1),
            "{:?}",
            report.items
        );
        assert_eq!(total_count(&library), 3);
    }

    #[test]
    fn a_trashed_row_is_stored_deleted_and_absent_from_the_library_search() {
        let (_dir, library) = library();

        run(&library, &[PathBuf::from(FIXTURE)]);

        assert_eq!(
            search(&library, SearchView::Library, ParsedTagSearch::default()).len(),
            2,
            "the two live rows, the deleted one held back"
        );
        let trashed = search(&library, SearchView::Trash, ParsedTagSearch::default());
        assert_eq!(
            trashed.len(),
            1,
            "exactly the one isDeleted row: {trashed:?}"
        );
    }

    #[test]
    fn a_tag_from_the_bundle_is_searchable() {
        let (_dir, library) = library();
        run(&library, &[PathBuf::from(FIXTURE)]);

        let found = search(
            &library,
            SearchView::Library,
            ParsedTagSearch {
                include_tags: vec!["honkai:_star_rail".to_string()],
                ..Default::default()
            },
        );

        assert_eq!(found.len(), 1, "{found:?}");
    }

    #[test]
    fn the_deleted_rows_rating_is_searchable_from_the_trash() {
        let (_dir, library) = library();
        run(&library, &[PathBuf::from(FIXTURE)]);

        let found = search(
            &library,
            SearchView::Trash,
            ParsedTagSearch {
                ratings: vec!["e".to_string()],
                ..Default::default()
            },
        );

        assert_eq!(found.len(), 1, "{found:?}");
    }

    #[test]
    fn captured_at_is_the_rows_saved_at() {
        let (_dir, library) = library();
        run(&library, &[PathBuf::from(FIXTURE)]);

        let record = with_library(&library, |library| {
            ingest::load_record(&library.conn, "02e747de-33df-444b-ad36-2e45afcd01ba")
        })
        .unwrap()
        .unwrap();

        assert_eq!(record.captured_at, 1_762_591_888_468);
    }

    #[test]
    fn path_is_the_page_url_and_id_is_set_on_skipped_items_too() {
        let (_dir, library) = library();
        run(&library, &[PathBuf::from(FIXTURE)]);

        let report = run(&library, &[PathBuf::from(FIXTURE)]);

        let skipped = items_with(&report, ImportStatus::Skipped);
        assert_eq!(skipped.len(), 3);
        for item in skipped {
            assert!(item.id.is_some(), "{item:?}");
            assert!(
                item.path.starts_with("https://x.com/"),
                "path must be the page url: {item:?}"
            );
        }
    }

    #[test]
    fn a_non_sqlite_file_among_files_is_one_failed_item_and_the_others_still_import() {
        let (_dir, library) = library();
        let bogus = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(bogus.path(), b"not a sqlite database at all").unwrap();

        let report = run(
            &library,
            &[PathBuf::from(FIXTURE), bogus.path().to_path_buf()],
        );

        assert_eq!(report.imported, 3);
        assert_eq!(report.failed, 2, "the corrupt row plus the bogus file");
        let file_failure = report
            .items
            .iter()
            .find(|item| item.path == bogus.path().display().to_string())
            .expect("the bogus file must be named in the report");
        assert_eq!(file_failure.status, ImportStatus::Failed);
        assert!(file_failure.id.is_none());
    }

    /// The legacy `images` table shape, shared by every test that builds its
    /// own part rather than using the fixture.
    const LEGACY_SCHEMA: &str = "CREATE TABLE images (id TEXT PRIMARY KEY, imageUrl TEXT NOT NULL,
         pageUrl TEXT NOT NULL, pageTitle TEXT, mimeType TEXT NOT NULL,
         fileSize INTEGER NOT NULL, width INTEGER NOT NULL, height INTEGER NOT NULL,
         savedAt INTEGER NOT NULL, updatedAt INTEGER, tags TEXT,
         isDeleted INTEGER, rating TEXT, blob BLOB NOT NULL);";

    #[test]
    fn an_unreadable_tags_value_fails_that_row_alone() {
        let (_dir, library) = library();
        let part = tempfile::NamedTempFile::new().unwrap();
        {
            let conn = Connection::open(part.path()).unwrap();
            conn.execute_batch(LEGACY_SCHEMA).unwrap();
            let good_bytes = crate::http::test_support::png_bytes(2, 2);
            conn.execute(
                "INSERT INTO images (id, imageUrl, pageUrl, pageTitle, mimeType, fileSize, width,
                     height, savedAt, tags, isDeleted, rating, blob)
                 VALUES ('good', 'https://example.test/i.png', 'https://example.test/p', 'ok',
                     'image/png', 1, 1, 1, 0, '[\"cat\"]', 0, NULL, ?1)",
                rusqlite::params![good_bytes],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO images (id, imageUrl, pageUrl, pageTitle, mimeType, fileSize, width,
                     height, savedAt, tags, isDeleted, rating, blob)
                 VALUES ('bad-tags', 'https://example.test/i2.png', 'https://example.test/p2',
                     'bad', 'image/png', 1, 1, 1, 0, 'not json', 0, NULL, X'00')",
                [],
            )
            .unwrap();
        }

        let report = run(&library, &[part.path().to_path_buf()]);

        assert_eq!(report.imported, 1);
        assert_eq!(report.failed, 1);
        let failed = items_with(&report, ImportStatus::Failed);
        assert_eq!(failed[0].id.as_deref(), Some("bad-tags"));
        assert!(
            failed[0].reason.as_deref().unwrap().contains("tags column"),
            "{:?}",
            failed[0]
        );
        assert_eq!(total_count(&library), 1);
    }

    /// The legacy column is nullable with `DEFAULT 0`; a real export never
    /// wrote a `NULL` there, but nothing rules it out, and it must read as
    /// "not deleted" rather than failing the row's `bool` fetch.
    #[test]
    fn a_null_is_deleted_column_is_treated_as_not_deleted() {
        let (_dir, library) = library();
        let part = tempfile::NamedTempFile::new().unwrap();
        {
            let conn = Connection::open(part.path()).unwrap();
            conn.execute_batch(LEGACY_SCHEMA).unwrap();
            let bytes = crate::http::test_support::png_bytes(2, 2);
            conn.execute(
                "INSERT INTO images (id, imageUrl, pageUrl, pageTitle, mimeType, fileSize, width,
                     height, savedAt, tags, isDeleted, rating, blob)
                 VALUES ('null-deleted', 'https://example.test/i.png', 'https://example.test/p',
                     'ok', 'image/png', 1, 1, 1, 0, '[]', NULL, NULL, ?1)",
                rusqlite::params![bytes],
            )
            .unwrap();
        }

        let report = run(&library, &[part.path().to_path_buf()]);

        assert_eq!(report.imported, 1, "{:?}", report.items);
        let record = with_library(&library, |library| {
            ingest::load_record(&library.conn, "null-deleted")
        })
        .unwrap()
        .unwrap();
        assert_eq!(record.deleted_at, None);
    }

    #[test]
    fn parts_are_ordered_by_part_number_then_name() {
        let files = [
            PathBuf::from("/bundle/database-part10of18.db"),
            PathBuf::from("/bundle/database-part2of18.db"),
            PathBuf::from("/bundle/database-part1of18.db"),
        ];

        let sorted = sorted_parts(&files);

        assert_eq!(
            sorted,
            vec![
                PathBuf::from("/bundle/database-part1of18.db"),
                PathBuf::from("/bundle/database-part2of18.db"),
                PathBuf::from("/bundle/database-part10of18.db"),
            ]
        );
    }

    /// A single-part bundle's plain `database.db` never sorts ahead of a
    /// numbered part by an accident of `Option`'s own ordering (finding 7).
    #[test]
    fn a_numberless_database_db_sorts_after_every_numbered_part() {
        let files = [
            PathBuf::from("/bundle/database.db"),
            PathBuf::from("/bundle/database-part2of2.db"),
            PathBuf::from("/bundle/database-part1of2.db"),
        ];

        let sorted = sorted_parts(&files);

        assert_eq!(
            sorted,
            vec![
                PathBuf::from("/bundle/database-part1of2.db"),
                PathBuf::from("/bundle/database-part2of2.db"),
                PathBuf::from("/bundle/database.db"),
            ]
        );
    }

    /// `part_number` used to match any file whose name merely contained
    /// "part" somewhere in it.
    #[test]
    fn a_name_that_merely_contains_part_is_not_mistaken_for_a_numbered_one() {
        assert_eq!(part_number(Path::new("/bundle/important.db")), None);
        assert_eq!(part_number(Path::new("/bundle/database-parted.db")), None);
    }

    #[test]
    fn a_file_listed_twice_is_imported_once() {
        let files = [PathBuf::from(FIXTURE), PathBuf::from(FIXTURE)];

        assert_eq!(sorted_parts(&files), vec![PathBuf::from(FIXTURE)]);
    }

    #[test]
    fn a_relative_path_among_files_is_one_failed_item_and_the_others_still_import() {
        let (_dir, library) = library();

        let report = run(
            &library,
            &[
                PathBuf::from(FIXTURE),
                PathBuf::from("relative/database.db"),
            ],
        );

        assert_eq!(report.imported, 3);
        assert_eq!(report.failed, 2, "the corrupt row plus the relative path");
        let file_failure = report
            .items
            .iter()
            .find(|item| item.path == "relative/database.db")
            .expect("the relative path must be named in the report");
        assert_eq!(file_failure.status, ImportStatus::Failed);
        assert!(file_failure.id.is_none());
    }
}
