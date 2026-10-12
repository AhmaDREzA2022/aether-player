mod commands;
mod error;

use aether_core::Library;
use std::sync::{Mutex, MutexGuard};
use tauri::Manager;

/// Shared state that every command can read.
pub struct AppState {
    library: Mutex<Library>,
}

impl AppState {
    /// Lock the library. If a thread panicked while holding the lock,
    /// carry on with the data instead of panicking too.
    pub fn library(&self) -> MutexGuard<'_, Library> {
        self.library.lock().unwrap_or_else(|e| e.into_inner())
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            let library = Library::open(&data_dir.join("library.db"))?;
            app.manage(AppState {
                library: Mutex::new(library),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::default_music_dir,
            commands::scan_folder,
            commands::list_tracks,
            commands::search_tracks,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}