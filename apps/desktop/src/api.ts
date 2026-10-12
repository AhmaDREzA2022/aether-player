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
  searchTracks: (query: string) => invoke<Track[]>("search_tracks", { query })
};
