import { CSSProperties, useEffect, useRef, useState } from "react";
import { Playlist } from "./api";

const MENU_WIDTH = 220;
const MENU_MAX_HEIGHT = 260;

interface Anchor {
  right: number;
  top: number;
  bottom: number;
}

function menuStyle(a: Anchor): CSSProperties {
  const left = Math.max(8, a.right - MENU_WIDTH);
  // Near the bottom of the window the menu opens upwards.
  const openUp = a.bottom + MENU_MAX_HEIGHT > window.innerHeight;
  return openUp
    ? { left, bottom: window.innerHeight - a.top }
    : { left, top: a.bottom };
}

interface Props {
  playlists: Playlist[];
  onPick: (playlistId: number) => void;
}

/** A "+" button that opens a small list of playlists to add a track to. */
export function AddToPlaylistMenu({ playlists, onPick }: Props) {
  const [anchor, setAnchor] = useState<Anchor | null>(null);
  const buttonRef = useRef<HTMLButtonElement>(null);
  const menuRef = useRef<HTMLDivElement>(null);
  const open = anchor !== null;

  // While open: close on an outside click, Escape, scrolling or resizing.
  useEffect(() => {
    if (!open) return;
    const close = () => setAnchor(null);
    const onPointerDown = (e: PointerEvent) => {
      const target = e.target as Node;
      if (
        menuRef.current?.contains(target) ||
        buttonRef.current?.contains(target)
      )
        return;
      close();
    };
    const onKeyDown = (e: KeyboardEvent) => e.key === "Escape" && close();

    window.addEventListener("pointerdown", onPointerDown);
    window.addEventListener("keydown", onKeyDown);
    window.addEventListener("resize", close);
    document.addEventListener("scroll", close, true);
    return () => {
      window.removeEventListener("pointerdown", onPointerDown);
      window.removeEventListener("keydown", onKeyDown);
      window.removeEventListener("resize", close);
      document.removeEventListener("scroll", close, true);
    };
  }, [open]);

  return (
    <>
      <button
        ref={buttonRef}
        className="icon"
        aria-label="Add to playlist"
        aria-haspopup="menu"
        aria-expanded={open}
        title="Add to playlist"
        onClick={(e) => {
          if (open) return setAnchor(null);
          const r = e.currentTarget.getBoundingClientRect();
          setAnchor({ right: r.right, top: r.top, bottom: r.bottom });
        }}
      >
        +
      </button>

      {anchor && (
        <div
          ref={menuRef}
          className="menu"
          role="menu"
          style={menuStyle(anchor)}
        >
          {playlists.length === 0 ? (
            <p className="menu-note">
              No playlists yet. Create one in the sidebar.
            </p>
          ) : (
            playlists.map((p) => (
              <button
                key={p.id}
                role="menuitem"
                onClick={() => {
                  onPick(p.id);
                  setAnchor(null);
                }}
              >
                {p.name}
              </button>
            ))
          )}
        </div>
      )}
    </>
  );
}
