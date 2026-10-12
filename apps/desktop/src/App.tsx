import { useEffect, useState } from "react";
import { api, SyncReport, Track } from "./api";
import "./App.css";

function formatTime(ms: number): string {
  const total = Math.floor(ms / 1000);
  return `${Math.floor(total / 60)}:${String(total % 60).padStart(2, "0")}`;
}

function describe(r: SyncReport): string {
  return `Scan finished: ${r.added} added, ${r.updated} updated, ${r.removed} removed, ${r.failed} failed`;
}

export default function App() {
  const [folder, setFolder] = useState("");
  const [query, setQuery] = useState("");
  const [tracks, setTracks] = useState<Track[]>([]);
  const [message, setMessage] = useState("");
  const [scanning, setScanning] = useState(false);
  const [version, setVersion] = useState(0); // bump this to reload the list

  // Prefill the folder box once, when the app starts.
  useEffect(() => {
    api
      .defaultMusicDir()
      .then(setFolder)
      .catch(() => {});
  }, []);

  // Load the list whenever the search text changes or a scan finishes.
  useEffect(() => {
    let cancelled = false;
    const timer = setTimeout(() => {
      const request = query.trim() ? api.searchTracks(query) : api.listTracks();
      request
        .then((result) => {
          if (!cancelled) setTracks(result);
        })
        .catch((e) => {
          if (!cancelled) setMessage(String(e));
        });
    }, 150);

    // Runs before the next effect: drop the pending timer and ignore a
    // slow answer to an older query.
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
          <table>
            <thead>
              <tr>
                <th>Title</th>
                <th>Artist</th>
                <th>Album</th>
                <th className="time">Time</th>
              </tr>
            </thead>
            <tbody>
              {tracks.map((t) => (
                <tr key={t.id}>
                  <td>{t.title}</td>
                  <td>{t.artist}</td>
                  <td>{t.album}</td>
                  <td className="time">{formatTime(t.durationMs)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </main>
    </div>
  );
}
