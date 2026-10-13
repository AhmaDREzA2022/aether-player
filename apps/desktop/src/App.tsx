import { useCallback, useEffect, useState } from "react";
import { api, SyncReport, Track } from "./api";
import { NowPlayingBar } from "./NowPlayingBar";
import { TrackTable } from "./TrackTable";
import { usePlayerStatus } from "./usePlayerStatus";
import "./App.css";

function describe(r: SyncReport): string {
  return `Scan finished: ${r.added} added, ${r.updated} updated, ${r.removed} removed, ${r.failed} failed`;
}

export default function App() {
  const [folder, setFolder] = useState("");
  const [query, setQuery] = useState("");
  const [tracks, setTracks] = useState<Track[]>([]);
  const [shownQuery, setShownQuery] = useState(""); // the query `tracks` came from
  const [message, setMessage] = useState("");
  const [scanning, setScanning] = useState(false);
  const [version, setVersion] = useState(0); // bump this to reload the list

  const status = usePlayerStatus();
  const playingPath = status?.track?.path ?? null;

  useEffect(() => {
    api
      .defaultMusicDir()
      .then(setFolder)
      .catch(() => {});
  }, []);

  useEffect(() => {
    let cancelled = false;
    const timer = setTimeout(() => {
      const request = query.trim() ? api.searchTracks(query) : api.listTracks();
      request
        .then((result) => {
          if (cancelled) return;
          setTracks(result);
          setShownQuery(query);
        })
        .catch((e) => {
          if (!cancelled) setMessage(String(e));
        });
    }, 150);

    return () => {
      cancelled = true;
      clearTimeout(timer);
    };
  }, [query, version]);

  async function scan() {
    setScanning(true);
    setMessage("Scanning…");
    try {
      const report = await api.scanFolder(folder);
      setMessage(describe(report));
      setVersion((v) => v + 1);
    } catch (e) {
      setMessage(`Scan failed: ${e}`);
    } finally {
      setScanning(false);
    }
  }

  // Double-click: queue the list as shown and start at that row.
  const play = useCallback(
    (index: number) => {
      api
        .playList(shownQuery, index)
        .catch((e) => setMessage(`Could not play: ${e}`));
    },
    [shownQuery],
  );

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
        <input
          className="search"
          type="search"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder="Search title, artist, album"
        />
      </header>

      <div className="status">{message || `${tracks.length} tracks`}</div>

      <main className="list">
        {tracks.length === 0 ? (
          <p className="empty">
            {query.trim()
              ? "No matches."
              : "No tracks yet. Scan a folder to get started."}
          </p>
        ) : (
          <TrackTable tracks={tracks} playingPath={playingPath} onPlay={play} />
        )}
      </main>

      <NowPlayingBar status={status} onError={setMessage} />
    </div>
  );
}
