//! Trashing, restoring and permanently deleting images (`trash` design
//! D3–D7): a soft delete that only a restore or a permanent delete undoes,
//! and the permanent delete that folds in what `maintenance::drop_image_record`
//! used to do alone before this change removed it.

use std::path::Path;

use rusqlite::{OptionalExtension, params};

use crate::db;
use crate::error::{AppError, Result};
use crate::library::Library;
use crate::model::DeleteReport;
use crate::tags;
use crate::thumbs::thumbnail_path;

/// Move every id in `ids` to the trash: `deleted_at` is set and `updated_at`
/// moves, and nothing else about the row changes (design D3) — the file, the
/// thumbnail, the tags and the rating all stay exactly as they were, because a
/// restore has to hand the image back as it was.
///
/// One transaction over every id (`selection-and-bulk` design D10): an
/// unknown id anywhere in `ids` fails the whole call and changes no row.
pub fn trash_images(library: &Library, ids: &[String]) -> Result<()> {
    let now = db::now_ms();
    set_deleted_at(library, ids, Some(now), now)
}

/// Put every id in `ids` back: `deleted_at` clears and `updated_at` moves,
/// exactly the reverse of [`trash_images`]. `captured_at` never moved, which
/// is what keeps a restored image in its original place in the grid's order
/// (design D10).
pub fn restore_images(library: &Library, ids: &[String]) -> Result<()> {
    set_deleted_at(library, ids, None, db::now_ms())
}

fn set_deleted_at(
    library: &Library,
    ids: &[String],
    deleted_at: Option<i64>,
    updated_at: i64,
) -> Result<()> {
    if ids.is_empty() {
        return Ok(());
    }
    let tx = library.conn.unchecked_transaction()?;
    for id in ids {
        let changed = tx.execute(
            "UPDATE images SET deleted_at = ?1, updated_at = ?2 WHERE id = ?3",
            params![deleted_at, updated_at, id],
        )?;
        if changed == 0 {
            return Err(AppError::NotFound(format!("image {id}")));
        }
    }
    tx.commit()?;

    // After the commit, never inside it (`library-sidecars` design D4).
    crate::sidecar::write_for(&library.paths, &library.conn, ids)?;
    Ok(())
}

/// Permanently delete every id in `ids`: the rows go in one transaction — the
/// cascades and the `images_fts_delete` trigger take `image_tags`, `posts` and
/// the search-index entry with them, and a tag whose last use was one of these
/// rows is collected once, right before the transaction commits — and only
/// then, outside the transaction, is each file unlinked and each thumbnail
/// removed (design D4).
///
/// Folds in what `maintenance::drop_image_record` used to do alone (design
/// D6): the row-and-thumbnail half is the same action; `delete_forever`
/// differs only in also unlinking the file under `images/`.
///
/// A file that will not go is named in `DeleteReport.files_left` rather than
/// failing the call — the record is gone either way, and there is no unlink
/// left to retry once it is (design D4). An id that is not in the trash fails
/// the whole call and removes no row, exactly as an unknown one does: the spec
/// offers permanent deletion only from the trash, and the row read is what
/// holds the app to that whatever a caller passes.
pub fn delete_forever(library: &Library, ids: &[String]) -> Result<DeleteReport> {
    if ids.is_empty() {
        return Ok(DeleteReport::default());
    }

    let tx = library.conn.unchecked_transaction()?;
    let mut removed = Vec::with_capacity(ids.len());
    let mut unlinked_tags = Vec::new();
    for id in ids {
        let ext: Option<String> = tx
            .query_row(
                "SELECT ext FROM images WHERE id = ?1 AND deleted_at IS NOT NULL",
                [id],
                |row| row.get(0),
            )
            .optional()?;
        let Some(ext) = ext else {
            return Err(AppError::NotFound(format!("image {id}")));
        };
        // Read before the delete: the cascade takes the `image_tags` rows with
        // the image, and nothing afterwards can say which tags they named.
        unlinked_tags.extend(tags::tag_ids_of(&tx, id)?);
        // `image_tags` and `posts` cascade off this row (`foreign_keys` is ON
        // from `db::open`), and the `images_fts_delete` trigger drops the
        // search-index entry.
        tx.execute("DELETE FROM images WHERE id = ?1", [id])?;
        removed.push((id.clone(), ext));
    }
    // Same transaction as the deletes (`tags-and-ratings` design D5): a `tags`
    // row with no use is an autocomplete suggestion that matches nothing.
    tags::collect_orphans(&tx, &unlinked_tags)?;
    tx.commit()?;

    let mut files_left = Vec::new();
    for (id, ext) in &removed {
        // Before the image file (`library-sidecars` design D5): a sidecar
        // that survives a permanent delete is the one thing that can
        // resurrect the image on a later rebuild, so its failure is named
        // exactly like a file that will not unlink rather than shrugged off.
        if crate::sidecar::remove_for(&library.paths, id).is_err() {
            files_left.push(
                crate::sidecar::path(&library.paths, id)
                    .display()
                    .to_string(),
            );
        }
        let path = library.paths.image_path(id, ext);
        if remove_if_present(&path).is_err() {
            files_left.push(path.display().to_string());
        }
        // A thumbnail is a derived cache `.thumbs/` may have lost at any time
        // (Phase 1 D7); a failure removing one costs nothing the user can
        // lose, so it is not worth reporting.
        let _ = remove_if_present(&thumbnail_path(&library.paths, id));
    }

    Ok(DeleteReport {
        deleted: removed.len() as i64,
        files_left,
    })
}

/// Permanently delete everything in the trash, over the same path as one
/// image (design D7): a second `DELETE … WHERE deleted_at IS NOT NULL` would
/// be a second place the unlink, the thumbnail removal, the orphan collection
/// and the report have to be kept in step, and it would drift the first time
/// one of them changes.
pub fn empty_trash(library: &Library) -> Result<DeleteReport> {
    delete_forever(library, &trashed_ids(library)?)
}

/// How many images are in the trash — a navigation badge with exactly one
/// class of writer, the four functions in this file (design D11).
pub fn trash_count(library: &Library) -> Result<i64> {
    let count = library.conn.query_row(
        "SELECT COUNT(*) FROM images WHERE deleted_at IS NOT NULL",
        [],
        |row| row.get(0),
    )?;
    Ok(count)
}

fn trashed_ids(library: &Library) -> Result<Vec<String>> {
    let mut stmt = library
        .conn
        .prepare("SELECT id FROM images WHERE deleted_at IS NOT NULL")?;
    let ids = stmt.query_map([], |row| row.get(0))?;
    Ok(ids.collect::<rusqlite::Result<_>>()?)
}

/// Remove `path`, treating "it was already gone" as success (design D4's "a
/// record whose file is already gone"). What a real failure means is the
/// caller's: an image is named to the user, a thumbnail is let go.
fn remove_if_present(path: &Path) -> std::io::Result<()> {
    match std::fs::remove_file(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ingest::{self, IngestInput, store_image};
    use crate::library::Library;
    use crate::maintenance;
    use crate::model::{ImageSource, ParsedTagSearch, SearchRequest, SearchView};
    use crate::query;

    fn png_bytes() -> Vec<u8> {
        let image = image::DynamicImage::ImageRgb8(image::RgbImage::new(4, 4));
        let mut out = std::io::Cursor::new(Vec::new());
        image.write_to(&mut out, image::ImageFormat::Png).unwrap();
        out.into_inner()
    }

    fn library() -> (tempfile::TempDir, Library) {
        let dir = tempfile::tempdir().unwrap();
        let library = Library::open_or_create(dir.path()).unwrap();
        (dir, library)
    }

    /// Rows enter through `ingest::store_image`, the one write path (Phase 1
    /// design D4), so a fixture cannot drift from what the app actually
    /// stores.
    fn store(library: &Library, id: &str, tags: &[&str], rating: Option<&str>) {
        let tags: Vec<String> = tags.iter().map(|tag| (*tag).to_string()).collect();
        store_image(
            library,
            IngestInput {
                id,
                bytes: &png_bytes(),
                source: ImageSource::Extension,
                source_ref: None,
                image_url: None,
                page_url: Some("https://x.com/alice/status/1"),
                page_title: Some("a page"),
                adapter: None,
                rating,
                tags: &tags,
                captured_at: 1_700_000_000_000,
                file_modified_at: None,
                deleted_at: None,
            },
        )
        .unwrap();
    }

    /// A row with no file, no thumbnail and no tags — cheap to insert by the
    /// thousand, for the tests that care only how many ids a call reaches.
    fn bare_image(library: &Library, id: &str) {
        library
            .conn
            .execute(
                "INSERT INTO images (id, ext, mime, size, width, height, source,
                                     captured_at, created_at, updated_at)
                 VALUES (?1, 'png', 'image/png', 1, 1, 1, 'local', 0, 0, 0)",
                [id],
            )
            .unwrap();
    }

    fn strs(ids: &[&str]) -> Vec<String> {
        ids.iter().map(|id| (*id).to_string()).collect()
    }

    fn search_view(library: &Library, view: SearchView) -> Vec<String> {
        query::search(
            &library.conn,
            &SearchRequest {
                query: ParsedTagSearch::default(),
                text: String::new(),
                view,
                sort: Default::default(),
                group: Default::default(),
                limit: 100,
                offset: 0,
            },
        )
        .unwrap()
        .images
        .into_iter()
        .map(|record| record.id)
        .collect()
    }

    fn thumbnail(library: &Library, id: &str) -> std::path::PathBuf {
        thumbnail_path(&library.paths, id)
    }

    /// A derived thumbnail these tests plant by hand rather than generating
    /// (`thumbs::ensure_thumbnail` costs an actual encode); the bucket it
    /// lives in is created on demand by the real writer (design D3), so the
    /// stand-in creates it too.
    fn write_thumbnail(library: &Library, id: &str, bytes: &[u8]) -> std::path::PathBuf {
        let path = thumbnail(library, id);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, bytes).unwrap();
        path
    }

    #[test]
    fn trashing_hides_an_image_from_the_library_view_and_shows_it_in_the_trash_view() {
        let (_dir, library) = library();
        store(&library, "a", &["cat"], Some("s"));
        store(&library, "b", &[], None);

        trash_images(&library, &strs(&["a"])).unwrap();

        assert_eq!(search_view(&library, SearchView::Library), vec!["b"]);
        assert_eq!(search_view(&library, SearchView::Trash), vec!["a"]);
    }

    #[test]
    fn trashing_touches_nothing_but_deleted_at_and_updated_at() {
        let (_dir, library) = library();
        store(&library, "a", &["cat", "cute"], Some("s"));
        write_thumbnail(&library, "a", b"a derived thumbnail");
        let image_path = library.paths.image_path("a", "png");
        let bytes_before = std::fs::read(&image_path).unwrap();

        trash_images(&library, &strs(&["a"])).unwrap();

        assert_eq!(
            std::fs::read(&image_path).unwrap(),
            bytes_before,
            "the file is untouched, byte for byte"
        );
        assert!(thumbnail(&library, "a").exists(), "the thumbnail stays");
        let record = ingest::require_record(&library.conn, "a").unwrap();
        assert_eq!(record.tags, vec!["cat".to_string(), "cute".to_string()]);
        assert_eq!(record.rating.as_deref(), Some("s"));
        assert!(record.deleted_at.is_some());
    }

    #[test]
    fn trashing_drops_the_library_counts_by_one_while_the_row_still_exists() {
        let (_dir, library) = library();
        store(&library, "a", &[], None);
        store(&library, "b", &[], None);

        trash_images(&library, &strs(&["a"])).unwrap();

        assert_eq!(library.image_count().unwrap(), 1);
        assert_eq!(maintenance::image_counts(&library).unwrap().total, 1);
        assert_eq!(
            library
                .conn
                .query_row("SELECT COUNT(*) FROM images", [], |row| row
                    .get::<_, i64>(0))
                .unwrap(),
            2,
            "the row is still there",
        );
    }

    #[test]
    fn restoring_returns_the_image_to_the_library_with_its_tags_and_rating() {
        let (_dir, library) = library();
        store(&library, "a", &["cat"], Some("q"));
        trash_images(&library, &strs(&["a"])).unwrap();

        restore_images(&library, &strs(&["a"])).unwrap();

        assert_eq!(search_view(&library, SearchView::Library), vec!["a"]);
        assert!(search_view(&library, SearchView::Trash).is_empty());
        let record = ingest::require_record(&library.conn, "a").unwrap();
        assert_eq!(record.tags, vec!["cat".to_string()]);
        assert_eq!(record.rating.as_deref(), Some("q"));
        assert_eq!(record.deleted_at, None);
    }

    /// `library-sidecars` task 1.7: a trashed image's sidecar records
    /// `deletedAt` and a restored one clears it, with the file never removed
    /// by either.
    #[test]
    fn trashing_records_deleted_at_in_the_sidecar_and_restoring_clears_it() {
        let (_dir, library) = library();
        store(&library, "a", &["cat"], Some("s"));
        let sidecar_path = crate::sidecar::path(&library.paths, "a");

        trash_images(&library, &strs(&["a"])).unwrap();
        let sidecar = crate::sidecar::read(&sidecar_path).unwrap();
        assert!(sidecar.deleted_at.is_some());
        assert!(sidecar_path.is_file());

        restore_images(&library, &strs(&["a"])).unwrap();
        let sidecar = crate::sidecar::read(&sidecar_path).unwrap();
        assert!(sidecar.deleted_at.is_none());
        assert!(sidecar_path.is_file());
    }

    #[test]
    fn an_unknown_id_fails_trash_and_restore_and_changes_no_row() {
        let (_dir, library) = library();
        store(&library, "a", &[], None);

        let error = trash_images(&library, &strs(&["a", "no-such-id"])).unwrap_err();
        assert!(matches!(error, AppError::NotFound(_)), "got {error}");
        assert!(
            ingest::require_record(&library.conn, "a")
                .unwrap()
                .deleted_at
                .is_none()
        );

        trash_images(&library, &strs(&["a"])).unwrap();
        let error = restore_images(&library, &strs(&["a", "no-such-id"])).unwrap_err();
        assert!(matches!(error, AppError::NotFound(_)), "got {error}");
        assert!(
            ingest::require_record(&library.conn, "a")
                .unwrap()
                .deleted_at
                .is_some()
        );
    }

    #[test]
    fn deleting_forever_removes_the_row_its_tags_its_posts_its_index_entry_its_thumbnail_and_its_file()
     {
        let (_dir, library) = library();
        store(&library, "a", &["cat", "solo"], None);
        store(&library, "b", &["cat"], None);
        library
            .conn
            .execute(
                "INSERT INTO posts (image_id, site, remote_id, posted_at)
                 VALUES ('a', 'testbooru', '1', 0)",
                [],
            )
            .unwrap();
        write_thumbnail(&library, "a", b"a derived thumbnail");
        let image_path = library.paths.image_path("a", "png");
        trash_images(&library, &strs(&["a"])).unwrap();

        let report = delete_forever(&library, &strs(&["a"])).unwrap();

        assert_eq!(
            report,
            DeleteReport {
                deleted: 1,
                files_left: vec![],
            }
        );
        assert_eq!(
            library
                .conn
                .query_row("SELECT COUNT(*) FROM images WHERE id = 'a'", [], |row| row
                    .get::<_, i64>(
                    0
                ))
                .unwrap(),
            0
        );
        assert_eq!(
            library
                .conn
                .query_row(
                    "SELECT COUNT(*) FROM image_tags WHERE image_id = 'a'",
                    [],
                    |row| row.get::<_, i64>(0)
                )
                .unwrap(),
            0
        );
        assert_eq!(
            library
                .conn
                .query_row(
                    "SELECT COUNT(*) FROM posts WHERE image_id = 'a'",
                    [],
                    |row| row.get::<_, i64>(0)
                )
                .unwrap(),
            0,
            "the cascade takes the post with the row"
        );
        assert!(search_view(&library, SearchView::Library).contains(&"b".to_string()));
        assert!(!thumbnail(&library, "a").exists());
        assert!(!image_path.exists());
        assert_eq!(
            library
                .conn
                .query_row("SELECT name FROM tags WHERE name = 'solo'", [], |row| {
                    row.get::<_, String>(0)
                })
                .optional()
                .unwrap(),
            None,
            "`solo` had no other use"
        );
        assert!(
            library
                .conn
                .query_row("SELECT name FROM tags WHERE name = 'cat'", [], |row| row
                    .get::<_, String>(
                    0
                ))
                .optional()
                .unwrap()
                .is_some(),
            "`cat` is still carried by b"
        );
    }

    /// `library-sidecars` task 1.7: `delete_forever` leaves no sidecar behind.
    #[test]
    fn delete_forever_leaves_no_sidecar() {
        let (_dir, library) = library();
        store(&library, "a", &[], None);
        trash_images(&library, &strs(&["a"])).unwrap();

        delete_forever(&library, &strs(&["a"])).unwrap();

        assert!(!crate::sidecar::path(&library.paths, "a").exists());
    }

    /// `library-sidecars` task 1.7: `empty_trash` over three images leaves
    /// none of their three sidecars.
    #[test]
    fn emptying_the_trash_leaves_no_sidecar_for_any_of_the_trashed_images() {
        let (_dir, library) = library();
        for id in ["a", "b", "c"] {
            store(&library, id, &[], None);
        }
        trash_images(&library, &strs(&["a", "b", "c"])).unwrap();

        empty_trash(&library).unwrap();

        for id in ["a", "b", "c"] {
            assert!(!crate::sidecar::path(&library.paths, id).exists());
        }
    }

    /// The `library-browse` search index must not keep matching an image the
    /// library no longer has.
    #[test]
    fn deleting_forever_takes_it_out_of_the_search_index() {
        let (_dir, library) = library();
        store(&library, "a", &[], None);
        let hits = |library: &Library, view: SearchView| {
            query::search(
                &library.conn,
                &SearchRequest {
                    query: ParsedTagSearch::default(),
                    text: "a page".to_string(),
                    view,
                    sort: Default::default(),
                    group: Default::default(),
                    limit: 10,
                    offset: 0,
                },
            )
            .unwrap()
            .total
        };
        assert_eq!(hits(&library, SearchView::Library), 1);
        trash_images(&library, &strs(&["a"])).unwrap();
        assert_eq!(hits(&library, SearchView::Trash), 1);

        delete_forever(&library, &strs(&["a"])).unwrap();

        assert_eq!(hits(&library, SearchView::Trash), 0);
    }

    #[test]
    fn deleting_forever_leaves_an_empty_files_left_when_the_file_is_already_gone() {
        let (_dir, library) = library();
        store(&library, "a", &[], None);
        trash_images(&library, &strs(&["a"])).unwrap();
        std::fs::remove_file(library.paths.image_path("a", "png")).unwrap();

        let report = delete_forever(&library, &strs(&["a"])).unwrap();

        assert_eq!(
            report,
            DeleteReport {
                deleted: 1,
                files_left: vec![],
            }
        );
    }

    #[test]
    fn an_unknown_id_fails_delete_forever_and_removes_no_row() {
        let (_dir, library) = library();
        store(&library, "a", &[], None);
        trash_images(&library, &strs(&["a"])).unwrap();

        let error = delete_forever(&library, &strs(&["a", "no-such-id"])).unwrap_err();

        assert!(matches!(error, AppError::NotFound(_)), "got {error}");
        assert_eq!(trash_count(&library).unwrap(), 1);
    }

    #[test]
    fn an_image_that_is_not_in_the_trash_cannot_be_deleted_forever() {
        let (_dir, library) = library();
        store(&library, "a", &[], None);
        store(&library, "b", &[], None);
        trash_images(&library, &strs(&["b"])).unwrap();

        let error = delete_forever(&library, &strs(&["b", "a"])).unwrap_err();

        assert!(matches!(error, AppError::NotFound(_)), "got {error}");
        assert_eq!(library.image_count().unwrap(), 1, "a is untouched");
        assert!(library.paths.image_path("a", "png").is_file());
        assert_eq!(trash_count(&library).unwrap(), 1, "and so is b");
    }

    #[cfg(unix)]
    #[test]
    fn a_file_that_will_not_unlink_is_named_and_the_row_still_goes() {
        use std::os::unix::fs::PermissionsExt;

        let (_dir, library) = library();
        store(&library, "a", &[], None);
        trash_images(&library, &strs(&["a"])).unwrap();
        let path = library.paths.image_path("a", "png");
        let sidecar_path = crate::sidecar::path(&library.paths, "a");
        let bucket = path.parent().unwrap().to_path_buf();
        // Removing a directory entry needs write access to the directory that
        // holds it, not to the file itself — this is what makes the unlink
        // below fail without touching the file's own permissions. That
        // directory is the id's bucket (design D1), not `images/` itself, and
        // it holds the sidecar too, so both unlinks fail the same way.
        std::fs::set_permissions(&bucket, std::fs::Permissions::from_mode(0o555)).unwrap();

        let report = delete_forever(&library, &strs(&["a"]));

        // Restored before any assertion can panic and leave the temp dir
        // undeletable.
        std::fs::set_permissions(&bucket, std::fs::Permissions::from_mode(0o755)).unwrap();
        let report = report.unwrap();

        assert_eq!(report.deleted, 1);
        assert_eq!(
            report.files_left,
            vec![
                sidecar_path.display().to_string(),
                path.display().to_string()
            ]
        );
        assert_eq!(trash_count(&library).unwrap(), 0, "the row went anyway");
    }

    #[test]
    fn emptying_the_trash_removes_only_the_trashed_images_and_leaves_the_rest_with_their_files() {
        let (_dir, library) = library();
        for id in ["t1", "t2", "t3"] {
            store(&library, id, &[], None);
        }
        for id in ["l1", "l2", "l3", "l4"] {
            store(&library, id, &[], None);
        }
        trash_images(&library, &strs(&["t1", "t2", "t3"])).unwrap();

        let report = empty_trash(&library).unwrap();

        assert_eq!(report.deleted, 3);
        assert!(report.files_left.is_empty());
        assert_eq!(library.image_count().unwrap(), 4);
        for id in ["l1", "l2", "l3", "l4"] {
            assert!(library.paths.image_path(id, "png").is_file());
        }
        for id in ["t1", "t2", "t3"] {
            assert!(!library.paths.image_path(id, "png").exists());
        }
    }

    #[test]
    fn emptying_an_empty_trash_is_a_no_op_reporting_zero() {
        let (_dir, library) = library();
        store(&library, "a", &[], None);

        let report = empty_trash(&library).unwrap();

        assert_eq!(report, DeleteReport::default());
        assert_eq!(library.image_count().unwrap(), 1);
    }

    #[test]
    fn trash_count_tracks_trash_restore_delete_and_empty_across_a_sequence() {
        let (_dir, library) = library();
        for id in ["a", "b", "c"] {
            store(&library, id, &[], None);
        }
        assert_eq!(trash_count(&library).unwrap(), 0);

        trash_images(&library, &strs(&["a", "b"])).unwrap();
        assert_eq!(trash_count(&library).unwrap(), 2);

        restore_images(&library, &strs(&["a"])).unwrap();
        assert_eq!(trash_count(&library).unwrap(), 1, "only b is still trashed");

        trash_images(&library, &strs(&["c"])).unwrap();
        assert_eq!(trash_count(&library).unwrap(), 2, "b and c are trashed");

        delete_forever(&library, &strs(&["b"])).unwrap();
        assert_eq!(trash_count(&library).unwrap(), 1, "c is still trashed");

        empty_trash(&library).unwrap();
        assert_eq!(trash_count(&library).unwrap(), 0);
    }

    /// A selection can cover a whole library, so a trash write has to reach
    /// every id in it — past the point where one statement could bind them
    /// all (`query::ID_CHUNK`).
    #[test]
    fn trashing_and_restoring_reach_every_id_past_the_sqlite_variable_chunk_size() {
        let (_dir, library) = library();
        let ids: Vec<String> = (0..2500).map(|index| format!("img-{index}")).collect();
        for id in &ids {
            bare_image(&library, id);
        }

        trash_images(&library, &ids).unwrap();
        assert_eq!(trash_count(&library).unwrap(), 2500);

        restore_images(&library, &ids).unwrap();
        assert_eq!(trash_count(&library).unwrap(), 0);
    }
}
