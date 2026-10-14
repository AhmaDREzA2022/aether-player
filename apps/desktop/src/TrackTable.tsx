import { memo } from "react";
import { Playlist, Track } from "./api";
import { AddToPlaylistMenu } from "./AddToPlaylistMenu";
import { formatTime } from "./format";

/** Extra row buttons that only exist when viewing a playlist. */
export interface PlaylistActions {
  onMove: (from: number, to: number) => void;
  onRemove: (index: number) => void;
}

interface Props {
  tracks: Track[];
  playingPath: string | null;
  favorites: Set<number>;
  playlists: Playlist[];
  showPlays: boolean;
  playlistActions?: PlaylistActions;
  onPlay: (index: number) => void;
  onToggleFavorite: (trackId: number) => void;
  onAddToPlaylist: (playlistId: number, trackId: number) => void;
}

export const TrackTable = memo(function TrackTable({
  tracks,
  playingPath,
  favorites,
  playlists,
  showPlays,
  playlistActions,
  onPlay,
  onToggleFavorite,
  onAddToPlaylist,
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
          <th className="actions-col" aria-label="Actions" />
        </tr>
      </thead>
      <tbody>
        {tracks.map((t, i) => {
          const isFavorite = favorites.has(t.id);
          return (
            <tr
              // A playlist can hold the same track twice, so use the position.
              key={playlistActions ? `${t.id}-${i}` : t.id}
              className={t.path === playingPath ? "playing" : undefined}
              onDoubleClick={() => onPlay(i)}
            >
              <td className="fav-col">
                <button
                  className={isFavorite ? "icon fav on" : "icon fav"}
                  aria-label={
                    isFavorite ? "Remove from favorites" : "Add to favorites"
                  }
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
              <td
                className="actions-col"
                onDoubleClick={(e) => e.stopPropagation()}
              >
                {playlistActions && (
                  <>
                    <button
                      className="icon"
                      aria-label="Move up"
                      disabled={i === 0}
                      onClick={() => playlistActions.onMove(i, i - 1)}
                    >
                      ↑
                    </button>
                    <button
                      className="icon"
                      aria-label="Move down"
                      disabled={i === tracks.length - 1}
                      onClick={() => playlistActions.onMove(i, i + 1)}
                    >
                      ↓
                    </button>
                    <button
                      className="icon"
                      aria-label="Remove from playlist"
                      onClick={() => playlistActions.onRemove(i)}
                    >
                      ✕
                    </button>
                  </>
                )}
                <AddToPlaylistMenu
                  playlists={playlists}
                  onPick={(playlistId) => onAddToPlaylist(playlistId, t.id)}
                />
              </td>
            </tr>
          );
        })}
      </tbody>
    </table>
  );
});
