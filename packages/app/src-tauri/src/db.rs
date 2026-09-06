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

/// One entry per schema version, applied in order. Appending is the only way to
/// change the schema: `user_version` counts how many of these have run.
const MIGRATIONS: &[&str] = &[SCHEMA_V1];

/// Open (creating if needed) the library database with the pragmas D2 fixes,
/// migrate it to the current schema, and register the SQL functions the query
/// compiler needs.
pub fn open(path: &Path) -> Result<Connection> {
    let mut conn = Connection::open(path)?;
    assert_rollback_journal(&conn)?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    migrate(&mut conn)?;
    crate::query::register_functions(&conn)?;
    Ok(conn)
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

    #[test]
    fn open_applies_schema_v1() {
        let (_dir, conn) = temp_db();

        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, 1);

        let names = table_names(&conn);
        for expected in ["images", "tags", "image_tags", "posts", "images_fts"] {
            assert!(
                names.contains(&expected.to_string()),
                "missing table {expected}: {names:?}"
            );
        }
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
        assert_eq!(version, 1);
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
            AppError::SchemaTooNew {
                found: 99,
                known: 1
            }
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
