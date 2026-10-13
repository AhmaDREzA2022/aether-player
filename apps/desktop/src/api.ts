import { invoke } from "@tauri-apps/api/core";

export interface Track {
  id: number;
  path: string;
  title: string;
  artist: string;
  album: string;
  durationMs: number;
  trackNo: number | null;
  /** Only set in the "Most played" view. */
  plays: number | null;
}

export interface SyncReport {
  added: number;
  updated: number;
  unchanged: number;
  removed: number;
  failed: number;
}

/** Which list of tracks to show or play. */
export type Source =
  | { kind: "library"; query: string }
  | { kind: "favorites" }
  | { kind: "recent" }
  | { kind: "mostPlayed" };

export type PlaybackState = "playing" | "paused" | "stopped" | "finished";

export interface NowPlaying {
  path: string;
  title: string;
  artist: string;
  album: string;
}

export interface PlayerStatus {
  state: PlaybackState;
  positionMs: number;
  durationMs: number;
  volume: number;
  track: NowPlaying | null;
}

export const api = {
  defaultMusicDir: () => invoke<string>("default_music_dir"),
  scanFolder: (path: string) => invoke<SyncReport>("scan_folder", { path }),
  listSource: (source: Source) => invoke<Track[]>("list_source", { source }),
  playSource: (source: Source, start: number) =>
    invoke<void>("play_source", { source, start }),
  favoriteIds: () => invoke<number[]>("favorite_ids"),
  toggleFavorite: (trackId: number) => invoke<boolean>("toggle_favorite", { trackId }),
  toggle: () => invoke<void>("player_toggle"),
  next: () => invoke<void>("player_next"),
  prev: () => invoke<void>("player_prev"),
  seek: (positionMs: number) => invoke<void>("player_seek", { positionMs }),
  setVolume: (volume: number) => invoke<void>("player_set_volume", { volume }),
  status: () => invoke<PlayerStatus>("player_status"),
};