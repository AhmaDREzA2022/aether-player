import { useEffect, useState } from "react";
import { api, PlayerStatus } from "./api";

/** Polls the player a few times a second and returns the latest status. */
export function usePlayerStatus(intervalMs = 250): PlayerStatus | null {
  const [status, setStatus] = useState<PlayerStatus | null>(null);

  useEffect(() => {
    let stopped = false;
    const poll = () =>
      api
        .status()
        .then((s) => {
          if (!stopped) setStatus(s);
        })
        .catch(() => {
          /* keep showing the last status */
        });

    poll();
    const timer = setInterval(poll, intervalMs);
    return () => {
      stopped = true;
      clearInterval(timer);
    };
  }, [intervalMs]);

  return status;
}
