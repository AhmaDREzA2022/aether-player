use crate::{Library, LibraryError};
use aether_common::{PlaybackState, PlayerStatus};
use std::{path::PathBuf, time::Duration};

const MAX_THRESHOLD: Duration = Duration::from_secs(30);

/// How far into a track it must get to count as a play: 30 seconds, or half
/// the track if that is shorter. An unknown (zero) duration uses 30 seconds.
pub fn play_threshold(duration: Duration) -> Duration {
    if duration.is_zero() {
        MAX_THRESHOLD
    } else {
        MAX_THRESHOLD.min(duration / 2)
    }
}

/// Watches player status snapshots and says when a play should be recorded.
/// Call `observe` on every tick; it returns the track's path exactly once
/// per play.
#[derive(Debug, Default)]
pub struct PlayTracker {
    current: Option<PathBuf>,
    counted: bool,
    was_finished: bool,
}

impl PlayTracker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn observe(&mut self, status: &PlayerStatus) -> Option<PathBuf> {
        let Some(track) = &status.track else {
            *self = Self::default();
            return None;
        };

        let finished = status.state == PlaybackState::Finished;
        let new_track = self.current.as_ref() != Some(&track.path);
        let restarted = self.was_finished && !finished;
        if new_track || restarted {
            self.current = Some(track.path.clone());
            self.counted = false;
        }
        self.was_finished = finished;

        if self.counted
            || status.state == PlaybackState::Stopped
            || status.position < play_threshold(status.duration)
        {
            return None;
        }

        self.counted = true;
        Some(track.path.clone())
    }
}

/// Feed the tracker a status; if a play is due, store it in the library.
/// Returns whether a play was recorded. A track that isn't in the library
/// is ignored.
pub fn record_if_due(
    tracker: &mut PlayTracker,
    status: &PlayerStatus,
    library: &Library,
) -> Result<bool, LibraryError> {
    let Some(path) = tracker.observe(status) else {
        return Ok(false);
    };
    match library.track_id_by_path(&path)? {
        Some(id) => {
            library.record_play(id)?;
            Ok(true)
        }
        None => Ok(false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aether_common::Track;
    use PlaybackState::{Finished, Playing, Stopped};

    fn status(path: &str, state: PlaybackState, position: u64, duration: u64) -> PlayerStatus {
        PlayerStatus {
            state,
            position: Duration::from_secs(position),
            duration: Duration::from_secs(duration),
            volume: 1.0,
            track: Some(Track {
                path: PathBuf::from(path),
                title: path.into(),
                artist: "A".into(),
                album: "B".into(),
                duration: Duration::from_secs(duration),
                track_no: None,
            }),
        }
    }

    #[test]
    fn a_long_track_counts_after_30_seconds_and_only_once() {
        let mut t = PlayTracker::new();
        assert!(t.observe(&status("a.mp3", Playing, 29, 300)).is_none());
        assert_eq!(
            t.observe(&status("a.mp3", Playing, 30, 300)),
            Some(PathBuf::from("a.mp3"))
        );
        assert!(t.observe(&status("a.mp3", Playing, 31, 300)).is_none());
        assert!(t.observe(&status("a.mp3", Playing, 200, 300)).is_none());
    }

    #[test]
    fn a_short_track_counts_at_half_its_length() {
        let mut t = PlayTracker::new();
        assert!(t.observe(&status("a.mp3", Playing, 19, 40)).is_none());
        assert!(t.observe(&status("a.mp3", Playing, 20, 40)).is_some());

        let mut t = PlayTracker::new();
        assert!(t.observe(&status("b.mp3", Playing, 4, 10)).is_none());
        assert!(t.observe(&status("b.mp3", Playing, 5, 10)).is_some());
    }

    #[test]
    fn an_unknown_duration_uses_the_30_second_rule() {
        let mut t = PlayTracker::new();
        assert!(t.observe(&status("a.mp3", Playing, 29, 0)).is_none());
        assert!(t.observe(&status("a.mp3", Playing, 30, 0)).is_some());
    }

    #[test]
    fn a_new_track_starts_a_new_count() {
        let mut t = PlayTracker::new();
        assert!(t.observe(&status("a.mp3", Playing, 40, 300)).is_some());
        assert!(t.observe(&status("b.mp3", Playing, 10, 300)).is_none());
        assert_eq!(
            t.observe(&status("b.mp3", Playing, 40, 300)),
            Some(PathBuf::from("b.mp3"))
        );
    }

    #[test]
    fn a_track_restarted_after_finishing_counts_again() {
        let mut t = PlayTracker::new();
        assert!(t.observe(&status("a.mp3", Playing, 31, 300)).is_some());
        assert!(t.observe(&status("a.mp3", Finished, 300, 300)).is_none());
        assert!(t.observe(&status("a.mp3", Playing, 0, 300)).is_none());
        assert!(t.observe(&status("a.mp3", Playing, 30, 300)).is_some());
    }

    #[test]
    fn a_tiny_track_that_finishes_between_ticks_still_counts() {
        let mut t = PlayTracker::new();
        assert!(t.observe(&status("a.mp3", Finished, 1, 1)).is_some());
    }

    #[test]
    fn a_stopped_player_never_counts() {
        let mut t = PlayTracker::new();
        assert!(t.observe(&status("a.mp3", Stopped, 100, 300)).is_none());
    }

    #[test]
    fn unloading_resets_the_tracker() {
        let mut t = PlayTracker::new();
        assert!(t.observe(&status("a.mp3", Playing, 40, 300)).is_some());

        let mut unloaded = status("a.mp3", Stopped, 0, 0);
        unloaded.track = None;
        assert!(t.observe(&unloaded).is_none());

        assert!(t.observe(&status("a.mp3", Playing, 40, 300)).is_some());
    }

    #[test]
    fn due_plays_are_stored_in_the_library() {
        let dir = tempfile::tempdir().unwrap();
        aether_common::testing::write_silent_wav(&dir.path().join("a.wav"), 60);
        let mut lib = Library::open_in_memory().unwrap();
        lib.sync_folder(dir.path()).unwrap();
        let stored = lib.tracks().unwrap().remove(0);
        let path = stored.track.path.to_string_lossy().into_owned();

        let mut tracker = PlayTracker::new();
        assert!(!record_if_due(&mut tracker, &status(&path, Playing, 10, 60), &lib).unwrap());
        assert!(record_if_due(&mut tracker, &status(&path, Playing, 31, 60), &lib).unwrap());
        assert!(!record_if_due(&mut tracker, &status(&path, Playing, 40, 60), &lib).unwrap());
        assert_eq!(lib.play_count(stored.id).unwrap(), 1);
    }

    #[test]
    fn plays_of_tracks_outside_the_library_are_ignored() {
        let lib = Library::open_in_memory().unwrap();
        let mut tracker = PlayTracker::new();
        let s = status("/elsewhere/x.mp3", Playing, 40, 60);

        assert!(!record_if_due(&mut tracker, &s, &lib).unwrap());
        assert!(lib.recently_played(10).unwrap().is_empty());
    }
}
