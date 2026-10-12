//! Aether Core
//!
//! Queue, Playlist, Player API and library management.

// Placeholder – real implementation comes in later commits.

mod controller;
mod queue;
mod ticker;
#[cfg(test)]
mod test_util;

pub use controller::PlaybackController;
pub use queue::Queue;
pub use ticker::{spawn_ticker, DEFAULT_TICK_INTERVAL};
