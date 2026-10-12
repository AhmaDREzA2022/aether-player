use aether_common::Track;
use std::{path::PathBuf, time::Duration};

/// A fake track for tests: title "t<n>", 100 seconds long.
pub fn track(n: u32) -> Track {
    Track {
        path: PathBuf::from(format!("t{n}.mp3")),
        title: format!("t{n}"),
        artist: "Artist".into(),
        album: "Album".into(),
        duration: Duration::from_secs(100),
        track_no: Some(n),
    }
}