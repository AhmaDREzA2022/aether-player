import { useEffect, useRef } from "react";
import { api, PlayerStatus } from "./api";
import { shortcutFor } from "./shortcuts";

const SEEK_STEP_MS = 5000;

export function useShortcuts(status: PlayerStatus | null, onError: (message: string) => void) {
  // The status changes four times a second. Keeping the newest one in a ref
  // lets the key listener read it without being re-registered each time.
  const statusRef = useRef(status);
  statusRef.current = status;

  // The volume to go back to when un-muting.
  const lastVolume = useRef(0.5);

  useEffect(() => {
    const run = (action: Promise<unknown>) => action.catch((e) => onError(String(e)));

    function onKeyDown(e: KeyboardEvent) {
      const action = shortcutFor(e, e.target);
      if (!action) return;
      e.preventDefault(); // stop Space from scrolling the page, arrows from moving it

      const s = statusRef.current;
      switch (action) {
        case "toggle":
          run(api.toggle());
          break;
        case "next":
          run(api.next());
          break;
        case "prev":
          run(api.prev());
          break;
        case "seekBack":
        case "seekForward": {
          if (!s?.track) return;
          const delta = action === "seekBack" ? -SEEK_STEP_MS : SEEK_STEP_MS;
          let target = Math.max(s.positionMs + delta, 0);
          if (s.durationMs > 0) target = Math.min(target, s.durationMs);
          run(api.seek(target));
          break;
        }
        case "mute":
          if (!s) return;
          if (s.volume > 0) {
            lastVolume.current = s.volume;
            run(api.setVolume(0));
          } else {
            run(api.setVolume(lastVolume.current));
          }
          break;
      }
    }

    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [onError]);
}
