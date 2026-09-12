//! The library's one free-text note (`notes` design D3). One row, pinned to
//! `id = 1` by the table's own `CHECK`, so no writer has to remember the
//! convention and no read has to decide which of several rows is the note.

use rusqlite::{Connection, OptionalExtension, params};

use crate::db;
use crate::error::Result;
use crate::library::Library;
use crate::model::Note;

/// The note, or an empty one when the library has never been written to — the
/// spec's "a library that has never been written to shows an empty note rather
/// than an error", so the panel can be typed into on a fresh library.
pub fn get(conn: &Connection) -> Result<Note> {
    let note = conn
        .query_row(
            "SELECT content, updated_at FROM notes WHERE id = 1",
            [],
            |row| {
                Ok(Note {
                    content: row.get(0)?,
                    updated_at: row.get(1)?,
                })
            },
        )
        .optional()?;
    Ok(note.unwrap_or_default())
}

/// Write the note, creating the row on the first write. An upsert rather than
/// an insert-then-update: `id = 1` is the only row there can be, so "does it
/// exist yet" is not a question any caller should have to ask.
///
/// Takes the whole `Library`, not just its connection (`library-sidecars`
/// design D3): the note is one third of `library.json`, and this is the one
/// write path for it, rewritten here after the row's own commit.
pub fn set(library: &Library, content: &str) -> Result<Note> {
    let updated_at = db::now_ms();
    library.conn.execute(
        "INSERT INTO notes (id, content, updated_at) VALUES (1, ?1, ?2)
         ON CONFLICT (id) DO UPDATE SET content = excluded.content,
                                        updated_at = excluded.updated_at",
        params![content, updated_at],
    )?;
    crate::sidecar::write_library(&library.paths, &library.conn)?;
    Ok(Note {
        content: content.to_string(),
        updated_at,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::Library;

    fn library() -> (tempfile::TempDir, Library) {
        let dir = tempfile::tempdir().unwrap();
        let library = Library::open_or_create(dir.path()).unwrap();
        (dir, library)
    }

    #[test]
    fn a_library_that_was_never_written_to_reads_as_an_empty_note() {
        let (_dir, library) = library();

        assert_eq!(get(&library.conn).unwrap(), Note::default());
    }

    #[test]
    fn a_note_round_trips() {
        let (_dir, library) = library();

        let written = set(&library, "tags still to add").unwrap();

        assert_eq!(get(&library.conn).unwrap(), written);
        assert_eq!(written.content, "tags still to add");
        assert!(written.updated_at > 0);
    }

    #[test]
    fn writing_again_replaces_the_note_rather_than_adding_a_row() {
        let (_dir, library) = library();
        set(&library, "first").unwrap();

        set(&library, "second").unwrap();

        assert_eq!(get(&library.conn).unwrap().content, "second");
        let rows: i64 = library
            .conn
            .query_row("SELECT COUNT(*) FROM notes", [], |row| row.get(0))
            .unwrap();
        assert_eq!(rows, 1);
    }

    /// Clearing is a write of the empty string, not a delete: the spec asks for
    /// an empty note after a relaunch, which is what an empty row reads as and
    /// what a missing row reads as too — but only the write keeps `updated_at`
    /// honest about when the user emptied it.
    #[test]
    fn clearing_the_note_leaves_it_empty() {
        let (_dir, library) = library();
        set(&library, "something").unwrap();

        set(&library, "").unwrap();

        assert_eq!(get(&library.conn).unwrap().content, "");
    }

    /// `library-sidecars` task 1.10: the note's text is in `library.json`
    /// after a write.
    #[test]
    fn writing_the_note_is_visible_in_library_json() {
        let (_dir, library) = library();

        set(&library, "tags still to add").unwrap();

        let file =
            crate::sidecar::read_library(&crate::sidecar::library_path(&library.paths)).unwrap();
        assert_eq!(file.note.content, "tags still to add");
    }
}
