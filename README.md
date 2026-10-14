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
│   ├── cli/           # Terminal player for testing the core
│   ├── desktop/       # Tauri (Linux first)
│   └── android/
├── protocol/          # Shared DTOs (serde)
└── docs/
```

### Core design rule

Everything goes through a `Player` trait:

```rust
pub trait Player: Send + Sync {
    fn load(&self, track: &Track) -> Result<(), PlayerError>;
    fn play(&self) -> Result<(), PlayerError>;
    fn pause(&self) -> Result<(), PlayerError>;
    fn seek(&self, pos: Duration) -> Result<(), PlayerError>;
    fn next(&self) -> Result<(), PlayerError>;
    fn prev(&self) -> Result<(), PlayerError>;
    fn set_volume(&self, vol: f32) -> Result<(), PlayerError>;
    fn status(&self) -> Result<PlayerStatus, PlayerError>;
}
```

- `LocalPlayer` → on-device playback (rodio, on a dedicated audio thread)
- `PlaybackController` → wraps any `Player`, adds the queue so `next`/`prev` work
- `RemotePlayer` → talks over WebSocket (V2)

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

## Building

Requirements: Rust (stable). On Debian/Ubuntu/Kali: `sudo apt install libasound2-dev pkg-config`.

```bash
cargo test --workspace
cargo run -p aether-cli -- --list ~/Music      # scan and print, no audio
cargo run --release -p aether-cli -- ~/Music   # play a folder
```

## Status

| Milestone | State |
|-----------|-------|
| V1.0 Playback core | ✅ done |
| V1.1 Library & history | ✅ done |
| V1.2 Desktop shell (Tauri) | ✅ done |
| V1.3 Desktop polish | 🚧 in progress |
| V1.4 Linux integration (MPRIS) | planned |
