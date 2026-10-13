import { memo } from "react";
import { Track } from "./api";
import { formatTime } from "./format";

interface Props {
  tracks: Track[];
  playingPath: string | null;
  onPlay: (index: number) => void;
}

// `memo` makes React skip re-drawing this table when its props are
// unchanged. The player status updates four times a second, and without
// this the whole list would be redrawn each time.
export const TrackTable = memo(function TrackTable({
  tracks,
  playingPath,
  onPlay,
}: Props) {
  return (
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
        {tracks.map((t, i) => (
          <tr
            key={t.id}
            className={t.path === playingPath ? "playing" : undefined}
            onDoubleClick={() => onPlay(i)}
          >
            <td>{t.title}</td>
            <td>{t.artist}</td>
            <td>{t.album}</td>
            <td className="time">{formatTime(t.durationMs)}</td>
          </tr>
        ))}
      </tbody>
    </table>
  );
});
