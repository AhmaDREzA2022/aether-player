import { memo } from "react";
import { Track } from "./api";
import { formatTime } from "./format";

interface Props {
  tracks: Track[];
  playingPath: string | null;
  favorites: Set<number>;
  showPlays: boolean;
  onPlay: (index: number) => void;
  onToggleFavorite: (trackId: number) => void;
}

export const TrackTable = memo(function TrackTable({
  tracks,
  playingPath,
  favorites,
  showPlays,
  onPlay,
  onToggleFavorite,
}: Props) {
  return (
    <table>
      <thead>
        <tr>
          <th className="fav-col" aria-label="Favorite" />
          <th>Title</th>
          <th>Artist</th>
          <th>Album</th>
          {showPlays && <th className="time">Plays</th>}
          <th className="time">Time</th>
        </tr>
      </thead>
      <tbody>
        {tracks.map((t, i) => {
          const isFavorite = favorites.has(t.id);
          return (
            <tr
              key={t.id}
              className={t.path === playingPath ? "playing" : undefined}
              onDoubleClick={() => onPlay(i)}
            >
              <td className="fav-col">
                <button
                  className={isFavorite ? "fav on" : "fav"}
                  aria-label={isFavorite ? "Remove from favorites" : "Add to favorites"}
                  aria-pressed={isFavorite}
                  onClick={() => onToggleFavorite(t.id)}
                  onDoubleClick={(e) => e.stopPropagation()}
                >
                  {isFavorite ? "♥" : "♡"}
                </button>
              </td>
              <td>{t.title}</td>
              <td>{t.artist}</td>
              <td>{t.album}</td>
              {showPlays && <td className="time">{t.plays}</td>}
              <td className="time">{formatTime(t.durationMs)}</td>
            </tr>
          );
        })}
      </tbody>
    </table>
  );
});