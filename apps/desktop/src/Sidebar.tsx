import { FormEvent, useState } from "react";
import { Playlist } from "./api";

export type View =
  | { kind: "library" }
  | { kind: "favorites" }
  | { kind: "recent" }
  | { kind: "mostPlayed" }
  | { kind: "playlist"; id: number };

const MAIN_ITEMS = [
  { kind: "library", label: "Library" },
  { kind: "favorites", label: "Favorites" },
  { kind: "recent", label: "Recently played" },
  { kind: "mostPlayed", label: "Most played" },
] as const;

interface Props {
  view: View;
  playlists: Playlist[];
  onSelect: (view: View) => void;
  /** Resolves to true when the playlist was created. */
  onCreate: (name: string) => Promise<boolean>;
}

export function Sidebar({ view, playlists, onSelect, onCreate }: Props) {
  const [creating, setCreating] = useState(false);
  const [name, setName] = useState("");

  async function submit(e: FormEvent) {
    e.preventDefault();
    if (await onCreate(name)) {
      setName("");
      setCreating(false);
    }
  }

  return (
    <nav className="sidebar" aria-label="Views">
      {MAIN_ITEMS.map((item) => (
        <button
          key={item.kind}
          className={view.kind === item.kind ? "active" : undefined}
          aria-current={view.kind === item.kind ? "page" : undefined}
          onClick={() => onSelect({ kind: item.kind })}
        >
          {item.label}
        </button>
      ))}

      <div className="sidebar-heading">
        <span>Playlists</span>
        <button
          className="icon"
          aria-label="New playlist"
          title="New playlist"
          onClick={() => setCreating((c) => !c)}
        >
          +
        </button>
      </div>

      {creating && (
        <form className="new-playlist" onSubmit={submit}>
          <input
            autoFocus
            value={name}
            onChange={(e) => setName(e.target.value)}
            onKeyDown={(e) => e.key === "Escape" && setCreating(false)}
            placeholder="Playlist name"
            aria-label="New playlist name"
          />
        </form>
      )}

      <div className="playlists">
        {playlists.map((p) => {
          const active = view.kind === "playlist" && view.id === p.id;
          return (
            <button
              key={p.id}
              className={active ? "active" : undefined}
              aria-current={active ? "page" : undefined}
              onClick={() => onSelect({ kind: "playlist", id: p.id })}
            >
              {p.name}
            </button>
          );
        })}
        {playlists.length === 0 && !creating && (
          <p className="sidebar-note">No playlists yet.</p>
        )}
      </div>
    </nav>
  );
}
