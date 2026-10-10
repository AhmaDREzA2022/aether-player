use crate::{
    mock::Call::{Next, Prev},
    PlaybackState, Player, PlayerError, PlayerStatus, Track,
};
use std::{sync::Mutex, time::Duration};

/// A record of one call made on the player.
#[derive(Debug, Clone, PartialEq)]
pub enum Call {
    Load(Track),
    Play,
    Pause,
    Seek(Duration),
    Next,
    Prev,
    SetVolume(f32),
}

/// A fake player for tests. It rembers every call and keeps a status
/// that tests can read or overwite.
pub struct MockPlayer {
    calls: Mutex<Vec<Call>>,
    status: Mutex<PlayerStatus>,
}

impl MockPlayer {
    pub fn new() -> Self {
        Self {
            calls: Mutex::new(Vec::new()),
            status: Mutex::new(PlayerStatus {
                state: PlaybackState::Stopped,
                position: Duration::ZERO,
                duration: Duration::ZERO,
                volume: 1.0,
                track: None,
            }),
        }
    }

    /// Every call made so far, in order.
    pub fn calls(&self) -> Vec<Call> {
        self.calls.lock().unwrap().clone()
    }

    /// Overwrite the status, e.g. to simulate a track finishing.
    pub fn set_status(&self, status: PlayerStatus) {
        *self.status.lock().unwrap() = status;
    }

    fn record(&self, call: Call) {
        self.calls.lock().unwrap().push(call);
    }
}

impl Default for MockPlayer {
    fn default() -> Self {
        Self::new()
    }
}

impl Player for MockPlayer {
    fn load(&self, track: &Track) -> Result<(), PlayerError> {
        self.record(Call::Load(track.clone()));
        let mut s = self.status.lock().unwrap();
        s.state = PlaybackState::Stopped;
        s.position = Duration::ZERO;
        s.duration = track.duration;
        s.track = Some(track.clone());
        Ok(())
    }

    fn play(&self) -> Result<(), PlayerError> {
        self.record(Call::Play);
        let mut s = self.status.lock().unwrap();
        if s.track.is_none() {
            return Err(PlayerError::NotFound("no track loaded".into()));
        }
        s.state = PlaybackState::Playing;
        Ok(())
    }

    fn pause(&self) -> Result<(), PlayerError> {
        self.record(Call::Pause);
        self.status.lock().unwrap().state = PlaybackState::Paused;
        Ok(())
    }

    fn seek(&self, pos: Duration) -> Result<(), PlayerError> {
        self.record(Call::Seek(pos));
        let mut s = self.status.lock().unwrap();
        s.position = pos.min(s.duration);
        Ok(())
    }

    fn next(&self) -> Result<(), PlayerError> {
        self.record(Call::Next);
        Err(PlayerError::NotSupported)
    }

    fn prev(&self) -> Result<(), PlayerError> {
        self.record(Call::Prev);
        Err(PlayerError::NotSupported)
    }

    fn set_volume(&self, vol: f32) -> Result<(), PlayerError> {
        self.record(Call::SetVolume(vol));
        self.status.lock().unwrap().volume = vol.clamp(0.0, 1.0);
        Ok(())
    }

    fn status(&self) -> Result<PlayerStatus, PlayerError> {
        Ok(self.status.lock().unwrap().clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn track() -> Track {
        Track {
            path: PathBuf::from("a.mp3"),
            title: "A".into(),
            artist: "Artist".into(),
            album: "Album".into(),
            duration: Duration::from_secs(100),
            track_no: Some(1),
        }
    }

    #[test]
    fn load_then_play_updates_status_and_records_calls() {
        let p = MockPlayer::new();
        p.load(&track()).unwrap();
        p.play().unwrap();

        assert_eq!(p.status().unwrap().state, PlaybackState::Playing);
        assert_eq!(p.calls(), vec![Call::Load(track()), Call::Play]);
    }

    #[test]
    fn play_without_a_track_fails() {
        let p = MockPlayer::new();
        assert!(p.play().is_err());
    }

    #[test]
    fn volume_is_clamped_and_seek_stays_inside_the_track() {
        let p = MockPlayer::new();
        p.load(&track()).unwrap();
        p.set_volume(5.0).unwrap();
        p.seek(Duration::from_secs(500)).unwrap();

        let s = p.status().unwrap();
        assert_eq!(s.volume, 1.0);
        assert_eq!(s.position, Duration::from_secs(100))
    }

    #[test]
    fn works_as_a_trait_object() {
        let p: Box<dyn Player> = Box::new(MockPlayer::new());
        assert!(p.status().is_ok());
    }
}
