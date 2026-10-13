//! Aether Core
//!
//! Queue, Playlist, Player API and library management.

// Placeholder – real implementation comes in later commits.

mod controller;
mod error;
mod history;
mod library;
mod queue;
#[cfg(test)]
mod test_util;
mod ticker;

pub use controller::PlaybackController;
pub use error::LibraryError;
pub use history::{play_threshold, record_if_due, PlayTracker};
pub use library::{Library, LibraryTrack, Playlist, SyncReport};
pub use queue::Queue;
pub use ticker::{spawn_ticker, DEFAULT_TICK_INTERVAL};
