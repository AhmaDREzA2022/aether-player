use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Error, Serialize, Deserialize)]
pub enum PlayerError {
    #[error("file not found: {0}")]
    NotFound(String),

    #[error("unsupported format: {0}")]
    UnsupportedFormat(String),

    #[error("no audio device available")]
    NoAudioDevice,

    #[error("seeking is not supported for this track")]
    SeekUnsupported,

    #[error("audio device was lost")]
    DeviceLost,

    #[error("operation not supported by this player")]
    NotSupported,

    #[error("io error: {0}")]
    Io(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_messages_are_readable() {
        let e = PlayerError::NotFound("song.mp3".into());
        assert_eq!(e.to_string(), "file not found: song.mp3")
    }
}
