use crate::Queue;
use aether_common::{PlaybackState, Player, PlayerError, PlayerStatus, Track};
use std::{
    sync::{Arc, Mutex, MutexGuard},
    time::Duration,
};

/// Further into a track than this, "previous" restarts it instead of going back.
const RESTART_THRESHOLD: Duration = Duration::from_secs(3);

/// Wraps any `Player` and adds a queue, so `next` and `prev` work.
/// It implements `Player` itself, so the UI only ever sees the trait.
pub struct PlaybackController {
    player: Arc<dyn Player>,
    queue: Mutex<Queue>,
}

impl PlaybackController {
    pub fn new(player: Arc<dyn Player>) -> Self {
        Self {
            player,
            queue: Mutex::new(Queue::new()),
        }
    }

    pub fn enqueue(&self, tracks: impl IntoIterator<Item = Track>) {
        self.queue().extend(tracks);
    }

    pub fn queue_snapshot(&self) -> Vec<Track> {
        self.queue().tracks().to_vec()
    }

    /// Jump to a queue position and start playing it.
    pub fn play_index(&self, index: usize) -> Result<(), PlayerError> {
        let track = self.queue().set_current(index).cloned();
        match track {
            Some(t) => self.load_and_play(&t),
            None => Err(PlayerError::NotFound(format!("no queue entry {index}"))),
        }
    }

    /// Call this regularly (every ~500 ms). If the current track finished,
    /// it moves on to the next one. Kept separate from any thread so it can
    /// be tested by simply calling it.
    pub fn tick(&self) {
        let finished = matches!(
            self.player.status(),
            Ok(s) if s.state == PlaybackState::Finished
        );
        let in_queue = self.queue().current().is_some();

        if finished && in_queue {
            if let Err(e) = self.advance() {
                tracing::warn!("auto-advance failed: {e}");
            }
        }
    }

    /// Lock the queue. If another thread panicked while holding the lock,
    /// carry on with the data anyway instead of panicking too.
    fn queue(&self) -> MutexGuard<'_, Queue> {
        self.queue.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn load_and_play(&self, track: &Track) -> Result<(), PlayerError> {
        self.player.load(track)?;
        self.player.play()
    }

    /// Move to the next queued track. Returns `false` at the end of the queue.
    fn advance(&self) -> Result<bool, PlayerError> {
        // The lock is released at the end of this statement, before we
        // talk to the player.
        let next = self.queue().next_track().cloned();
        match next {
            Some(t) => {
                self.load_and_play(&t)?;
                Ok(true)
            }
            None => Ok(false),
        }
    }
}

impl Player for PlaybackController {
    fn load(&self, track: &Track) -> Result<(), PlayerError> {
        self.player.load(track)
    }

    fn play(&self) -> Result<(), PlayerError> {
        // Nothing loaded yet: start the queue from its current (or first) track.
        if self.player.status()?.track.is_none() {
            let track = {
                let mut q = self.queue();
                if q.current().is_none() {
                    q.next_track();
                }
                q.current().cloned()
            };
            return match track {
                Some(t) => self.load_and_play(&t),
                None => Err(PlayerError::NotFound("queue is empty".into())),
            };
        }
        self.player.play()
    }

    fn pause(&self) -> Result<(), PlayerError> {
        self.player.pause()
    }

    fn seek(&self, pos: Duration) -> Result<(), PlayerError> {
        self.player.seek(pos)
    }

    /// At the end of the queue this does nothing and returns `Ok`.
    fn next(&self) -> Result<(), PlayerError> {
        self.advance().map(|_| ())
    }

    fn prev(&self) -> Result<(), PlayerError> {
        if self.player.status()?.position > RESTART_THRESHOLD {
            return self.player.seek(Duration::ZERO);
        }
        let prev = self.queue().prev_track().cloned();
        match prev {
            Some(t) => self.load_and_play(&t),
            None => self.player.seek(Duration::ZERO), // first track: restart it
        }
    }

    fn set_volume(&self, vol: f32) -> Result<(), PlayerError> {
        self.player.set_volume(vol)
    }

    fn status(&self) -> Result<PlayerStatus, PlayerError> {
        self.player.status()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::track;
    use aether_common::mock::{Call, MockPlayer};

    /// A controller over a mock with `n` queued tracks. The test keeps its
    /// own handle to the mock so it can inspect the calls.
    fn setup(n: u32) -> (Arc<MockPlayer>, PlaybackController) {
        let mock = Arc::new(MockPlayer::new());
        let controller = PlaybackController::new(mock.clone());
        controller.enqueue((0..n).map(track));
        (mock, controller)
    }

    fn set_state(mock: &MockPlayer, state: PlaybackState, position: Duration) {
        let mut s = mock.status().unwrap();
        s.state = state;
        s.position = position;
        mock.set_status(s);
    }

    #[test]
    fn play_starts_the_queue_from_the_first_track() {
        let (mock, c) = setup(3);
        c.play().unwrap();
        assert_eq!(mock.calls(), vec![Call::Load(track(0)), Call::Play]);
    }

    #[test]
    fn play_on_an_empty_queue_is_an_error() {
        let (_mock, c) = setup(0);
        assert!(c.play().is_err());
    }

    #[test]
    fn play_resumes_without_reloading_when_a_track_is_loaded() {
        let (mock, c) = setup(2);
        c.play().unwrap();
        c.pause().unwrap();
        c.play().unwrap();
        assert_eq!(
            mock.calls(),
            vec![Call::Load(track(0)), Call::Play, Call::Pause, Call::Play]
        );
    }

    #[test]
    fn next_loads_and_plays_the_following_track() {
        let (mock, c) = setup(3);
        c.play().unwrap();
        c.next().unwrap();
        assert_eq!(mock.calls()[2..], [Call::Load(track(1)), Call::Play]);
        assert_eq!(c.status().unwrap().track.unwrap().title, "t1");
    }

    #[test]
    fn next_at_the_end_does_nothing() {
        let (mock, c) = setup(1);
        c.play().unwrap();
        c.next().unwrap();
        assert_eq!(mock.calls().len(), 2); // just the original Load + Play
    }

    #[test]
    fn prev_restarts_the_track_when_far_into_it() {
        let (mock, c) = setup(3);
        c.next().unwrap();
        c.next().unwrap();
        set_state(&mock, PlaybackState::Playing, Duration::from_secs(10));
        let calls_before = mock.calls().len();

        c.prev().unwrap();
        assert_eq!(mock.calls()[calls_before..], [Call::Seek(Duration::ZERO)]);
    }

    #[test]
    fn prev_goes_to_the_previous_track_near_the_start() {
        let (mock, c) = setup(3);
        c.play().unwrap();
        c.next().unwrap(); // now on t1, position 0
        c.prev().unwrap();
        assert_eq!(mock.calls()[4..], [Call::Load(track(0)), Call::Play]);
    }

    #[test]
    fn prev_on_the_first_track_restarts_it() {
        let (mock, c) = setup(2);
        c.play().unwrap();
        c.prev().unwrap();
        assert_eq!(mock.calls()[2..], [Call::Seek(Duration::ZERO)]);
    }

    #[test]
    fn tick_advances_when_the_track_finished() {
        let (mock, c) = setup(2);
        c.play().unwrap();
        set_state(&mock, PlaybackState::Finished, Duration::from_secs(100));

        c.tick();
        assert_eq!(mock.calls()[2..], [Call::Load(track(1)), Call::Play]);
    }

    #[test]
    fn tick_leaves_a_playing_track_alone() {
        let (mock, c) = setup(2);
        c.play().unwrap();
        c.tick();
        assert_eq!(mock.calls().len(), 2);
    }
}