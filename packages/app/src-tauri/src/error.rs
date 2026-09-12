//! The one error type the Rust core returns, and its wire form for Tauri
//! commands (which can only reject with something `Serialize`).

/// Every failure the core can hand back. The HTTP layer maps these to status
/// codes: `BadRequest` → 400, `Decode` → 422, `NoLibrary` → 503.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    /// The database is damaged (design D8): `db::open`'s `PRAGMA
    /// quick_check(1)` failed, or a rusqlite call mid-session carried one of
    /// SQLite's corrupt-class codes (`DatabaseCorrupt`, `NotADatabase`) —
    /// [`AppError::from_rusqlite`] maps the first case and
    /// [`AppError::or_corrupt`], from `library::with_library_if_open`, the
    /// second, both through one predicate, so the same message names the
    /// recovery wherever the damage surfaces. Distinct from `SchemaTooNew` (a
    /// newer build wrote it, it is not damaged) and `JournalMode` (the wrong
    /// pragma, not corruption).
    #[error("the library at {} is damaged; rebuild it to restore access", path.display())]
    LibraryCorrupt { path: std::path::PathBuf },

    #[error("{0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Db(#[from] rusqlite::Error),

    #[error("not an image this app can decode: {0}")]
    Decode(#[from] image::ImageError),

    #[error("no library is open")]
    NoLibrary,

    #[error("{0}")]
    BadRequest(String),

    #[error("{0} not found")]
    NotFound(String),

    #[error(
        "this library was written by a newer BooruBox (schema v{found}; this build knows v{known})"
    )]
    SchemaTooNew { found: i64, known: i64 },

    #[error("library is in {mode} journal mode; BooruBox requires the rollback journal (DELETE)")]
    JournalMode { mode: String },

    /// The OS credential store refused to read or write a site's API key —
    /// locked, access denied, or no backend on this platform (`booru-sites`
    /// design D7). Never a paraphrase of the reason it gave, since that
    /// reason is what the user acts on to fix it. Distinct from
    /// `crate::model::UploadStep::Authenticate`, which is the *booru's own*
    /// rejection of a credential it did receive.
    #[error("{reason}")]
    Credential { reason: String },
}

/// Commands reject with a plain string: the webview has no use for the variant,
/// and a structured error would be a second contract to keep in step with
/// `packages/shared`.
impl serde::Serialize for AppError {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

pub type Result<T> = std::result::Result<T, AppError>;

impl AppError {
    /// A rusqlite error against `path`, mapped to `LibraryCorrupt` when it
    /// carries one of SQLite's corrupt-class codes (design D8: `DatabaseCorrupt`
    /// for a malformed file, `NotADatabase` for one that is not a database at
    /// all), and passed through as `Db` for everything else — a busy lock or a
    /// constraint violation is not damage. The one place both `db::open`'s
    /// `quick_check` and a mid-session failure go through, so the message is
    /// never spelled twice.
    pub fn from_rusqlite(path: &std::path::Path, error: rusqlite::Error) -> AppError {
        if is_corrupt_class(&error) {
            AppError::LibraryCorrupt {
                path: path.to_path_buf(),
            }
        } else {
            AppError::Db(error)
        }
    }

    /// The same mapping one step later, for damage that surfaces mid-session
    /// (design D8: "wherever they arrive — at open or mid-session"). Every
    /// write and every query reaches rusqlite through `?`, which has already
    /// taken the `From<rusqlite::Error>` road to `Db` by the time anyone can
    /// name the file it happened to; `library::with_library_if_open` is where
    /// the open library's own path is known, so it hands the error back
    /// through here rather than each of a hundred call sites learning about
    /// SQLite's error codes.
    pub fn or_corrupt(self, path: &std::path::Path) -> AppError {
        match self {
            AppError::Db(error) => AppError::from_rusqlite(path, error),
            other => other,
        }
    }
}

/// SQLite saying the file itself is damaged, rather than the request being
/// wrong: `DatabaseCorrupt` (`SQLITE_CORRUPT`, a malformed file) and
/// `NotADatabase` (`SQLITE_NOTADB`, a file that is not one at all).
fn is_corrupt_class(error: &rusqlite::Error) -> bool {
    matches!(
        error,
        rusqlite::Error::SqliteFailure(inner, _)
            if matches!(
                inner.code,
                rusqlite::ErrorCode::DatabaseCorrupt | rusqlite::ErrorCode::NotADatabase
            )
    )
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    fn sqlite_failure(code: rusqlite::ErrorCode) -> rusqlite::Error {
        rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error {
                code,
                extended_code: 0,
            },
            None,
        )
    }

    /// design D8: both corrupt-class codes convert to `LibraryCorrupt`, naming
    /// the path the error was raised against.
    #[test]
    fn each_corrupt_class_code_converts_to_library_corrupt() {
        for code in [
            rusqlite::ErrorCode::DatabaseCorrupt,
            rusqlite::ErrorCode::NotADatabase,
        ] {
            let error =
                AppError::from_rusqlite(Path::new("/a/library.sqlite"), sqlite_failure(code));
            assert!(
                matches!(&error, AppError::LibraryCorrupt { path } if path == Path::new("/a/library.sqlite")),
                "unexpected error for {code:?}: {error:?}"
            );
        }
    }

    /// Every other rusqlite failure — a busy lock here — stays `Db`, not
    /// `LibraryCorrupt`: only the two codes above mean the file is damaged.
    #[test]
    fn an_unrelated_code_stays_db() {
        let error = AppError::from_rusqlite(
            Path::new("/a/library.sqlite"),
            sqlite_failure(rusqlite::ErrorCode::DatabaseBusy),
        );
        assert!(
            matches!(error, AppError::Db(_)),
            "unexpected error: {error:?}"
        );
    }

    /// design D8: damage that arrives mid-session has already been converted
    /// to `Db` by `?` before anyone can name the file — `or_corrupt` is the
    /// second door onto the same mapping, and it leaves every other error
    /// exactly as it found it.
    #[test]
    fn a_db_error_carrying_a_corrupt_code_becomes_library_corrupt_after_the_fact() {
        let path = Path::new("/a/library.sqlite");

        let corrupt =
            AppError::Db(sqlite_failure(rusqlite::ErrorCode::DatabaseCorrupt)).or_corrupt(path);
        assert!(
            matches!(&corrupt, AppError::LibraryCorrupt { path: found } if found == path),
            "unexpected error: {corrupt:?}"
        );

        let busy = AppError::Db(sqlite_failure(rusqlite::ErrorCode::DatabaseBusy)).or_corrupt(path);
        assert!(
            matches!(busy, AppError::Db(_)),
            "unexpected error: {busy:?}"
        );

        let missing = AppError::NotFound("library".to_string()).or_corrupt(path);
        assert!(
            matches!(missing, AppError::NotFound(_)),
            "unexpected error: {missing:?}"
        );
    }

    /// `SchemaTooNew` and `JournalMode` are never produced by
    /// `from_rusqlite` — they are raised directly by `db::migrate` and
    /// `db::assert_rollback_journal`, which never carry a rusqlite error to
    /// convert, and stay their own variants regardless of this mapping.
    #[test]
    fn schema_too_new_and_journal_mode_are_unrelated_variants() {
        let schema = AppError::SchemaTooNew { found: 9, known: 5 };
        let journal = AppError::JournalMode {
            mode: "wal".to_string(),
        };
        assert!(!matches!(schema, AppError::LibraryCorrupt { .. }));
        assert!(!matches!(journal, AppError::LibraryCorrupt { .. }));
    }
}
