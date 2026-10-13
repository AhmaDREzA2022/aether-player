use aether_core::LibraryError;
use aether_common::PlayerError;
use serde::{Serialize, Serializer};

/// the error type commands return. tauri sends errors to the fronntend as
/// json, so this serialize as a plain message string
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error(transparent)]
    Library(#[from] LibraryError),

    #[error(transparent)]
    Player(#[from] PlayerError),

    #[error("{0}")]
    Other(String),
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}
