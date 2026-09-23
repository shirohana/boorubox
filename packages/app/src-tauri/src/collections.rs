//! Collections (`collections` design D1–D3): named, unordered sets of images
//! the user keeps for themselves — never a tag, never uploaded. Shaped like
//! `tags.rs` and `rules.rs`: a small store over one table plus a join table,
//! with the library-level file rewritten by every write that changes a
//! collection's own row (`create`, `rename`, `delete`), never by a membership
//! change (`add`, `remove`), which only ever touches the members' sidecars.

use rusqlite::{Connection, Row, params, params_from_iter};

use crate::db;
use crate::error::{AppError, Result};
use crate::ingest;
use crate::library::Library;
use crate::model::{Collection, CollectionCount, ImageRecord};
use crate::query::{ID_CHUNK, placeholders, text_values};
use crate::tags;

const COLLECTION_COLUMNS: &str = "id, name, slug, created_at, updated_at, pinned";

fn row_to_collection(row: &Row) -> rusqlite::Result<Collection> {
    Ok(Collection {
        id: row.get(0)?,
        name: row.get(1)?,
        slug: row.get(2)?,
        created_at: row.get(3)?,
        updated_at: row.get(4)?,
        pinned: row.get(5)?,
    })
}

/// `name`, trimmed, lower-cased, every run of whitespace collapsed to one `_`
/// (design D2) — `"My  Favorites"` becomes `"my_favorites"`, and a name that
/// trims to nothing becomes the empty string, which every writer below refuses
/// as `BadRequest`. Computed here, once: `collection:` compiles against this
/// same value (`query::push_collections`), and every `Collection` /
/// `CollectionCount` record hands it to the webview rather than letting
/// TypeScript compute a second answer.
///
/// The spelling itself is [`tags::underscored`], shared with the derived
/// artist tag (`auto-artist-tag` design D1): change it there, for both.
pub fn slug(name: &str) -> String {
    tags::underscored(name)
}

fn require_collection(conn: &Connection, id: &str) -> Result<Collection> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {COLLECTION_COLUMNS} FROM collections WHERE id = ?1"
    ))?;
    match stmt.query_row([id], row_to_collection) {
        Ok(collection) => Ok(collection),
        Err(rusqlite::Error::QueryReturnedNoRows) => {
            Err(AppError::NotFound(format!("collection {id}")))
        }
        Err(error) => Err(error.into()),
    }
}

/// The id of the collection whose slug is `slug_value`, or the refusal
/// `tags::apply_edit` (`stamps` design D2) answers a stamp naming one that
/// does not exist with. Resolved before any write of a stamp's edit: a bad
/// slug must refuse the whole thing rather than leave some ids of it changed
/// and others not.
pub(crate) fn id_for_slug(conn: &Connection, slug_value: &str) -> Result<String> {
    conn.query_row(
        "SELECT id FROM collections WHERE slug = ?1",
        [slug_value],
        |row| row.get(0),
    )
    .map_err(|error| match error {
        rusqlite::Error::QueryReturnedNoRows => {
            AppError::BadRequest(format!("no collection named `{slug_value}`"))
        }
        other => other.into(),
    })
}

/// The name of whichever other collection already holds `slug_value`, or
/// `None` when it is free — the one check `create` and `rename` share, so a
/// slug clash always names the collection that holds it (spec `collections`,
/// "A duplicate name"). `excluding` is the id being renamed, so renaming a
/// collection to the name it already has is never a clash with itself.
fn holder_of_slug(
    conn: &Connection,
    slug_value: &str,
    excluding: Option<&str>,
) -> Result<Option<String>> {
    let mut stmt = conn.prepare("SELECT name FROM collections WHERE slug = ?1 AND id != ?2")?;
    match stmt.query_row(params![slug_value, excluding.unwrap_or("")], |row| {
        row.get::<_, String>(0)
    }) {
        Ok(name) => Ok(Some(name)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(error) => Err(error.into()),
    }
}

/// Every collection, by name (design D3) — the sidebar and the menus sort it
/// themselves where they need a different order (active-first, design D8).
pub fn list(conn: &Connection) -> Result<Vec<Collection>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {COLLECTION_COLUMNS} FROM collections ORDER BY name COLLATE NOCASE"
    ))?;
    let rows = stmt.query_map([], row_to_collection)?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

/// Create a collection named `name` (design D3): a uuid id, `BadRequest`
/// naming the holder on a slug clash, `BadRequest` on a blank name.
pub fn create(library: &Library, name: &str) -> Result<Collection> {
    let name = name.trim();
    let slug_value = slug(name);
    if slug_value.is_empty() {
        return Err(AppError::BadRequest(
            "a collection needs a name".to_string(),
        ));
    }
    if let Some(holder) = holder_of_slug(&library.conn, &slug_value, None)? {
        return Err(AppError::BadRequest(format!(
            "{holder:?} already has this name"
        )));
    }

    let now = db::now_ms();
    let id = uuid::Uuid::new_v4().to_string();
    library.conn.execute(
        "INSERT INTO collections (id, name, slug, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?4)",
        params![id, name, slug_value, now],
    )?;
    let collection = require_collection(&library.conn, &id)?;
    crate::sidecar::write_library(&library.paths, &library.conn)?;
    Ok(collection)
}

/// Rename `id` to `name` (design D3): same slug-clash and blank-name rules as
/// [`create`]. Rewrites `library.json`; never a sidecar — a rename changes no
/// image (spec `collections`, "Create and rename"; `library-recovery`,
/// "Rename touches one file").
pub fn rename(library: &Library, id: &str, name: &str) -> Result<Collection> {
    require_collection(&library.conn, id)?;
    let name = name.trim();
    let slug_value = slug(name);
    if slug_value.is_empty() {
        return Err(AppError::BadRequest(
            "a collection needs a name".to_string(),
        ));
    }
    if let Some(holder) = holder_of_slug(&library.conn, &slug_value, Some(id))? {
        return Err(AppError::BadRequest(format!(
            "{holder:?} already has this name"
        )));
    }

    let now = db::now_ms();
    library.conn.execute(
        "UPDATE collections SET name = ?1, slug = ?2, updated_at = ?3 WHERE id = ?4",
        params![name, slug_value, now, id],
    )?;
    let collection = require_collection(&library.conn, id)?;
    crate::sidecar::write_library(&library.paths, &library.conn)?;
    Ok(collection)
}

/// Delete `id` (design D3): its memberships go with it (`ON DELETE CASCADE`),
/// and no image otherwise changes. Deleting Favorites is allowed — it is an
/// ordinary collection once seeded (spec `collections`, "Every library has
/// Favorites from the start").
pub fn delete(library: &Library, id: &str) -> Result<()> {
    require_collection(&library.conn, id)?;
    let members: Vec<String> = {
        let mut stmt = library
            .conn
            .prepare("SELECT image_id FROM image_collections WHERE collection_id = ?1")?;
        stmt.query_map([id], |row| row.get(0))?
            .collect::<rusqlite::Result<_>>()?
    };

    library
        .conn
        .execute("DELETE FROM collections WHERE id = ?1", [id])?;

    crate::sidecar::write_for(&library.paths, &library.conn, &members)?;
    crate::sidecar::write_library(&library.paths, &library.conn)?;
    Ok(())
}

/// Pin or unpin `id` (`pinned-collections` design D3): `NotFound` naming the
/// id for one that does not exist, the twin of `tags::set_pinned`'s refusal —
/// keyed by id here because the collection's id is its identity and its name
/// is not. Rewrites `library.json`; never a sidecar (no image changed) and
/// never `updated_at` (a pin is a display preference, the same reason a
/// rename moves that time and a pin does not).
pub fn set_pinned(library: &Library, id: &str, pinned: bool) -> Result<Vec<Collection>> {
    require_collection(&library.conn, id)?;
    library.conn.execute(
        "UPDATE collections SET pinned = ?1 WHERE id = ?2",
        params![pinned, id],
    )?;
    crate::sidecar::write_library(&library.paths, &library.conn)?;
    list(&library.conn)
}

/// The transaction body of [`add`] (`stamps` design D2): idempotent per id
/// (`INSERT OR IGNORE`), no `images.updated_at` mark, since favouriting is
/// not an edit of the picture and must not reorder "Changed last" (design
/// D3). Split out at the transaction boundary, not copied, so
/// `tags::apply_edit` can run it inside the one transaction a stamp's whole
/// edit shares — a membership change is one part of an edit among several,
/// and opening a second transaction for it here would break the "one write"
/// guarantee the rest of the edit is under.
pub(crate) fn add_in(tx: &Connection, ids: &[String], collection_id: &str) -> Result<()> {
    let now = db::now_ms();
    for id in ids {
        tx.execute(
            "INSERT OR IGNORE INTO image_collections (image_id, collection_id, added_at)
             VALUES (?1, ?2, ?3)",
            params![id, collection_id, now],
        )?;
    }
    Ok(())
}

/// The transaction body of [`remove`], the same split as [`add_in`] and for
/// the same reason.
pub(crate) fn remove_in(tx: &Connection, ids: &[String], collection_id: &str) -> Result<()> {
    for id in ids {
        tx.execute(
            "DELETE FROM image_collections WHERE image_id = ?1 AND collection_id = ?2",
            params![id, collection_id],
        )?;
    }
    Ok(())
}

/// Put every id in `ids` into `collection_id` (design D3): idempotent per id,
/// one transaction. Answers with the written rows (design D8), so the caller
/// can `replace` them without a second `search`.
pub fn add(library: &Library, ids: &[String], collection_id: &str) -> Result<Vec<ImageRecord>> {
    require_collection(&library.conn, collection_id)?;
    if ids.is_empty() {
        return Ok(Vec::new());
    }

    let tx = library.conn.unchecked_transaction()?;
    add_in(&tx, ids, collection_id)?;
    tx.commit()?;

    // FIXME(collections D9): every bulk sidecar write in this module — here,
    // in `remove` and in `delete` — rewrites one file per id while holding the
    // library for the whole call, so adding a 25,000-image selection to
    // Favorites blocks a search or a capture until the last file is on disk.
    // The right shape is `library::backfill_sidecars`': take the library again
    // per item through `with_library`, checking the open root still matches,
    // so the pass is answerable between two writes. Not built because the
    // accepted `library-sidecars` cost (D9) is the same one every other bulk
    // write in the app already pays, and doing it here alone would leave the
    // app with two answers to the same question.
    crate::sidecar::write_for(&library.paths, &library.conn, ids)?;
    ingest::load_records(&library.conn, ids)
}

/// Take every id in `ids` out of `collection_id` (design D3): idempotent per
/// id, one transaction — the same rule [`add_in`] follows, for the same
/// reason. Answers with the written rows (design D8).
pub fn remove(library: &Library, ids: &[String], collection_id: &str) -> Result<Vec<ImageRecord>> {
    require_collection(&library.conn, collection_id)?;
    if ids.is_empty() {
        return Ok(Vec::new());
    }

    let tx = library.conn.unchecked_transaction()?;
    remove_in(&tx, ids, collection_id)?;
    tx.commit()?;

    crate::sidecar::write_for(&library.paths, &library.conn, ids)?;
    ingest::load_records(&library.conn, ids)
}

/// For exactly `collection_ids`, how many of `ids` are in each
/// (`pinned-collections` design D4): the pinned collection chip's tri-state
/// over a selection, the same shape `tags::selection_tag_counts` answers for
/// a pinned tag and for the same reason — a whole-library selection is past
/// SQLite's variable limit, so the query is chunked to [`ID_CHUNK`] ids with
/// the per-chunk counts summed in memory. Empty `ids` or empty
/// `collection_ids` answers empty without touching the database: neither
/// names anything to count. A collection none of `ids` is in is absent from
/// the answer, as a tag is from `selection_tag_counts`.
pub fn selection_counts(
    conn: &Connection,
    ids: &[String],
    collection_ids: &[String],
) -> Result<Vec<CollectionCount>> {
    if ids.is_empty() || collection_ids.is_empty() {
        return Ok(Vec::new());
    }

    let mut totals: std::collections::HashMap<String, (String, String, i64)> =
        std::collections::HashMap::new();
    for chunk in ids.chunks(ID_CHUNK) {
        let sql = format!(
            "SELECT collections.id, collections.name, collections.slug, COUNT(*)
             FROM image_collections
             JOIN collections ON collections.id = image_collections.collection_id
             WHERE image_collections.image_id IN ({})
               AND image_collections.collection_id IN ({})
             GROUP BY collections.id",
            placeholders(chunk.len()),
            placeholders(collection_ids.len())
        );
        let mut values = text_values(chunk);
        values.extend(text_values(collection_ids));

        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(params_from_iter(values), |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
            ))
        })?;
        for row in rows {
            let (id, name, slug, count) = row?;
            let entry = totals.entry(id).or_insert((name, slug, 0));
            entry.2 += count;
        }
    }

    let mut counts: Vec<CollectionCount> = totals
        .into_iter()
        .map(|(id, (name, slug, count))| CollectionCount {
            id,
            name,
            slug,
            count,
        })
        .collect();
    counts.sort_by_key(|count| count.name.to_lowercase());
    Ok(counts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ingest::{IngestInput, store_image};
    use crate::model::ImageSource;
    use crate::tags;

    fn library() -> (tempfile::TempDir, Library) {
        let dir = tempfile::tempdir().unwrap();
        let library = Library::open_or_create(dir.path()).unwrap();
        (dir, library)
    }

    fn png_bytes() -> Vec<u8> {
        let image = image::DynamicImage::ImageRgb8(image::RgbImage::new(2, 2));
        let mut out = std::io::Cursor::new(Vec::new());
        image.write_to(&mut out, image::ImageFormat::Png).unwrap();
        out.into_inner()
    }

    fn store(library: &Library, id: &str) {
        store_image(
            library,
            IngestInput {
                id,
                bytes: &png_bytes(),
                source: ImageSource::Local,
                source_ref: None,
                image_url: None,
                page_url: None,
                page_title: None,
                adapter: None,
                rating: None,
                tags: &[],
                captured_at: 1_700_000_000_000,
                file_modified_at: None,
                deleted_at: None,
            },
        )
        .unwrap();
    }

    fn favorites(library: &Library) -> Collection {
        list(&library.conn)
            .unwrap()
            .into_iter()
            .find(|collection| collection.id == "favorites")
            .unwrap()
    }

    // -- slug --------------------------------------------------------------

    #[test]
    fn slug_lower_cases_and_collapses_whitespace_to_underscores() {
        assert_eq!(slug("My  Favorites"), "my_favorites");
        assert_eq!(slug("  Queue  "), "queue");
        assert_eq!(slug("Queue"), "queue");
    }

    #[test]
    fn slug_of_a_blank_name_is_empty() {
        assert_eq!(slug(""), "");
        assert_eq!(slug("   "), "");
    }

    // -- id_for_slug (`stamps` design D2) -------------------------------------

    #[test]
    fn id_for_slug_finds_the_collection_that_slug_resolves_to() {
        let (_dir, library) = library();
        let created = create(&library, "To upload").unwrap();

        let id = id_for_slug(&library.conn, "to_upload").unwrap();

        assert_eq!(id, created.id);
    }

    #[test]
    fn id_for_slug_of_an_unknown_slug_is_refused_naming_it() {
        let (_dir, library) = library();

        let error = id_for_slug(&library.conn, "nope").unwrap_err();

        let AppError::BadRequest(reason) = error else {
            panic!("got {error}")
        };
        assert!(reason.contains("nope"), "{reason}");
    }

    // -- list / create -------------------------------------------------------

    #[test]
    fn a_fresh_library_lists_exactly_favorites() {
        let (_dir, library) = library();

        let collections = list(&library.conn).unwrap();

        assert_eq!(collections.len(), 1);
        assert_eq!(collections[0].id, "favorites");
        assert_eq!(collections[0].name, "Favorites");
        assert_eq!(collections[0].slug, "favorites");
    }

    #[test]
    fn create_stores_the_name_and_computes_the_slug() {
        let (_dir, library) = library();

        let created = create(&library, "To upload").unwrap();

        assert_eq!(created.name, "To upload");
        assert_eq!(created.slug, "to_upload");
        let names: Vec<String> = list(&library.conn)
            .unwrap()
            .into_iter()
            .map(|collection| collection.name)
            .collect();
        assert!(names.contains(&"To upload".to_string()));
    }

    #[test]
    fn a_blank_name_is_refused() {
        let (_dir, library) = library();

        let error = create(&library, "   ").unwrap_err();

        assert!(matches!(error, AppError::BadRequest(_)), "got {error}");
    }

    #[test]
    fn creating_a_name_that_clashes_by_slug_is_refused_naming_the_holder() {
        let (_dir, library) = library();
        create(&library, "Queue").unwrap();

        let error = create(&library, "queue").unwrap_err();

        let AppError::BadRequest(reason) = error else {
            panic!("got {error}")
        };
        assert!(reason.contains("Queue"), "{reason}");
    }

    #[test]
    fn create_rewrites_library_json() {
        let (_dir, library) = library();

        create(&library, "Queue").unwrap();

        let file =
            crate::sidecar::read_library(&crate::sidecar::library_path(&library.paths)).unwrap();
        assert!(
            file.collections
                .expect("the key is always written")
                .iter()
                .any(|collection| collection.name == "Queue")
        );
    }

    // -- rename ----------------------------------------------------------------

    #[test]
    fn rename_keeps_the_id_and_the_members() {
        let (_dir, library) = library();
        let created = create(&library, "To upload").unwrap();
        store(&library, "a");
        add(&library, &["a".to_string()], &created.id).unwrap();

        let renamed = rename(&library, &created.id, "Queue").unwrap();

        assert_eq!(renamed.id, created.id);
        assert_eq!(renamed.name, "Queue");
        assert_eq!(renamed.slug, "queue");
        let record = ingest::require_record(&library.conn, "a").unwrap();
        assert_eq!(record.collections, vec![created.id]);
    }

    #[test]
    fn rename_writes_no_sidecar() {
        let (_dir, library) = library();
        store(&library, "a");
        let favorites = favorites(&library);
        add(&library, &["a".to_string()], &favorites.id).unwrap();
        let before = std::fs::read(crate::sidecar::path(&library.paths, "a")).unwrap();

        rename(&library, &favorites.id, "Starred").unwrap();

        let after = std::fs::read(crate::sidecar::path(&library.paths, "a")).unwrap();
        assert_eq!(before, after, "renaming must not touch a member's sidecar");
    }

    #[test]
    fn renaming_to_a_name_that_clashes_is_refused() {
        let (_dir, library) = library();
        create(&library, "Queue").unwrap();
        let other = create(&library, "Other").unwrap();

        let error = rename(&library, &other.id, "queue").unwrap_err();

        assert!(matches!(error, AppError::BadRequest(_)), "got {error}");
    }

    #[test]
    fn renaming_to_the_same_name_is_not_a_clash_with_itself() {
        let (_dir, library) = library();
        let created = create(&library, "Queue").unwrap();

        let renamed = rename(&library, &created.id, "Queue").unwrap();

        assert_eq!(renamed.name, "Queue");
    }

    #[test]
    fn renaming_an_unknown_id_is_refused() {
        let (_dir, library) = library();

        let error = rename(&library, "no-such-id", "Queue").unwrap_err();

        assert!(matches!(error, AppError::NotFound(_)), "got {error}");
    }

    // -- delete ------------------------------------------------------------

    #[test]
    fn delete_removes_the_collection_and_the_memberships_and_rewrites_the_members_sidecars() {
        let (_dir, library) = library();
        let created = create(&library, "Queue").unwrap();
        store(&library, "a");
        store(&library, "b");
        add(&library, &["a".to_string(), "b".to_string()], &created.id).unwrap();

        delete(&library, &created.id).unwrap();

        assert!(
            list(&library.conn)
                .unwrap()
                .iter()
                .all(|collection| collection.id != created.id)
        );
        for id in ["a", "b"] {
            let record = ingest::require_record(&library.conn, id).unwrap();
            assert!(record.collections.is_empty());
            let sidecar = crate::sidecar::read(&crate::sidecar::path(&library.paths, id)).unwrap();
            assert!(sidecar.collections.is_empty());
        }
    }

    #[test]
    fn delete_leaves_the_images_tags_and_ratings_untouched() {
        let (_dir, library) = library();
        store(&library, "a");
        tags::update_tags(&library, "a", &["cat".to_string(), "rating:s".to_string()]).unwrap();
        let created = create(&library, "Queue").unwrap();
        add(&library, &["a".to_string()], &created.id).unwrap();

        delete(&library, &created.id).unwrap();

        let record = ingest::require_record(&library.conn, "a").unwrap();
        assert_eq!(record.tags, vec!["cat".to_string()]);
        assert_eq!(record.rating.as_deref(), Some("s"));
    }

    #[test]
    fn deleting_favorites_is_allowed() {
        let (_dir, library) = library();

        delete(&library, "favorites").unwrap();

        assert!(list(&library.conn).unwrap().is_empty());
    }

    #[test]
    fn deleting_an_unknown_id_is_refused() {
        let (_dir, library) = library();

        let error = delete(&library, "no-such-id").unwrap_err();

        assert!(matches!(error, AppError::NotFound(_)), "got {error}");
    }

    // -- add / remove --------------------------------------------------------

    #[test]
    fn add_puts_every_id_in_and_answers_with_the_written_records() {
        let (_dir, library) = library();
        store(&library, "a");
        store(&library, "b");
        let favorites = favorites(&library);

        let records = add(&library, &["a".to_string(), "b".to_string()], &favorites.id).unwrap();

        assert_eq!(records.len(), 2);
        for record in &records {
            assert_eq!(record.collections, vec![favorites.id.clone()]);
        }
    }

    #[test]
    fn adding_an_image_already_in_the_collection_changes_nothing() {
        let (_dir, library) = library();
        store(&library, "a");
        let favorites = favorites(&library);
        add(&library, &["a".to_string()], &favorites.id).unwrap();

        let records = add(&library, &["a".to_string()], &favorites.id).unwrap();

        assert_eq!(records[0].collections, vec![favorites.id]);
    }

    #[test]
    fn add_never_marks_updated_at() {
        let (_dir, library) = library();
        store(&library, "a");
        let before = ingest::require_record(&library.conn, "a")
            .unwrap()
            .updated_at;
        let favorites = favorites(&library);

        add(&library, &["a".to_string()], &favorites.id).unwrap();

        let after = ingest::require_record(&library.conn, "a")
            .unwrap()
            .updated_at;
        assert_eq!(before, after, "favouriting is not an edit of the picture");
    }

    #[test]
    fn add_rewrites_the_members_sidecars() {
        let (_dir, library) = library();
        store(&library, "a");
        let favorites = favorites(&library);

        add(&library, &["a".to_string()], &favorites.id).unwrap();

        let sidecar = crate::sidecar::read(&crate::sidecar::path(&library.paths, "a")).unwrap();
        assert_eq!(sidecar.collections, vec![favorites.id]);
    }

    #[test]
    fn add_to_an_unknown_collection_is_refused() {
        let (_dir, library) = library();
        store(&library, "a");

        let error = add(&library, &["a".to_string()], "no-such-id").unwrap_err();

        assert!(matches!(error, AppError::NotFound(_)), "got {error}");
    }

    #[test]
    fn remove_takes_every_id_out_and_answers_with_the_written_records() {
        let (_dir, library) = library();
        store(&library, "a");
        store(&library, "b");
        let favorites = favorites(&library);
        add(&library, &["a".to_string(), "b".to_string()], &favorites.id).unwrap();

        let records = remove(&library, &["a".to_string(), "b".to_string()], &favorites.id).unwrap();

        assert_eq!(records.len(), 2);
        for record in &records {
            assert!(record.collections.is_empty());
        }
    }

    #[test]
    fn removing_an_image_not_in_the_collection_changes_nothing() {
        let (_dir, library) = library();
        store(&library, "a");
        let favorites = favorites(&library);

        let records = remove(&library, &["a".to_string()], &favorites.id).unwrap();

        assert!(records[0].collections.is_empty());
    }

    #[test]
    fn remove_never_marks_updated_at() {
        let (_dir, library) = library();
        store(&library, "a");
        let favorites = favorites(&library);
        add(&library, &["a".to_string()], &favorites.id).unwrap();
        let before = ingest::require_record(&library.conn, "a")
            .unwrap()
            .updated_at;

        remove(&library, &["a".to_string()], &favorites.id).unwrap();

        let after = ingest::require_record(&library.conn, "a")
            .unwrap()
            .updated_at;
        assert_eq!(before, after);
    }

    #[test]
    fn remove_from_an_unknown_collection_is_refused() {
        let (_dir, library) = library();
        store(&library, "a");

        let error = remove(&library, &["a".to_string()], "no-such-id").unwrap_err();

        assert!(matches!(error, AppError::NotFound(_)), "got {error}");
    }

    // -- set_pinned (`pinned-collections` design D3) --------------------------

    #[test]
    fn set_pinned_toggles_and_answers_the_list() {
        let (_dir, library) = library();
        let created = create(&library, "Cute").unwrap();

        let answer = set_pinned(&library, &created.id, true).unwrap();
        assert!(
            answer
                .iter()
                .find(|collection| collection.id == created.id)
                .unwrap()
                .pinned
        );

        let answer = set_pinned(&library, &created.id, false).unwrap();
        assert!(
            !answer
                .iter()
                .find(|collection| collection.id == created.id)
                .unwrap()
                .pinned
        );
    }

    #[test]
    fn set_pinned_for_an_unknown_collection_is_refused() {
        let (_dir, library) = library();

        let error = set_pinned(&library, "no-such-id", true).unwrap_err();

        assert!(matches!(error, AppError::NotFound(_)), "got {error}");
    }

    #[test]
    fn set_pinned_rewrites_library_json_and_no_sidecar() {
        let (_dir, library) = library();
        store(&library, "a");
        let favorites = favorites(&library);
        add(&library, &["a".to_string()], &favorites.id).unwrap();
        let before = std::fs::read(crate::sidecar::path(&library.paths, "a")).unwrap();

        set_pinned(&library, &favorites.id, true).unwrap();

        let after = std::fs::read(crate::sidecar::path(&library.paths, "a")).unwrap();
        assert_eq!(before, after, "pinning must not touch a member's sidecar");
        let file =
            crate::sidecar::read_library(&crate::sidecar::library_path(&library.paths)).unwrap();
        let entry = file
            .collections
            .expect("the key is always written")
            .into_iter()
            .find(|collection| collection.id == favorites.id)
            .unwrap();
        assert!(entry.pinned);
    }

    #[test]
    fn set_pinned_leaves_updated_at_alone() {
        let (_dir, library) = library();
        let created = create(&library, "Cute").unwrap();
        library
            .conn
            .execute(
                "UPDATE collections SET updated_at = 0 WHERE id = ?1",
                params![created.id],
            )
            .unwrap();

        set_pinned(&library, &created.id, true).unwrap();

        let after = require_collection(&library.conn, &created.id).unwrap();
        assert_eq!(
            after.updated_at, 0,
            "a pin is not an edit of the collection"
        );
    }

    // -- selection_counts (`pinned-collections` design D4) --------------------

    #[test]
    fn selection_counts_counts_only_the_named_collections_over_the_ids() {
        let (_dir, library) = library();
        store(&library, "a");
        store(&library, "b");
        store(&library, "c");
        let cute = create(&library, "Cute").unwrap();
        let queue = create(&library, "Queue").unwrap();
        add(&library, &["a".to_string(), "b".to_string()], &cute.id).unwrap();
        add(&library, &["a".to_string()], &queue.id).unwrap();

        let counts = selection_counts(
            &library.conn,
            &["a".to_string(), "b".to_string(), "c".to_string()],
            &[cute.id.clone(), queue.id.clone()],
        )
        .unwrap();

        assert_eq!(
            counts,
            vec![
                CollectionCount {
                    id: cute.id.clone(),
                    name: "Cute".to_string(),
                    slug: "cute".to_string(),
                    count: 2,
                },
                CollectionCount {
                    id: queue.id.clone(),
                    name: "Queue".to_string(),
                    slug: "queue".to_string(),
                    count: 1,
                },
            ],
        );
    }

    #[test]
    fn selection_counts_leaves_out_a_collection_none_of_the_ids_is_in() {
        let (_dir, library) = library();
        store(&library, "a");
        let cute = create(&library, "Cute").unwrap();
        let empty = create(&library, "Empty").unwrap();

        let counts = selection_counts(
            &library.conn,
            &["a".to_string()],
            &[cute.id.clone(), empty.id.clone()],
        )
        .unwrap();

        assert!(
            counts.iter().all(|count| count.id != empty.id),
            "{counts:?}"
        );
    }

    #[test]
    fn selection_counts_with_no_ids_or_no_collections_is_empty() {
        let (_dir, library) = library();
        store(&library, "a");
        let cute = create(&library, "Cute").unwrap();
        add(&library, &["a".to_string()], &cute.id).unwrap();

        assert!(
            selection_counts(&library.conn, &[], std::slice::from_ref(&cute.id))
                .unwrap()
                .is_empty()
        );
        assert!(
            selection_counts(&library.conn, &["a".to_string()], &[])
                .unwrap()
                .is_empty()
        );
    }

    /// A selection this large crosses more than one chunk of the `IN (…)`
    /// query, so the totals have to be summed across chunks to be right — the
    /// shape of `tags::selection_tag_counts_sums_across_chunks_past_the_sqlite_variable_chunk_size`.
    #[test]
    fn selection_counts_reaches_every_id_past_the_sqlite_variable_chunk_size() {
        let (_dir, library) = library();
        let cute = create(&library, "Cute").unwrap();
        let ids: Vec<String> = (0..2500).map(|index| format!("img-{index}")).collect();
        for id in &ids {
            store(&library, id);
        }
        add(&library, &ids, &cute.id).unwrap();

        let counts = selection_counts(&library.conn, &ids, std::slice::from_ref(&cute.id)).unwrap();

        assert_eq!(
            counts,
            vec![CollectionCount {
                id: cute.id,
                name: "Cute".to_string(),
                slug: "cute".to_string(),
                count: 2500,
            }],
        );
    }
}
