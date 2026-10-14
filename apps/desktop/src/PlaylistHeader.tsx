import { useEffect, useState } from "react";
import { Playlist } from "./api";

interface Props {
  playlist: Playlist;
  onPlay: () => void;
  /** Resolves to true when the rename succeeded. */
  onRename: (name: string) => Promise<boolean>;
  onDelete: () => void;
}

export function PlaylistHeader({
  playlist,
  onPlay,
  onRename,
  onDelete,
}: Props) {
  const [editing, setEditing] = useState(false);
  const [name, setName] = useState(playlist.name);
  const [confirming, setConfirming] = useState(false);

  // Switching playlists (or a finished rename) resets the local state.
  useEffect(() => {
    setEditing(false);
    setConfirming(false);
    setName(playlist.name);
  }, [playlist.id, playlist.name]);

  // The delete button disarms itself after a few seconds.
  useEffect(() => {
    if (!confirming) return;
    const timer = setTimeout(() => setConfirming(false), 3000);
    return () => clearTimeout(timer);
  }, [confirming]);

  async function save() {
    if (await onRename(name)) setEditing(false);
  }

  return (
    <div className="playlist-header">
      {editing ? (
        <input
          className="rename"
          autoFocus
          aria-label="Playlist name"
          value={name}
          onChange={(e) => setName(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") save();
            if (e.key === "Escape") {
              setEditing(false);
              setName(playlist.name);
            }
          }}
        />
      ) : (
        <h2>{playlist.name}</h2>
      )}
      <span className="count">{playlist.trackCount} tracks</span>
      <div className="spacer" />
      <button onClick={onPlay} disabled={playlist.trackCount === 0}>
        Play
      </button>
      {editing ? (
        <button className="secondary" onClick={save}>
          Save
        </button>
      ) : (
        <button className="secondary" onClick={() => setEditing(true)}>
          Rename
        </button>
      )}
      <button
        className={confirming ? "danger" : "secondary"}
        onClick={() => (confirming ? onDelete() : setConfirming(true))}
      >
        {confirming ? "Click again to delete" : "Delete"}
      </button>
    </div>
  );
}
