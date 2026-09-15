//! The SQLite side of the library: opening a connection with the pragmas the
//! design fixes, and the migration runner keyed on `PRAGMA user_version`.

use std::path::Path;

use rusqlite::Connection;

use crate::error::{AppError, Result};

/// Schema v1 (design D2). `posts` is created now so §6's `posted:` filter has a
/// home; Phase 1 never writes it.
///
/// `images_fts` is an FTS5 external-content table: it stores no copy of the
/// text, only the index, and the three triggers below are the whole reason it
/// stays in step with `images`. A write that changes `page_title`, `page_url`
/// or `image_url` without going through them silently rots the search index —
/// `rebuild_fts` is the way back.
const SCHEMA_V1: &str = r"
CREATE TABLE images (
    id          TEXT PRIMARY KEY,
    ext         TEXT NOT NULL,
    mime        TEXT NOT NULL,
    size        INTEGER NOT NULL,
    width       INTEGER NOT NULL,
    height      INTEGER NOT NULL,
    source      TEXT NOT NULL,
    source_ref  TEXT,
    image_url   TEXT,
    page_url    TEXT,
    page_title  TEXT,
    rating      TEXT,
    captured_at INTEGER NOT NULL,
    created_at  INTEGER NOT NULL,
    updated_at  INTEGER NOT NULL,
    deleted_at  INTEGER,
    missing     INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX images_created_at ON images (created_at DESC);
CREATE INDEX images_source ON images (source);

-- The grid's one ordering, tie-break column included, so ten thousand images
-- page without sorting the table. Part of v1 rather than a v2 migration
-- because v1 has not shipped: there is no database anywhere that has the
-- table without the index.
CREATE INDEX images_captured_at ON images (captured_at DESC, id DESC);

CREATE TABLE tags (
    id   INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE
);

CREATE TABLE image_tags (
    image_id TEXT NOT NULL REFERENCES images (id) ON DELETE CASCADE,
    tag_id   INTEGER NOT NULL REFERENCES tags (id),
    PRIMARY KEY (image_id, tag_id)
);

CREATE INDEX image_tags_by_tag ON image_tags (tag_id, image_id);

CREATE TABLE posts (
    image_id  TEXT NOT NULL REFERENCES images (id) ON DELETE CASCADE,
    site      TEXT NOT NULL,
    remote_id TEXT,
    posted_at INTEGER,
    PRIMARY KEY (image_id, site)
);

CREATE VIRTUAL TABLE images_fts USING fts5 (
    page_title,
    page_url,
    image_url,
    content = 'images',
    content_rowid = 'rowid'
);

CREATE TRIGGER images_fts_insert AFTER INSERT ON images BEGIN
    INSERT INTO images_fts (rowid, page_title, page_url, image_url)
    VALUES (new.rowid, new.page_title, new.page_url, new.image_url);
END;

CREATE TRIGGER images_fts_delete AFTER DELETE ON images BEGIN
    INSERT INTO images_fts (images_fts, rowid, page_title, page_url, image_url)
    VALUES ('delete', old.rowid, old.page_title, old.page_url, old.image_url);
END;

CREATE TRIGGER images_fts_update AFTER UPDATE ON images BEGIN
    INSERT INTO images_fts (images_fts, rowid, page_title, page_url, image_url)
    VALUES ('delete', old.rowid, old.page_title, old.page_url, old.image_url);
    INSERT INTO images_fts (rowid, page_title, page_url, image_url)
    VALUES (new.rowid, new.page_title, new.page_url, new.image_url);
END;
";

/// Schema v2 (design D11): the site-adapter record the capturing client sent,
/// stored as it arrived.
///
/// Named `_json` because that is what the next reader has to know about it: it
/// holds a document, not a value to compare with `=`, and the reader that wants
/// one field out of it reaches in with SQLite's JSON functions. No FTS change —
/// the adapter fields are storage in this schema, not search.
const SCHEMA_V2: &str = r"
ALTER TABLE images ADD COLUMN adapter_json TEXT;
";

/// Schema v3 (`browse-polish` design D11): the file's own modification time,
/// kept apart from `captured_at` now that capture time is always the import
/// moment. Nullable — most images never came from a file the user had, and
/// `NULL` is "there is no such fact" rather than an invented measurement. Not
/// indexed: it is not a sort key (D11).
const SCHEMA_V3: &str = r"
ALTER TABLE images ADD COLUMN file_modified_at INTEGER;
";

/// Schema v4 (`auto-tag-rules` design D1). This change owns v4, not v3: v1 is
/// `phase-1-app-mvp`, v2 is `bridge-extension` (`adapter_json`), and
/// `browse-polish` already took v3 (`file_modified_at`) before this change was
/// implemented — the design's sketch named v3, written before that shipped.
///
/// `rules.id` is `TEXT` because rule ids come from `uuid::Uuid::new_v4()`, the
/// same as image ids, and because the JSON import/export shape carries string
/// ids (design D10). No index on either table: `rules` holds tens of rows and
/// every read is "all of them", and `notes` is one row by its own `CHECK`.
///
/// `rules.tags_json` is a document, not a value ever compared with `=` — named
/// `_json` for the reason `adapter_json` was (schema v2's comment).
const SCHEMA_V4: &str = r"
CREATE TABLE rules (
    id         TEXT PRIMARY KEY,
    name       TEXT NOT NULL,
    pattern    TEXT NOT NULL,
    is_regex   INTEGER NOT NULL,
    tags_json  TEXT NOT NULL,
    enabled    INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE notes (
    id         INTEGER PRIMARY KEY CHECK (id = 1),
    content    TEXT NOT NULL,
    updated_at INTEGER NOT NULL
);
";

/// Schema v5 (`booru-upload` design D1). This change owns v5, not the v4 its
/// own design doc sketched before landing order was known: v1 is
/// `phase-1-app-mvp`, v2 is `bridge-extension` (`adapter_json`), v3 is
/// `browse-polish` (`file_modified_at`), and v4 is `auto-tag-rules` (`rules`,
/// `notes`) — `tags-and-ratings` landed between them needing no migration of
/// its own, so this is the fifth entry, not the fourth.
///
/// No `REFERENCES` to `images`: design D2 spells out why `posts.site` already
/// holds no foreign key to this table either — a post record must outlive the
/// site it was made against, and `base_url UNIQUE` alone is what "two sites
/// SHALL NOT share a base address" (`booru-sites`) needs enforced in SQL.
const SCHEMA_V5: &str = r"
CREATE TABLE booru_sites (
    id         TEXT PRIMARY KEY,
    name       TEXT NOT NULL,
    base_url   TEXT NOT NULL UNIQUE,
    username   TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);
";

/// Schema v6 (`collections` design D1): named, unordered sets of images,
/// separate from tags (§6 — a collection is never uploaded, a tag is). The
/// seed runs inside the migration itself, which is what makes it run exactly
/// once per database, for a fresh library and one upgraded from an older
/// version alike, and never again: "deleted stays deleted" (spec
/// `collections`) falls out of a migration never re-running rather than out of
/// a check at open time. The fixed id `favorites` is what lets `library.json`
/// and this seed agree on a rebuild (design D5).
///
/// `<now>` is spelled in SQL rather than bound as a parameter: the migration
/// text has no Rust `now_ms` to call, and SQLite can spell the same
/// milliseconds-since-epoch unit itself. Uniqueness is on `slug`, not `name`:
/// `Queue` and `queue` are one collection to a query (design D1).
const SCHEMA_V6: &str = r"
CREATE TABLE collections (
    id         TEXT PRIMARY KEY,
    name       TEXT NOT NULL,
    slug       TEXT NOT NULL UNIQUE,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE image_collections (
    image_id      TEXT NOT NULL REFERENCES images (id) ON DELETE CASCADE,
    collection_id TEXT NOT NULL REFERENCES collections (id) ON DELETE CASCADE,
    added_at      INTEGER NOT NULL,
    PRIMARY KEY (image_id, collection_id)
);

CREATE INDEX image_collections_by_collection ON image_collections (collection_id, image_id);

INSERT INTO collections (id, name, slug, created_at, updated_at)
VALUES ('favorites', 'Favorites', 'favorites',
        CAST(strftime('%s','now') AS INTEGER) * 1000,
        CAST(strftime('%s','now') AS INTEGER) * 1000);
";

/// One entry per schema version, applied in order. Appending is the only way to
/// change the schema: `user_version` counts how many of these have run.
const MIGRATIONS: &[&str] = &[
    SCHEMA_V1, SCHEMA_V2, SCHEMA_V3, SCHEMA_V4, SCHEMA_V5, SCHEMA_V6,
];

/// Open (creating if needed) the library database with the pragmas D2 fixes,
/// migrate it to the current schema, and register the SQL functions the query
/// compiler needs.
pub fn open(path: &Path) -> Result<Connection> {
    let mut conn = Connection::open(path).map_err(|error| AppError::from_rusqlite(path, error))?;
    quick_check(&conn, path)?;
    assert_rollback_journal(&conn)?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    migrate(&mut conn)?;
    crate::query::register_functions(&conn)?;
    Ok(conn)
}

/// `PRAGMA quick_check(1)` before anything else touches the file (design D8):
/// it reads every page and verifies each b-tree, and `(1)` stops at the first
/// problem because one problem is the whole answer. A file so damaged it is
/// not a database at all fails the query itself; a `quick_check` that runs but
/// answers with anything but `"ok"` is damage `quick_check` could name in
/// detail but this app only ever reports as one thing: `LibraryCorrupt`.
fn quick_check(conn: &Connection, path: &Path) -> Result<()> {
    let result: String = conn
        .query_row("PRAGMA quick_check(1)", [], |row| row.get(0))
        .map_err(|error| AppError::from_rusqlite(path, error))?;
    if result == "ok" {
        Ok(())
    } else {
        Err(AppError::LibraryCorrupt {
            path: path.to_path_buf(),
        })
    }
}

/// WAL splits the library across `library.sqlite`, `-wal` and `-shm`. A cloud
/// sync client that copies only the first restores a torn database, which is
/// why §7 fixes the rollback journal. Setting it is not enough: an existing
/// file already in WAL keeps its mode when the change cannot be applied, so the
/// answer is read back. Delete this check and a WAL library opens silently.
fn assert_rollback_journal(conn: &Connection) -> Result<()> {
    let mode: String = conn.query_row("PRAGMA journal_mode = DELETE", [], |row| row.get(0))?;
    if mode.eq_ignore_ascii_case("delete") {
        Ok(())
    } else {
        Err(AppError::JournalMode { mode })
    }
}

fn migrate(conn: &mut Connection) -> Result<()> {
    let applied: i64 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    let known = MIGRATIONS.len() as i64;
    if applied > known {
        return Err(AppError::SchemaTooNew {
            found: applied,
            known,
        });
    }
    for (index, sql) in MIGRATIONS.iter().enumerate().skip(applied as usize) {
        let tx = conn.transaction()?;
        tx.execute_batch(sql)?;
        tx.pragma_update(None, "user_version", index as i64 + 1)?;
        tx.commit()?;
    }
    Ok(())
}

/// Rebuild the FTS index from `images`. The external-content table can drift
/// from its source if a write ever bypasses the triggers; this is the way back,
/// and it exists from day one because a search index that cannot be repaired
/// makes the whole library suspect.
pub fn rebuild_fts(conn: &Connection) -> Result<()> {
    conn.execute("INSERT INTO images_fts (images_fts) VALUES ('rebuild')", [])?;
    Ok(())
}

/// Epoch milliseconds, the unit every timestamp column and the shared contract
/// use.
pub fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|since| since.as_millis() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_db() -> (tempfile::TempDir, Connection) {
        let dir = tempfile::tempdir().unwrap();
        let conn = open(&dir.path().join("library.sqlite")).unwrap();
        (dir, conn)
    }

    fn table_names(conn: &Connection) -> Vec<String> {
        let mut stmt = conn
            .prepare("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")
            .unwrap();
        let names = stmt.query_map([], |row| row.get::<_, String>(0)).unwrap();
        names.map(std::result::Result::unwrap).collect()
    }

    fn column_names(conn: &Connection, table: &str) -> Vec<String> {
        let mut stmt = conn
            .prepare(&format!("SELECT name FROM pragma_table_info('{table}')"))
            .unwrap();
        let names = stmt.query_map([], |row| row.get::<_, String>(0)).unwrap();
        names.map(std::result::Result::unwrap).collect()
    }

    #[test]
    fn open_applies_every_migration() {
        let (_dir, conn) = temp_db();

        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, MIGRATIONS.len() as i64);

        let names = table_names(&conn);
        for expected in ["images", "tags", "image_tags", "posts", "images_fts"] {
            assert!(
                names.contains(&expected.to_string()),
                "missing table {expected}: {names:?}"
            );
        }
        assert!(
            column_names(&conn, "images").contains(&"adapter_json".to_string()),
            "schema v2 did not add adapter_json"
        );
        assert!(
            column_names(&conn, "images").contains(&"file_modified_at".to_string()),
            "schema v3 did not add file_modified_at"
        );
        for expected in ["rules", "notes", "booru_sites"] {
            assert!(
                names.contains(&expected.to_string()),
                "missing table {expected}: {names:?}"
            );
        }
        for expected in ["collections", "image_collections"] {
            assert!(
                names.contains(&expected.to_string()),
                "missing table {expected}: {names:?}"
            );
        }
        assert_eq!(
            favorites_row(&conn),
            Some(("Favorites".to_string(), "favorites".to_string()))
        );
    }

    /// The seed row's name and slug, or `None` when `collections` has no row
    /// with the fixed id the migration inserts.
    fn favorites_row(conn: &Connection) -> Option<(String, String)> {
        match conn.query_row(
            "SELECT name, slug FROM collections WHERE id = 'favorites'",
            [],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        ) {
            Ok(row) => Some(row),
            Err(rusqlite::Error::QueryReturnedNoRows) => None,
            Err(error) => panic!("{error}"),
        }
    }

    /// A library written by the shipped v1 build has to reach the current
    /// version with its rows, which is the whole point of appending to
    /// `MIGRATIONS` rather than editing `SCHEMA_V1`: edit v1 and this database
    /// never gets the columns later versions add.
    #[test]
    fn a_v1_library_migrates_forward_keeping_its_rows() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("library.sqlite");

        let conn = Connection::open(&path).unwrap();
        conn.execute_batch(SCHEMA_V1).unwrap();
        conn.pragma_update(None, "user_version", 1i64).unwrap();
        insert_bare_image(&conn, "a", "sunset over kyoto");
        drop(conn);

        let conn = open(&path).unwrap();

        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, MIGRATIONS.len() as i64);
        assert!(column_names(&conn, "images").contains(&"adapter_json".to_string()));
        assert!(column_names(&conn, "images").contains(&"file_modified_at".to_string()));

        let adapter: Option<String> = conn
            .query_row(
                "SELECT adapter_json FROM images WHERE id = 'a'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            adapter, None,
            "an existing row reads the new column as NULL"
        );
        assert_eq!(fts_matches(&conn, "kyoto"), vec!["a".to_string()]);
    }

    /// A library written after `bridge-extension` shipped (v2, no
    /// `file_modified_at`) has to reach v3 with its rows intact and `NULL` in
    /// the new column — `browse-polish` D12 backfills nothing.
    #[test]
    fn a_v2_library_migrates_to_v3_keeping_its_rows() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("library.sqlite");

        let conn = Connection::open(&path).unwrap();
        conn.execute_batch(SCHEMA_V1).unwrap();
        conn.execute_batch(SCHEMA_V2).unwrap();
        conn.pragma_update(None, "user_version", 2i64).unwrap();
        insert_bare_image(&conn, "a", "sunset over kyoto");
        drop(conn);

        let conn = open(&path).unwrap();

        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, MIGRATIONS.len() as i64);
        assert!(column_names(&conn, "images").contains(&"file_modified_at".to_string()));

        let file_modified_at: Option<i64> = conn
            .query_row(
                "SELECT file_modified_at FROM images WHERE id = 'a'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            file_modified_at, None,
            "an existing row reads the new column as NULL"
        );
        assert_eq!(fts_matches(&conn, "kyoto"), vec!["a".to_string()]);
    }

    /// A library written after `browse-polish` shipped (v3, no `rules` or
    /// `notes` tables) has to reach v4 with its rows intact — `auto-tag-rules`
    /// adds nothing to `images`, only the two new tables.
    #[test]
    fn a_v3_library_migrates_to_v4_keeping_its_rows() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("library.sqlite");

        let conn = Connection::open(&path).unwrap();
        conn.execute_batch(SCHEMA_V1).unwrap();
        conn.execute_batch(SCHEMA_V2).unwrap();
        conn.execute_batch(SCHEMA_V3).unwrap();
        conn.pragma_update(None, "user_version", 3i64).unwrap();
        insert_bare_image(&conn, "a", "sunset over kyoto");
        drop(conn);

        let conn = open(&path).unwrap();

        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, MIGRATIONS.len() as i64);
        for expected in ["rules", "notes"] {
            assert!(table_names(&conn).contains(&expected.to_string()));
        }
        assert_eq!(fts_matches(&conn, "kyoto"), vec!["a".to_string()]);
    }

    /// A library written after `auto-tag-rules` shipped (v4, no `booru_sites`
    /// table) has to reach v5 with its rows intact — `booru-upload` adds
    /// nothing to any existing table, only this one.
    #[test]
    fn a_v4_library_migrates_to_v5_keeping_its_rows() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("library.sqlite");

        let conn = Connection::open(&path).unwrap();
        conn.execute_batch(SCHEMA_V1).unwrap();
        conn.execute_batch(SCHEMA_V2).unwrap();
        conn.execute_batch(SCHEMA_V3).unwrap();
        conn.execute_batch(SCHEMA_V4).unwrap();
        conn.pragma_update(None, "user_version", 4i64).unwrap();
        insert_bare_image(&conn, "a", "sunset over kyoto");
        drop(conn);

        let conn = open(&path).unwrap();

        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, MIGRATIONS.len() as i64);
        assert!(table_names(&conn).contains(&"booru_sites".to_string()));
        assert_eq!(fts_matches(&conn, "kyoto"), vec!["a".to_string()]);
    }

    /// A library written after `booru-upload` shipped (v5, no `collections`
    /// table) has to reach v6 with its rows intact and exactly one collection,
    /// the seed (task 1.1).
    #[test]
    fn a_v5_library_migrates_to_v6_keeping_its_rows_and_seeding_favorites_once() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("library.sqlite");

        let conn = Connection::open(&path).unwrap();
        conn.execute_batch(SCHEMA_V1).unwrap();
        conn.execute_batch(SCHEMA_V2).unwrap();
        conn.execute_batch(SCHEMA_V3).unwrap();
        conn.execute_batch(SCHEMA_V4).unwrap();
        conn.execute_batch(SCHEMA_V5).unwrap();
        conn.pragma_update(None, "user_version", 5i64).unwrap();
        insert_bare_image(&conn, "a", "sunset over kyoto");
        drop(conn);

        let conn = open(&path).unwrap();

        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, MIGRATIONS.len() as i64);
        assert!(table_names(&conn).contains(&"collections".to_string()));
        assert_eq!(
            favorites_row(&conn),
            Some(("Favorites".to_string(), "favorites".to_string()))
        );
        let collection_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM collections", [], |row| row.get(0))
            .unwrap();
        assert_eq!(
            collection_count, 1,
            "an upgraded library has exactly one collection, the seed"
        );
        assert_eq!(fts_matches(&conn, "kyoto"), vec!["a".to_string()]);
    }

    /// A fresh library gets the same one seeded collection a migrated one
    /// does (task 1.1) — the migration runs for `Library::open_or_create`
    /// exactly as it does for an upgrade, since both go through `open`.
    #[test]
    fn a_fresh_library_has_exactly_one_collection_favorites() {
        let (_dir, conn) = temp_db();

        let collection_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM collections", [], |row| row.get(0))
            .unwrap();
        assert_eq!(collection_count, 1);
        assert_eq!(
            favorites_row(&conn),
            Some(("Favorites".to_string(), "favorites".to_string()))
        );
    }

    /// Opening twice must not reseed: the migration runs once, and
    /// `reopening_does_not_reapply_migrations` already pins that the whole
    /// migration list does not rerun on a second open, so this only has to
    /// prove the deleted case (spec `collections`, "Deleted stays deleted").
    #[test]
    fn deleting_favorites_and_reopening_does_not_bring_it_back() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("library.sqlite");
        let conn = open(&path).unwrap();
        conn.execute("DELETE FROM collections WHERE id = 'favorites'", [])
            .unwrap();
        drop(conn);

        let conn = open(&path).unwrap();

        assert_eq!(favorites_row(&conn), None);
    }

    #[test]
    fn a_second_row_in_notes_is_refused() {
        let (_dir, conn) = temp_db();
        conn.execute(
            "INSERT INTO notes (id, content, updated_at) VALUES (1, 'first', 0)",
            [],
        )
        .unwrap();

        let error = conn
            .execute(
                "INSERT INTO notes (id, content, updated_at) VALUES (2, 'second', 0)",
                [],
            )
            .unwrap_err();

        assert!(
            error.to_string().contains("CHECK"),
            "expected a CHECK constraint failure, got {error}"
        );
    }

    #[test]
    fn open_uses_the_rollback_journal() {
        let (_dir, conn) = temp_db();

        let mode: String = conn
            .query_row("PRAGMA journal_mode", [], |row| row.get(0))
            .unwrap();
        assert_eq!(mode, "delete");
    }

    #[test]
    fn reopening_does_not_reapply_migrations() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("library.sqlite");
        drop(open(&path).unwrap());

        let conn = open(&path).unwrap();
        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, MIGRATIONS.len() as i64);
    }

    /// design D8: bytes overwritten past the header trip `quick_check`, which
    /// refuses the open as `LibraryCorrupt` and leaves the file exactly as
    /// `open` found it — a refused open must never rewrite the very evidence
    /// a rebuild would need.
    #[test]
    fn a_database_with_bytes_overwritten_mid_file_is_refused_as_library_corrupt() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("library.sqlite");
        drop(open(&path).unwrap());

        let mut bytes = std::fs::read(&path).unwrap();
        assert!(
            bytes.len() > 4096,
            "the schema must have grown past the first page: {} bytes",
            bytes.len()
        );
        // Well past the header (`SQLite format 3\0`, the first 16 bytes) and
        // into a later page, so this corrupts a b-tree rather than turning the
        // file into something SQLite refuses to recognise as a database at
        // all — the other corrupt-class code, exercised in `error.rs`.
        for byte in &mut bytes[4096..4200] {
            *byte ^= 0xff;
        }
        std::fs::write(&path, &bytes).unwrap();

        let error = open(&path).unwrap_err();

        assert!(
            matches!(&error, AppError::LibraryCorrupt { path: found } if found == &path),
            "unexpected error: {error}"
        );
        assert_eq!(
            std::fs::read(&path).unwrap(),
            bytes,
            "a refused open must not touch the file"
        );
    }

    #[test]
    fn rejects_a_library_from_a_newer_build() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("library.sqlite");
        let conn = open(&path).unwrap();
        conn.pragma_update(None, "user_version", 99i64).unwrap();
        drop(conn);

        let error = open(&path).unwrap_err();
        assert!(matches!(
            error,
            AppError::SchemaTooNew { found: 99, known } if known == MIGRATIONS.len() as i64
        ));
    }

    fn insert_bare_image(conn: &Connection, id: &str, title: &str) {
        conn.execute(
            "INSERT INTO images (id, ext, mime, size, width, height, source, page_title,
                                 captured_at, created_at, updated_at)
             VALUES (?1, 'png', 'image/png', 1, 1, 1, 'local', ?2, 0, 0, 0)",
            rusqlite::params![id, title],
        )
        .unwrap();
    }

    fn fts_matches(conn: &Connection, needle: &str) -> Vec<String> {
        let mut stmt = conn
            .prepare(
                "SELECT images.id FROM images_fts
                 JOIN images ON images.rowid = images_fts.rowid
                 WHERE images_fts MATCH ?1",
            )
            .unwrap();
        let ids = stmt
            .query_map([needle], |row| row.get::<_, String>(0))
            .unwrap();
        ids.map(std::result::Result::unwrap).collect()
    }

    #[test]
    fn triggers_keep_the_search_index_in_step() {
        let (_dir, conn) = temp_db();
        insert_bare_image(&conn, "a", "sunset over kyoto");

        assert_eq!(fts_matches(&conn, "kyoto"), vec!["a".to_string()]);

        conn.execute(
            "UPDATE images SET page_title = 'sunrise over osaka' WHERE id = 'a'",
            [],
        )
        .unwrap();
        assert!(fts_matches(&conn, "kyoto").is_empty());
        assert_eq!(fts_matches(&conn, "osaka"), vec!["a".to_string()]);

        conn.execute("DELETE FROM images WHERE id = 'a'", [])
            .unwrap();
        assert!(fts_matches(&conn, "osaka").is_empty());
    }

    #[test]
    fn rebuild_restores_an_index_that_drifted() {
        let (_dir, conn) = temp_db();
        insert_bare_image(&conn, "a", "sunset over kyoto");
        conn.execute(
            "INSERT INTO images_fts (images_fts) VALUES ('delete-all')",
            [],
        )
        .unwrap();
        assert!(fts_matches(&conn, "kyoto").is_empty());

        rebuild_fts(&conn).unwrap();
        assert_eq!(fts_matches(&conn, "kyoto"), vec!["a".to_string()]);
    }
}
