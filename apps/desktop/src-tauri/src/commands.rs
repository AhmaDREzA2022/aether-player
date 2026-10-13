use crate::{error::AppError, AppState};
use aether_common::{PlaybackState, Player, PlayerStatus, Track};
use aether_core::{LibraryTrack, SyncReport};
use serde::Serialize;
use std::{path::PathBuf, time::Duration};
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

/// What the now-playing bar needs to know about the loaded track.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NowPlayingDto {
    path: String,
    title: String,
    artist: String,
    album: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusDto {
    state: &'static str,
    position_ms: u64,
    duration_ms: u64,
    volume: f32,
    track: Option<NowPlayingDto>,
}

impl From<PlayerStatus> for StatusDto {
    fn from(s: PlayerStatus) -> Self {
        Self {
            state: match s.state {
                PlaybackState::Playing => "playing",
                PlaybackState::Paused => "paused",
                PlaybackState::Stopped => "stopped",
                PlaybackState::Finished => "finished",
            },
            position_ms: s.position.as_millis() as u64,
            duration_ms: s.duration.as_millis() as u64,
            volume: s.volume,
            track: s.track.map(|t| NowPlayingDto {
                path: t.path.to_string_lossy().into_owned(),
                title: t.title,
                artist: t.artist,
                album: t.album,
            }),
        }
    }
}

/// Play the results of a search (an empty query means the whole library),
/// starting at the clicked row. The list in the window and the queue come
/// from the same query, so they match.
#[tauri::command(async)]
pub fn play_list(state: State<'_, AppState>, query: String, start: usize) -> Result<(), AppError> {
    let tracks: Vec<Track> = state
        .library()
        .search(&query)?
        .into_iter()
        .map(|t| t.track)
        .collect();
    state.player()?.play_queue(tracks, start)?;
    Ok(())
}

#[tauri::command(async)]
pub fn player_toggle(state: State<'_, AppState>) -> Result<(), AppError> {
    let player = state.player()?;
    if player.status()?.state == PlaybackState::Playing {
        player.pause()?;
    } else {
        player.play()?;
    }
    Ok(())
}

#[tauri::command(async)]
pub fn player_next(state: State<'_, AppState>) -> Result<(), AppError> {
    state.player()?.next()?;
    Ok(())
}

#[tauri::command(async)]
pub fn player_prev(state: State<'_, AppState>) -> Result<(), AppError> {
    state.player()?.prev()?;
    Ok(())
}

#[tauri::command(async)]
pub fn player_seek(state: State<'_, AppState>, position_ms: u64) -> Result<(), AppError> {
    state.player()?.seek(Duration::from_millis(position_ms))?;
    Ok(())
}

#[tauri::command(async)]
pub fn player_set_volume(state: State<'_, AppState>, volume: f32) -> Result<(), AppError> {
    state.player()?.set_volume(volume)?;
    Ok(())
}

#[tauri::command(async)]
pub fn player_status(state: State<'_, AppState>) -> Result<StatusDto, AppError> {
    Ok(state.player()?.status()?.into())
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

    #[test]
    fn status_dto_uses_the_json_names_the_frontend_expects() {
        let dto = StatusDto::from(PlayerStatus {
            state: PlaybackState::Paused,
            position: Duration::from_millis(1500),
            duration: Duration::from_secs(60),
            volume: 0.5,
            track: None,
        });

        let json = serde_json::to_value(&dto).unwrap();
        assert_eq!(json["state"], "paused");
        assert_eq!(json["positionMs"], 1500);
        assert_eq!(json["durationMs"], 60_000);
        assert_eq!(json["volume"], 0.5);
        assert!(json["track"].is_null());
    }
}
