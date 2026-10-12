use crate::{AppState, error::AppError};
use aether_core::{LibraryTrack, SyncReport};
use serde::Serialize;
use std::{path::PathBuf};
use tauri::{Manager, State};

/// a track as the frontend sees it. field names become camelCase in json.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackDto {
    id: i64,
    path: String,
    title: String,
    artist: String,
    album: String,
    duration_ms: u64,
    track_no: Option<u32>,
}

impl From<LibraryTrack> for TrackDto {
    fn from(t: LibraryTrack) -> Self {
        Self {
            id: t.id,
            path: t.track.path.to_string_lossy().into_owned(),
            title: t.track.title,
            artist: t.track.artist,
            album: t.track.album,
            duration_ms: t.track.duration.as_millis() as u64,
            track_no: t.track.track_no,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncReportDto {
    added: usize,
    updated: usize,
    unchanged: usize,
    removed: usize,
    failed: usize,
}

impl From<SyncReport> for SyncReportDto {
    fn from(r: SyncReport) -> Self {
        Self {
            added: r.added,
            updated: r.updated,
            unchanged: r.unchanged,
            removed: r.removed,
            failed: r.failed,
        }
    }
}

/// the users music folder, to prefill the sxan box.
#[tauri::command]
pub fn default_music_dir(app: tauri::AppHandle) -> Result<String, AppError> {
    let dir = app
        .path()
        .audio_dir()
        .map_err(|e| AppError::Other(e.to_string()))?;
    Ok(dir.to_string_lossy().into_owned())
}

// using async to run the func on a background so a long scan doesnt freeze the window
#[tauri::command(async)]
pub fn scan_folder(state: State<'_, AppState>, path: String) -> Result<SyncReportDto, AppError> {
    let report = state.library().sync_folder(&PathBuf::from(path))?;
    Ok(report.into())
}

#[tauri::command(async)]
pub fn list_tracks(state: State<'_, AppState>) -> Result<Vec<TrackDto>, AppError> {
    let tracks = state.library().tracks()?;
    Ok(tracks.into_iter().map(TrackDto::from).collect())
}

#[tauri::command(async)]
pub fn search_tracks(state: State<'_, AppState>, query: String) -> Result<Vec<TrackDto>, AppError> {
    let tracks = state.library().search(&query)?;
    Ok(tracks.into_iter().map(TrackDto::from).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use aether_common::Track;
    use std::time::Duration;

    /// The frontend depends on these exact JSON names, so pin them down.
    #[test]
    fn track_dto_uses_the_json_names_the_frontend_expects() {
        let dto = TrackDto::from(LibraryTrack {
            id: 7,
            track: Track {
                path: PathBuf::from("/m/a.mp3"),
                title: "T".into(),
                artist: "A".into(),
                album: "B".into(),
                duration: Duration::from_millis(61_500),
                track_no: Some(3),
            },
        });

        let json = serde_json::to_value(&dto).unwrap();
        assert_eq!(json["id"], 7);
        assert_eq!(json["path"], "/m/a.mp3");
        assert_eq!(json["durationMs"], 61_500);
        assert_eq!(json["trackNo"], 3);
    }
}
