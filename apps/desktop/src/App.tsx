import { useCallback, useEffect, useMemo, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { api, Playlist, Source, SyncReport, Track } from "./api";
import { NowPlayingBar } from "./NowPlayingBar";
import { PlaylistHeader } from "./PlaylistHeader";
import { Sidebar, View } from "./Sidebar";
import { TrackTable } from "./TrackTable";
import { usePlayerStatus } from "./usePlayerStatus";
import { useShortcuts } from "./useShortcuts";
import "./App.css";

const EMPTY_TEXT: Record<string, string> = {
  library: "No tracks yet. Scan a folder to get started.",
  favorites: "No favorites yet. Click the heart next to a track.",
  recent: "Nothing played yet. A play counts after 30 seconds.",
  mostPlayed: "Nothing played yet. A play counts after 30 seconds.",
  playlist: "This playlist is empty. Add tracks with the + button.",
};

function describe(r: SyncReport): string {
  return `Scan finished: ${r.added} added, ${r.updated} updated, ${r.removed} removed, ${r.failed} failed`;
}

export default function App() {
  const [folder, setFolder] = useState("");
  const [view, setView] = useState<View>({ kind: "library" });
  const [query, setQuery] = useState("");
  const [tracks, setTracks] = useState<Track[]>([]);
  const [shownSource, setShownSource] = useState<Source>({
    kind: "library",
    query: "",
  });
  const [favorites, setFavorites] = useState<Set<number>>(new Set());
  const [playlists, setPlaylists] = useState<Playlist[]>([]);
  const [message, setMessage] = useState("");
  const [scanning, setScanning] = useState(false);
  const [version, setVersion] = useState(0);

  const status = usePlayerStatus();
  useShortcuts(status, setMessage);
  const playingPath = status?.track?.path ?? null;

  const source = useMemo<Source>(() => {
    switch (view.kind) {
      case "favorites":
        return { kind: "favorites" };
      case "recent":
        return { kind: "recent" };
      case "mostPlayed":
        return { kind: "mostPlayed" };
      case "playlist":
        return { kind: "playlist", id: view.id };
      default:
        return { kind: "library", query: query.trim() };
    }
  }, [view, query]);

  const activePlaylist =
    view.kind === "playlist"
      ? playlists.find((p) => p.id === view.id) ?? null
      : null;

  const loadFavorites = useCallback(() => {
    api
      .favoriteIds()
      .then((ids) => setFavorites(new Set(ids)))
      .catch(() => {});
  }, []);

  const loadPlaylists = useCallback(() => {
    api
      .listPlaylists()
      .then(setPlaylists)
      .catch(() => {});
  }, []);

  useEffect(() => {
    api.defaultMusicDir().then(setFolder).catch(() => {});
    loadFavorites();
    loadPlaylists();
  }, [loadFavorites, loadPlaylists]);

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;
    listen("history-changed", () => setVersion((v) => v + 1)).then((fn) => {
      if (disposed) fn();
      else unlisten = fn;
    });
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

  useEffect(() => {
    let cancelled = false;
    const delay = source.kind === "library" ? 150 : 0;
    const timer = setTimeout(() => {
      api
        .listSource(source)
        .then((result) => {
          if (cancelled) return;
          setTracks(result);
          setShownSource(source);
        })
        .catch((e) => {
          if (!cancelled) setMessage(String(e));
        });
    }, delay);

    return () => {
      cancelled = true;
      clearTimeout(timer);
    };
  }, [source, version]);

  async function scan() {
    setScanning(true);
    setMessage("Scanning…");
    try {
      const report = await api.scanFolder(folder);
      setMessage(describe(report));
      setVersion((v) => v + 1);
      loadFavorites();
    } catch (e) {
      setMessage(`Scan failed: ${e}`);
    } finally {
      setScanning(false);
    }
  }

  const play = useCallback(
    (index: number) => {
      api
        .playSource(shownSource, index)
        .catch((e) => setMessage(`Could not play: ${e}`));
    },
    [shownSource],
  );

  const toggleFavorite = useCallback((id: number) => {
    api
      .toggleFavorite(id)
      .then((isFavorite) => {
        setFavorites((prev) => {
          const next = new Set(prev);
          if (isFavorite) next.add(id);
          else next.delete(id);
          return next;
        });
        setVersion((v) => v + 1);
      })
      .catch((e) => setMessage(`Could not change favorite: ${e}`));
  }, []);

  async function createPlaylist(name: string): Promise<boolean> {
    const trimmed = name.trim();
    if (!trimmed) return false;
    try {
      const p = await api.createPlaylist(trimmed);
      loadPlaylists();
      setView({ kind: "playlist", id: p.id });
      return true;
    } catch (e) {
      setMessage(`Could not create playlist: ${e}`);
      return false;
    }
  }

  async function renamePlaylist(name: string): Promise<boolean> {
    if (view.kind !== "playlist") return false;
    try {
      await api.renamePlaylist(view.id, name.trim());
      loadPlaylists();
      return true;
    } catch (e) {
      setMessage(`Could not rename: ${e}`);
      return false;
    }
  }

  function deletePlaylist() {
    if (view.kind !== "playlist") return;
    api
      .deletePlaylist(view.id)
      .then(() => {
        setView({ kind: "library" });
        loadPlaylists();
      })
      .catch((e) => setMessage(`Could not delete: ${e}`));
  }

  function addToPlaylist(playlistId: number, trackId: number) {
    api
      .addToPlaylist(playlistId, [trackId])
      .then(() => {
        loadPlaylists();
        if (view.kind === "playlist" && view.id === playlistId) {
          setVersion((v) => v + 1);
        }
      })
      .catch((e) => setMessage(`Could not add to playlist: ${e}`));
  }

  function removeFromPlaylist(position: number) {
    if (view.kind !== "playlist") return;
    api
      .removeFromPlaylist(view.id, position)
      .then(() => {
        loadPlaylists();
        setVersion((v) => v + 1);
      })
      .catch((e) => setMessage(`Could not remove: ${e}`));
  }

  function moveInPlaylist(from: number, to: number) {
    if (view.kind !== "playlist") return;
    api
      .moveInPlaylist(view.id, from, to)
      .then(() => setVersion((v) => v + 1))
      .catch((e) => setMessage(`Could not reorder: ${e}`));
  }

  const emptyText =
    view.kind === "library" && query.trim()
      ? "No matches."
      : EMPTY_TEXT[view.kind] ?? "Nothing here.";

  return (
    <div className="app">
      <header className="toolbar">
        <input
          className="folder"
          value={folder}
          onChange={(e) => setFolder(e.target.value)}
          placeholder="Music folder"
        />
        <button onClick={scan} disabled={scanning || !folder.trim()}>
          {scanning ? "Scanning…" : "Scan"}
        </button>
        {view.kind === "library" && (
          <input
            className="search"
            type="search"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Search title, artist, album"
          />
        )}
      </header>

      <div className="body">
        <Sidebar
          view={view}
          playlists={playlists}
          onSelect={setView}
          onCreate={createPlaylist}
        />
        <div className="content">
          {activePlaylist && (
            <PlaylistHeader
              playlist={activePlaylist}
              onPlay={() => play(0)}
              onRename={renamePlaylist}
              onDelete={deletePlaylist}
            />
          )}
          <div className="status">
            {message || `${tracks.length} tracks`}
          </div>
          <main className="list">
            {tracks.length === 0 ? (
              <p className="empty">{emptyText}</p>
            ) : (
              <TrackTable
                tracks={tracks}
                playingPath={playingPath}
                favorites={favorites}
                playlists={playlists}
                showPlays={shownSource.kind === "mostPlayed"}
                playlistActions={
                  view.kind === "playlist"
                    ? {
                        onMove: moveInPlaylist,
                        onRemove: removeFromPlaylist,
                      }
                    : undefined
                }
                onPlay={play}
                onToggleFavorite={toggleFavorite}
                onAddToPlaylist={addToPlaylist}
              />
            )}
          </main>
        </div>
      </div>

      <NowPlayingBar status={status} onError={setMessage} />
    </div>
  );
}
