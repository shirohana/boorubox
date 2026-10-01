//! Stamps: saved edits, written once in the tag language and applied to an
//! image by a click in edit mode (`stamps` design D1, D3). This module is
//! only the store — `list`, `upsert`, `reorder`, `delete` — shaped like `rules.rs`'s,
//! without the matching or the import/export halves a stamp does not need:
//! a stamp's text is never matched against anything, only parsed by the
//! webview's grammar and, once parsed, applied through `tags::apply_edit`.

use rusqlite::{Connection, Row, params};

use crate::db;
use crate::error::{AppError, Result};
use crate::library::Library;
use crate::model::{Stamp, StampInput};

const STAMP_COLUMNS: &str = "id, name, text, created_at, updated_at";

fn row_to_stamp(row: &Row) -> rusqlite::Result<Stamp> {
    Ok(Stamp {
        id: row.get(0)?,
        name: row.get(1)?,
        text: row.get(2)?,
        created_at: row.get(3)?,
        updated_at: row.get(4)?,
    })
}

/// Every stamp, in the user's order (`stamp-order` design D1): the stamp bar
/// and the settings table both read this one list.
///
/// Ordered by `position, created_at, rowid` rather than ending in `id`: `id`
/// is a random uuid, so two rows tied on the earlier keys would come back in
/// random order on every read — a bar that reshuffles itself between
/// sessions. `rowid` is monotonic on insert (SQLite's own row-creation order
/// for a table with no `WITHOUT ROWID`).
pub fn list(conn: &Connection) -> Result<Vec<Stamp>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {STAMP_COLUMNS} FROM stamps ORDER BY position, created_at, rowid"
    ))?;
    let rows = stmt.query_map([], row_to_stamp)?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

fn require_stamp(conn: &Connection, id: &str) -> Result<Stamp> {
    let mut stmt = conn.prepare(&format!("SELECT {STAMP_COLUMNS} FROM stamps WHERE id = ?1"))?;
    match stmt.query_row([id], row_to_stamp) {
        Ok(stamp) => Ok(stamp),
        Err(rusqlite::Error::QueryReturnedNoRows) => Err(AppError::NotFound(format!("stamp {id}"))),
        Err(error) => Err(error.into()),
    }
}

/// Create a stamp when `input.id` is absent, or edit the one it names (design
/// D3). Refuses an empty name and an empty text, both judged after trimming;
/// the name is stored trimmed, the same rule `rules::upsert` follows for its
/// own name, but the text is stored exactly as typed — the grammar that reads
/// it is the webview's (design D1), and storing anything but what the user
/// wrote would freeze a stamp they meant to keep editing.
pub fn upsert(library: &Library, input: &StampInput) -> Result<Stamp> {
    let name = input.name.trim();
    if name.is_empty() {
        return Err(AppError::BadRequest("a stamp needs a name".to_string()));
    }
    if input.text.trim().is_empty() {
        return Err(AppError::BadRequest("a stamp needs a text".to_string()));
    }
    let text = &input.text;

    let now = db::now_ms();
    let stamp = match &input.id {
        Some(id) => {
            require_stamp(&library.conn, id)?;
            library.conn.execute(
                "UPDATE stamps SET name = ?1, text = ?2, updated_at = ?3 WHERE id = ?4",
                params![name, text, now, id],
            )?;
            require_stamp(&library.conn, id)?
        }
        None => {
            let id = uuid::Uuid::new_v4().to_string();
            library.conn.execute(
                "INSERT INTO stamps (id, name, text, created_at, updated_at, position)
                 VALUES (?1, ?2, ?3, ?4, ?4, (SELECT COALESCE(MAX(position), 0) + 1 FROM stamps))",
                params![id, name, text, now],
            )?;
            require_stamp(&library.conn, &id)?
        }
    };
    // Library-level, after the commit (`library-sidecars` design D3, D5): a
    // single `execute` outside a transaction is already its own commit.
    crate::sidecar::write_library(&library.paths, &library.conn)?;
    Ok(stamp)
}

/// Set every stamp's place to its index in `ids` and answer the list as it
/// now stands (`stamp-order` design D3). `ids` must be exactly the library's
/// stamps — a duplicate, a missing id or an unknown id is refused with nothing
/// written, because a partial order would interleave with the positions it did
/// not name in a way the user never saw.
pub fn reorder(library: &Library, ids: &[String]) -> Result<Vec<Stamp>> {
    let mut held: Vec<String> = list(&library.conn)?
        .into_iter()
        .map(|stamp| stamp.id)
        .collect();
    let mut asked: Vec<String> = ids.to_vec();
    held.sort();
    asked.sort();
    if held != asked {
        return Err(AppError::BadRequest(
            "the new order must list every stamp exactly once".to_string(),
        ));
    }

    let tx = library.conn.unchecked_transaction()?;
    for (index, id) in ids.iter().enumerate() {
        tx.execute(
            "UPDATE stamps SET position = ?1 WHERE id = ?2",
            params![index as i64 + 1, id],
        )?;
    }
    tx.commit()?;
    crate::sidecar::write_library(&library.paths, &library.conn)?;
    list(&library.conn)
}

/// Delete a stamp. Idempotent — one already gone is the same outcome as one
/// deleted now — and no image it was ever applied to is touched: a stamp is
/// an instruction, not a record of having been used.
pub fn delete(library: &Library, id: &str) -> Result<()> {
    library
        .conn
        .execute("DELETE FROM stamps WHERE id = ?1", [id])?;
    crate::sidecar::write_library(&library.paths, &library.conn)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn library() -> (tempfile::TempDir, Library) {
        let dir = tempfile::tempdir().unwrap();
        let library = Library::open_or_create(dir.path()).unwrap();
        (dir, library)
    }

    fn new_stamp(name: &str, text: &str) -> StampInput {
        StampInput {
            id: None,
            name: name.to_string(),
            text: text.to_string(),
        }
    }

    #[test]
    fn upsert_round_trips_a_stamp() {
        let (_dir, library) = library();

        let created = upsert(&library, &new_stamp("Cat", "cat animal")).unwrap();

        let listed = list(&library.conn).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0], created);
        assert_eq!(created.name, "Cat");
        assert_eq!(created.text, "cat animal");
    }

    #[test]
    fn editing_keeps_the_id_and_moves_updated_at() {
        let (_dir, library) = library();
        let created = upsert(&library, &new_stamp("Cat", "cat animal")).unwrap();

        let edited = upsert(
            &library,
            &StampInput {
                id: Some(created.id.clone()),
                name: "Cat".to_string(),
                text: "cat animal -dog".to_string(),
            },
        )
        .unwrap();

        assert_eq!(edited.id, created.id);
        assert_eq!(edited.text, "cat animal -dog");
        assert!(edited.updated_at >= created.updated_at);
        assert_eq!(list(&library.conn).unwrap().len(), 1);
    }

    #[test]
    fn an_empty_name_is_refused() {
        let (_dir, library) = library();

        let error = upsert(&library, &new_stamp("   ", "cat animal")).unwrap_err();

        assert!(matches!(error, AppError::BadRequest(_)), "got {error}");
        assert!(list(&library.conn).unwrap().is_empty());
    }

    #[test]
    fn an_empty_text_is_refused() {
        let (_dir, library) = library();

        let error = upsert(&library, &new_stamp("Cat", "   ")).unwrap_err();

        assert!(matches!(error, AppError::BadRequest(_)), "got {error}");
        assert!(list(&library.conn).unwrap().is_empty());
    }

    #[test]
    fn editing_an_unknown_id_is_refused() {
        let (_dir, library) = library();

        let error = upsert(
            &library,
            &StampInput {
                id: Some("no-such-id".to_string()),
                name: "Cat".to_string(),
                text: "cat".to_string(),
            },
        )
        .unwrap_err();

        assert!(matches!(error, AppError::NotFound(_)), "got {error}");
    }

    #[test]
    fn deleting_a_stamp_is_idempotent() {
        let (_dir, library) = library();
        let created = upsert(&library, &new_stamp("Cat", "cat animal")).unwrap();

        delete(&library, &created.id).unwrap();
        assert!(list(&library.conn).unwrap().is_empty());

        delete(&library, &created.id).unwrap();
    }

    #[test]
    fn stamps_are_listed_by_position_not_by_name() {
        let (_dir, library) = library();
        let made: Vec<Stamp> = ["Mango", "Zebra", "Aardvark"]
            .iter()
            .map(|name| upsert(&library, &new_stamp(name, "x")).unwrap())
            .collect();
        reorder(
            &library,
            &[made[1].id.clone(), made[0].id.clone(), made[2].id.clone()],
        )
        .unwrap();

        assert_eq!(
            names(&list(&library.conn).unwrap()),
            vec!["Zebra", "Mango", "Aardvark"]
        );
    }

    fn names(stamps: &[Stamp]) -> Vec<&str> {
        stamps.iter().map(|stamp| stamp.name.as_str()).collect()
    }

    fn three_stamps(library: &Library) -> Vec<Stamp> {
        ["Cat", "Dog", "Bird"]
            .iter()
            .map(|name| upsert(library, &new_stamp(name, "x")).unwrap())
            .collect()
    }

    #[test]
    fn a_new_stamp_lands_last() {
        let (_dir, library) = library();
        let made = three_stamps(&library);
        reorder(
            &library,
            &[made[2].id.clone(), made[0].id.clone(), made[1].id.clone()],
        )
        .unwrap();

        upsert(&library, &new_stamp("Fish", "x")).unwrap();

        assert_eq!(
            names(&list(&library.conn).unwrap()),
            vec!["Bird", "Cat", "Dog", "Fish"]
        );
    }

    #[test]
    fn reorder_rewrites_positions_and_answers_the_new_order() {
        let (_dir, library) = library();
        let made = three_stamps(&library);

        let answered = reorder(
            &library,
            &[made[2].id.clone(), made[0].id.clone(), made[1].id.clone()],
        )
        .unwrap();

        assert_eq!(names(&answered), vec!["Bird", "Cat", "Dog"]);
        assert_eq!(answered, list(&library.conn).unwrap());
    }

    #[test]
    fn reorder_refuses_a_list_that_is_not_the_whole_set() {
        let (_dir, library) = library();
        let made = three_stamps(&library);
        let (cat, dog, bird) = (&made[0].id, &made[1].id, &made[2].id);
        let cases: Vec<Vec<String>> = vec![
            vec![bird.clone(), cat.clone()],
            vec![bird.clone(), cat.clone(), cat.clone()],
            vec![bird.clone(), cat.clone(), "no-such-id".to_string()],
            vec![bird.clone(), cat.clone(), dog.clone(), dog.clone()],
        ];

        for ids in cases {
            let error = reorder(&library, &ids).unwrap_err();
            assert!(matches!(error, AppError::BadRequest(_)), "got {error}");
            assert_eq!(
                names(&list(&library.conn).unwrap()),
                vec!["Cat", "Dog", "Bird"]
            );
        }
    }

    #[test]
    fn editing_a_stamp_keeps_its_position() {
        let (_dir, library) = library();
        let made = three_stamps(&library);
        reorder(
            &library,
            &[made[2].id.clone(), made[0].id.clone(), made[1].id.clone()],
        )
        .unwrap();

        upsert(
            &library,
            &StampInput {
                id: Some(made[0].id.clone()),
                name: "Kitten".to_string(),
                text: "x".to_string(),
            },
        )
        .unwrap();

        assert_eq!(
            names(&list(&library.conn).unwrap()),
            vec!["Bird", "Kitten", "Dog"]
        );
    }

    #[test]
    fn reorder_rewrites_library_json() {
        let (_dir, library) = library();
        let made = three_stamps(&library);

        reorder(
            &library,
            &[made[2].id.clone(), made[0].id.clone(), made[1].id.clone()],
        )
        .unwrap();

        let file =
            crate::sidecar::read_library(&crate::sidecar::library_path(&library.paths)).unwrap();
        assert_eq!(names(&file.stamps.unwrap()), vec!["Bird", "Cat", "Dog"]);
    }

    /// `library-sidecars` design D3, D5: creating, editing and deleting a
    /// stamp are each visible in `library.json`, the same rule
    /// `rules::saving_disabling_and_deleting_a_rule_are_each_visible_in_
    /// library_json` pins for rules.
    #[test]
    fn creating_editing_and_deleting_a_stamp_are_each_visible_in_library_json() {
        let (_dir, library) = library();
        let library_json = crate::sidecar::library_path(&library.paths);

        let created = upsert(&library, &new_stamp("Cat", "cat animal")).unwrap();
        let file = crate::sidecar::read_library(&library_json).unwrap();
        assert_eq!(
            file.stamps.expect("the key is always written"),
            vec![created.clone()]
        );

        let edited = upsert(
            &library,
            &StampInput {
                id: Some(created.id.clone()),
                name: "Cat".to_string(),
                text: "cat animal -dog".to_string(),
            },
        )
        .unwrap();
        let file = crate::sidecar::read_library(&library_json).unwrap();
        assert_eq!(file.stamps.unwrap()[0].text, edited.text);

        delete(&library, &created.id).unwrap();
        let file = crate::sidecar::read_library(&library_json).unwrap();
        assert!(file.stamps.unwrap().is_empty());
    }
}
