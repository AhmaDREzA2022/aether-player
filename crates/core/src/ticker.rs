use crate::PlaybackController;
use std::{
    io,
    sync::Arc,
    thread::{self, JoinHandle},
    time::Duration,
};

pub const DEFAULT_TICK_INTERVAL: Duration = Duration::from_millis(500);

/// Starts a background thread that calls ` controler.tick()` every `interval`
/// It holds only a weak refrence, so it stops by itself once the controller
/// is dropped.
pub fn spawn_ticker(
    controller: &Arc<PlaybackController>,
    interval: Duration,
) -> io::Result<JoinHandle<()>> {
    let weak = Arc::downgrade(controller);

    thread::Builder::new()
        .name("aether-ticker".into())
        .spawn(move || loop {
            thread::sleep(interval);
            match weak.upgrade() {
                Some(controller) => controller.tick(),
                None => break,
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::track;
    use aether_common::{
        mock::{Call, MockPlayer},
        PlaybackState, Player,
    };

    #[test]
    fn ticker_advances_a_finished_track() {
        let mock = Arc::new(MockPlayer::new());
        let controller = Arc::new(PlaybackController::new(mock.clone()));
        controller.enqueue((0..2).map(track));
        controller.play().unwrap();

        let mut status = mock.status().unwrap();
        status.state = PlaybackState::Finished;
        mock.set_status(status);

        let _handle = spawn_ticker(&controller, Duration::from_millis(10)).unwrap();
        thread::sleep(Duration::from_millis(300));

        assert_eq!(mock.calls()[2..], [Call::Load(track(1)), Call::Play]);
    }

    #[test]
    fn ticker_stops_when_the_controller_is_dropped() {
        let mock = Arc::new(MockPlayer::new());
        let controller = Arc::new(PlaybackController::new(mock));
        let handle = spawn_ticker(&controller, Duration::from_millis(10)).unwrap();

        drop(controller);
        handle.join().unwrap(); // would hang forever if the thread never stopped
    }
}
