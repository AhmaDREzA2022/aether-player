use crate::{error::AppError, AppState};
use aether_common::{PlaybackState, Player, PlayerStatus, Track};
use aether_core::{Library, LibraryError, LibraryTrack, SyncReport};
use serde::{Deserialize, Serialize};
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
    plays: Option<usize>,
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
            plays: None,
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

/// Which list the UI is showing. It arrives from the frontend as JSON like
/// `{ "kind": "library", "query": "daft" }` or `{ "kind": "favorites" }`.
#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ListSource {
    Library { query: String },
    Favorites,
    Recent,
    MostPlayed,
}

const HISTORY_LIMIT: usize = 100;

/// The tracks of a source, each with a play count where that applies.
fn load_source(
    library: &Library,
    source: &ListSource,
) -> Result<Vec<(LibraryTrack, Option<usize>)>, LibraryError> {
    let plain = |tracks: Vec<LibraryTrack>| tracks.into_iter().map(|t| (t, None)).collect();
    Ok(match source {
        ListSource::Library { query } => plain(library.search(query)?),
        ListSource::Favorites => plain(library.favorites()?),
        ListSource::Recent => plain(library.recently_played(HISTORY_LIMIT)?),
        ListSource::MostPlayed => library
            .most_played(HISTORY_LIMIT)?
            .into_iter()
            .map(|(t, plays)| (t, Some(plays)))
            .collect(),
    })
}

#[tauri::command(async)]
pub fn list_source(
    state: State<'_, AppState>,
    source: ListSource,
) -> Result<Vec<TrackDto>, AppError> {
    let rows = load_source(&state.library(), &source)?;
    Ok(rows
        .into_iter()
        .map(|(track, plays)| {
            let mut dto = TrackDto::from(track);
            dto.plays = plays;
            dto
        })
        .collect())
}

/// Queue the source's tracks, in the order the window shows them, and start
/// at the clicked row.
#[tauri::command(async)]
pub fn play_source(
    state: State<'_, AppState>,
    source: ListSource,
    start: usize,
) -> Result<(), AppError> {
    let tracks: Vec<Track> = load_source(&state.library(), &source)?
        .into_iter()
        .map(|(t, _)| t.track)
        .collect();
    state.player()?.play_queue(tracks, start)?;
    Ok(())
}

#[tauri::command(async)]
pub fn favorite_ids(state: State<'_, AppState>) -> Result<Vec<i64>, AppError> {
    Ok(state.library().favorite_ids()?)
}

/// Flip a track's favorite flag and return the new state.
#[tauri::command(async)]
pub fn toggle_favorite(state: State<'_, AppState>, track_id: i64) -> Result<bool, AppError> {
    Ok(state.library().toggle_favorite(track_id)?)
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
        assert!(json["plays"].is_null());
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

        #[test]
    fn list_source_parses_the_frontend_json() {
        let q: ListSource =
            serde_json::from_str(r#"{"kind":"library","query":"daft"}"#).unwrap();
        assert!(matches!(q, ListSource::Library { query } if query == "daft"));

        let m: ListSource = serde_json::from_str(r#"{"kind":"mostPlayed"}"#).unwrap();
        assert!(matches!(m, ListSource::MostPlayed));
    }
}
