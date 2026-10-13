import { invoke } from "@tauri-apps/api/core";

export interface Track {
  id: number;
  path: string;
  title: string;
  artist: string;
  durationMs: number;
  trackNo: number | null;
}

export interface SyncReport {
  added: number;
  updated: number;
  unchanged: number;
  removed: number;
  failed: number;
}

export const api = {
  defaultMusicDir: () => invoke<string>("default_music_dir"),
  scanFolder: (path: string) => invoke<SyncReport>("scan_folder", { path }),
  listTracks: () => invoke<Track[]>("list_tracks"),
  searchTracks: (query: string) => invoke<Track[]>("search_tracks", { query }),
  playList: (query: string, start: number) =>
    invoke<void>("play_list", { query, start }),
  toggle: () => invoke<void>("player_toggle"),
  next: () => invoke<void>("player_next"),
  prev: () => invoke<void>("player_prev"),
  seek: (positionMs: number) => invoke<void>("player_seek", { positionMs }),
  setVolume: (volume: number) => invoke<void>("player_set_volume", { volume }),
  status: () => invoke<PlayerStatus>("player_status"),
};

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
