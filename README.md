# Aether Player

**Modular cross-platform music player** with multi-device sync and streaming.

Built in Rust. Desktop (Tauri) + Android. Designed to be clean, expandable, and résumé-worthy.

## Goals

- Local music library management & playback
- Real-time multi-device sync (queue, status, volume, position)
- Streaming (phone → desktop and bidirectional)
- Professional pairing (QR + mDNS + auth token)
- Power-user features (lyrics, equalizer, visualizer, Discord RPC, Last.fm, etc.)

## Architecture

```
aether-player/
├── crates/
│   ├── core/          # Queue, Playlist, Player API, library
│   ├── audio/         # Playback backends (rodio → gstreamer)
│   ├── metadata/      # ID3 / FLAC / etc. parsing (lofty)
│   ├── sync/          # WebSocket protocol
│   ├── discovery/     # mDNS + pairing
│   └── common/        # Shared utilities
├── apps/
│   ├── desktop/       # Tauri (Linux first)
│   └── android/
├── protocol/          # Shared DTOs (serde)
└── docs/
```

### Core design rule

Everything goes through a `Player` trait:

```rust
trait Player {
    fn play(&self);
    fn pause(&self);
    fn seek(&self, pos: Duration);
    fn next(&self);
    fn prev(&self);
    fn set_volume(&self, vol: f32);
    fn status(&self) -> PlayerStatus;
}
```

- `LocalPlayer` → on-device playback
- `RemotePlayer` → talks over WebSocket

The UI never knows (or cares) whether the music is local or remote.

## Version Roadmap

| Version | Focus |
|---------|-------|
| **V1**  | Local player foundation (scan, playlist, playback, search, favorites, MPRIS…) |
| **V2**  | Multi-device sync (WebSocket) |
| **V3**  | Streaming (phone ↔ desktop) |
| **V4**  | Professional pairing & discovery (QR, mDNS, auth) |
| **V5**  | Power features (lyrics, EQ, visualizer, Discord, Last.fm…) |

## License

GPL-3.0-only

## Status

🚧 Early development – project skeleton only.
