
use aether_common::{mock::MockPlayer, testing::write_silent_wav, Player};
use aether_core::{Library, PlayTracker, PlaybackController};
use std::{sync::Arc, time::Duration};

fn set_position(mock: &MockPlayer, secs: u64) {
    let mut status = mock.status().unwrap();
    status.position = Duration::from_secs(secs);
    mock.set_status(status);
}

#[test]
fn scan_make_a_playlist_play_and_record_history() {
    // A folder with two 60-second tracks.
    let dir = tempfile::tempdir().unwrap();
    write_silent_wav(&dir.path().join("a.wav"), 60);
    write_silent_wav(&dir.path().join("b.wav"), 60);

    // Scan them and put both in a playlist.
    let mut lib = Library::open_in_memory().unwrap();
    lib.sync_folder(dir.path()).unwrap();
    let playlist = lib.create_playlist("Mix").unwrap();
    let ids: Vec<i64> = lib.tracks().unwrap().iter().map(|t| t.id).collect();
    lib.add_to_playlist(playlist.id, &ids).unwrap();

    // Play the playlist through the controller (a mock player, no speakers).
    let mock = Arc::new(MockPlayer::new());
    let controller = PlaybackController::new(mock.clone());
    controller.enqueue(
        lib.playlist_tracks(playlist.id)
            .unwrap()
            .into_iter()
            .map(|t| t.track),
    );
    controller.play().unwrap();

    let mut tracker = PlayTracker::new();

    // 10 seconds in: too early to count.
    set_position(&mock, 10);
    assert!(tracker.observe(&controller.status().unwrap()).is_none());

    // 31 seconds in: counts, exactly once.
    set_position(&mock, 31);
    let path = tracker
        .observe(&controller.status().unwrap())
        .expect("a play should be recorded");
    let id = lib.track_id_by_path(&path).unwrap().unwrap();
    lib.record_play(id).unwrap();
    assert!(tracker.observe(&controller.status().unwrap()).is_none());

    // The history row exists and shows up in both lists.
    assert_eq!(lib.play_count(id).unwrap(), 1);
    assert_eq!(lib.recently_played(10).unwrap()[0].id, id);
    assert_eq!(lib.most_played(10).unwrap()[0].1, 1);
}
