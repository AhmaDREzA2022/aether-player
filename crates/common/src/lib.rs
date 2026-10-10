//! Aether Common
//!
//! Shared utilities and types used across all crates.

// Placeholder – real implementation comes in later commits.
mod error;
mod player;
mod track;

pub use error::PlayerError;
pub use player::{PlaybackState, Player, PlayerStatus};
pub use track::Track;
