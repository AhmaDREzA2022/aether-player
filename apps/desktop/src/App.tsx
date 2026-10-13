import { useCallback, useEffect, useMemo, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { api, Source, SyncReport, Track } from "./api";
import { NowPlayingBar } from "./NowPlayingBar";
import { Sidebar, View } from "./Sidebar";
import { TrackTable } from "./TrackTable";
import { usePlayerStatus } from "./usePlayerStatus";
import { useShortcuts } from "./useShortcuts";
import "./App.css";

const EMPTY_TEXT: Record<View, string> = {
  library: "No tracks yet. Scan a folder to get started.",
  favorites: "No favorites yet. Click the heart next to a track.",
  recent: "Nothing played yet. A play counts after 30 seconds.",
  mostPlayed: "Nothing played yet. A play counts after 30 seconds.",
};

function describe(r: SyncReport): string {
  return `Scan finished: ${r.added} added, ${r.updated} updated, ${r.removed} removed, ${r.failed} failed`;
}

export default function App() {
  const [folder, setFolder] = useState("");
  const [view, setView] = useState<View>("library");
  const [query, setQuery] = useState("");
  const [tracks, setTracks] = useState<Track[]>([]);
  const [shownSource, setShownSource] = useState<Source>({ kind: "library", query: "" });
  const [favorites, setFavorites] = useState<Set<number>>(new Set());
  const [message, setMessage] = useState("");
  const [scanning, setScanning] = useState(false);
  const [version, setVersion] = useState(0); // bump this to reload the list

  const status = usePlayerStatus();
  useShortcuts(status, setMessage);
  const playingPath = status?.track?.path ?? null;

  // The list to load. `useMemo` keeps the same object between renders, so
  // the loading effect below only re-runs when the view or search changes.
  const source = useMemo<Source>(() => {
    switch (view) {
      case "favorites":
        return { kind: "favorites" };
      case "recent":
        return { kind: "recent" };
      case "mostPlayed":
        return { kind: "mostPlayed" };
      default:
        return { kind: "library", query: query.trim() };
    }
  }, [view, query]);

  const loadFavorites = useCallback(() => {
    api
      .favoriteIds()
      .then((ids) => setFavorites(new Set(ids)))
      .catch(() => {});
  }, []);

  useEffect(() => {
    api.defaultMusicDir().then(setFolder).catch(() => {});
    loadFavorites();
  }, [loadFavorites]);

  // Rust tells us when it stored a play, so "Recently played" stays current.
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

  // Load the list whenever the source changes or something bumps `version`.
  useEffect(() => {
    let cancelled = false;
    const delay = source.kind === "library" ? 150 : 0; // debounce typing only
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

  // Double-click: queue the list as shown and start at that row.
  const play = useCallback(
    (index: number) => {
      api.playSource(shownSource, index).catch((e) => setMessage(`Could not play: ${e}`));
    },
    [shownSource],
  );

  const toggleFavorite = useCallback((id: number) => {
    api
      .toggleFavorite(id)
      .then((isFavorite) => {
        // Build a new Set. React only notices a change when the object is new.
        setFavorites((prev) => {
          const next = new Set(prev);
          if (isFavorite) next.add(id);
          else next.delete(id);
          return next;
        });
        setVersion((v) => v + 1); // the Favorites view must drop an un-liked track
      })
      .catch((e) => setMessage(`Could not change favorite: ${e}`));
  }, []);

  const emptyText =
    view === "library" && query.trim() ? "No matches." : EMPTY_TEXT[view];

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
        {view === "library" && (
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
        <Sidebar view={view} onSelect={setView} />
        <div className="content">
          <div className="status">{message || `${tracks.length} tracks`}</div>
          <main className="list">
            {tracks.length === 0 ? (
              <p className="empty">{emptyText}</p>
            ) : (
              <TrackTable
                tracks={tracks}
                playingPath={playingPath}
                favorites={favorites}
                showPlays={shownSource.kind === "mostPlayed"}
                onPlay={play}
                onToggleFavorite={toggleFavorite}
              />
            )}
          </main>
        </div>
      </div>

      <NowPlayingBar status={status} onError={setMessage} />
    </div>
  );
}