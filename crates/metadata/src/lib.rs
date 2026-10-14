//! Aether Metadata
//!
//! Music metadata parsing using lofty.

// Placeholder – real implementation comes in later commits.
mod scan;

pub use scan::{find_audio_files, read_cover, read_track, scan_folder, CoverArt, SUPPORTED_EXTENSIONS};
