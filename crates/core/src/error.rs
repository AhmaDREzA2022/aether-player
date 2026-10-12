use thiserror::Error;

#[derive(Debug, Error)]
pub enum LibraryError {
    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),

    #[error("io error: {0}")]
    Io(String),

    #[error("database schema version {found} is newer than this app supports ({supported})")]
    SchemaTooNew { found: i64, supported: i64 },

    #[error("no track with id {0}")]
    TrackNotFound(i64),
}
