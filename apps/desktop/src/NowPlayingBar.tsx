import { api, PlayerStatus } from "./api";
import { formatTime } from "./format";
import { Slider } from "./Slider";

interface Props {
  status: PlayerStatus | null;
  onError: (message: string) => void;
}

export function NowPlayingBar({ status, onError }: Props) {
  const run = (action: Promise<unknown>) =>
    action.catch((e) => onError(String(e)));
  const track = status?.track ?? null;
  const playing = status?.state === "playing";

  return (
    <footer className="player">
      <div className="now">
        {track ? (
          <>
            <div className="now-title">{track.title}</div>
            <div className="now-artist">{track.artist}</div>
          </>
        ) : (
          <div className="now-artist">Nothing playing</div>
        )}
      </div>

      <div className="transport">
        <div className="buttons">
          <button onClick={() => run(api.prev())} disabled={!track}>
            Prev
          </button>
          <button onClick={() => run(api.toggle())} disabled={!track}>
            {playing ? "Pause" : "Play"}
          </button>
          <button onClick={() => run(api.next())} disabled={!track}>
            Next
          </button>
        </div>
        <div className="seek">
          <span className="clock">{formatTime(status?.positionMs ?? 0)}</span>
          <Slider
            label="Seek"
            value={status?.positionMs ?? 0}
            max={status?.durationMs ?? 0}
            step={250}
            disabled={!track}
            onCommit={(ms) => run(api.seek(ms))}
          />
          <span className="clock">{formatTime(status?.durationMs ?? 0)}</span>
        </div>
      </div>

      <div className="volume">
        <span>Vol</span>
        <Slider
          label="Volume"
          value={Math.round((status?.volume ?? 1) * 100)}
          max={100}
          step={1}
          onLive={(v) => run(api.setVolume(v / 100))}
        />
      </div>
    </footer>
  );
}
