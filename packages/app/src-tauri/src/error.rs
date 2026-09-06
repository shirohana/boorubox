//! The one error type the Rust core returns, and its wire form for Tauri
//! commands (which can only reject with something `Serialize`).

/// Every failure the core can hand back. The HTTP layer maps these to status
/// codes: `BadRequest` → 400, `Decode` → 422, `NoLibrary` → 503.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
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
