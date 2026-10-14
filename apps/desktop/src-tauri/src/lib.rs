mod commands;
mod error;

use aether_audio::LocalPlayer;
use aether_common::Player;
use aether_core::{
    record_if_due, spawn_ticker, Library, PlayTracker, PlaybackController, DEFAULT_TICK_INTERVAL,
};
use error::AppError;
use std::{
    sync::{Arc, Mutex, MutexGuard},
    thread,
    time::Duration,
};
use tauri::{Emitter, Manager};

/// Shared state that every command can read.
pub struct AppState {
    library: Mutex<Library>,
    /// `None` when no audio device could be opened. The library still works.
    player: Option<Arc<PlaybackController>>,
}

impl AppState {
    /// Lock the library. If a thread panicked while holding the lock,
    /// carry on with the data instead of panicking too.
    pub fn library(&self) -> MutexGuard<'_, Library> {
        self.library.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn player(&self) -> Result<&Arc<PlaybackController>, AppError> {
        self.player
            .as_ref()
            .ok_or_else(|| AppError::Other("no audio device available".into()))
    }
}

/// Records plays in the library and tells the window when one was stored.
/// Ends by itself when the player is dropped.
fn spawn_history_recorder(handle: tauri::AppHandle, controller: &Arc<PlaybackController>) {
    let weak = Arc::downgrade(controller);
    thread::spawn(move || {
        let mut tracker = PlayTracker::new();
        loop {
            thread::sleep(Duration::from_millis(500));
            let Some(controller) = weak.upgrade() else { break };
            let Ok(status) = controller.status() else { continue };

            let recorded = {
                let state = handle.state::<AppState>();
                let library = state.library();
                match record_if_due(&mut tracker, &status, &library) {
                    Ok(recorded) => recorded,
                    Err(e) => {
                        eprintln!("could not record play: {e}");
                        false
                    }
                }
            };
            if recorded {
                let _ = handle.emit("history-changed", ());
            }
        }
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            let library = Library::open(&data_dir.join("library.db"))?;

            let player = match LocalPlayer::new() {
                Ok(p) => Some(Arc::new(PlaybackController::new(Arc::new(p)))),
                Err(e) => {
                    eprintln!("audio unavailable: {e}");
                    None
                }
            };

            app.manage(AppState {
                library: Mutex::new(library),
                player: player.clone(),
            });

            if let Some(controller) = player {
                let _ticker = spawn_ticker(&controller, DEFAULT_TICK_INTERVAL)?;
                spawn_history_recorder(app.handle().clone(), &controller);
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::default_music_dir,
            commands::scan_folder,
            commands::list_source,
            commands::play_source,
            commands::favorite_ids,
            commands::toggle_favorite,
            commands::list_playlists,
            commands::create_playlist,
            commands::rename_playlist,
            commands::delete_playlist,
            commands::add_to_playlist,
            commands::remove_from_playlist,
            commands::move_in_playlist,
            commands::player_toggle,
            commands::player_next,
            commands::player_prev,
            commands::player_seek,
            commands::player_set_volume,
            commands::player_status,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
