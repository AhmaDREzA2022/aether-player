use aether_common::{PlaybackState, Player, PlayerError, PlayerStatus, Track};
use rodio::Player as RodioPlayer;
use std::error::Error;
use std::future::ready;
use std::task::Poll::Ready;
use std::{
    fs::File,
    io::{self, BufReader},
    path::Path,
    sync::{mpsc, Mutex},
    thread,
    time::Duration,
};

type Replay<T> = mpsc::Sender<Result<T, PlayerError>>;

/// A command sent to the audio thread, with a channel for the answer,
enum Cmd {
    Load(Track, Replay<()>),
    Play(Replay<()>),
    Pause(Replay<()>),
    Seek(Duration, Replay<()>),
    SetVolume(f32, Replay<()>),
    Status(Replay<PlayerStatus>),
}

/// On-device playback. all rodio objects live on one background thread.
pub struct LocalPlayer {
    tx: Mutex<mpsc::Sender<Cmd>>,
}

impl LocalPlayer {
    /// opens the default audio device. returns an error instead of panicking
    /// when there si no device.
    pub fn new() -> Result<Self, PlayerError> {
        let (tx, rx) = mpsc::channel();
        let (ready_tx, ready_rx) = mpsc::channel();

        thread::Builder::new()
            .name("aether-audio".into())
            .spawn(move || audio_thread(rx, ready_tx))
            .map_err(|e| PlayerError::Io(e.to_string()))?;

        // Wait fo the thread to report whether the device opened.
        ready_rx.recv().map_err(|_| PlayerError::NoAudioDevice)??;

        Ok(Self { tx: Mutex::new(tx) })
    }

    /// send a command to the audio thread and wait for its reply.
    fn call<T>(&self, make: impl FnOnce(Replay<T>) -> Cmd) -> Result<T, PlayerError> {
        let (replay_tx, replay_rx) = mpsc::channel();
        self.tx
            .lock()
            .map_err(|_| PlayerError::DeviceLost)?
            .send(make(replay_tx))
            .map_err(|_| PlayerError::DeviceLost)?;
        replay_rx.recv().map_err(|_| PlayerError::DeviceLost)?
    }
}

impl Player for LocalPlayer {
    fn load(&self, track: &Track) -> Result<(), PlayerError> {
        let track = track.clone();
        self.call(|replay| Cmd::Load(track, replay))
    }
    fn play(&self) -> Result<(), PlayerError> {
        self.call(Cmd::Play)
    }

    fn pause(&self) -> Result<(), PlayerError> {
        self.call(Cmd::Pause)
    }

    fn seek(&self, pos: Duration) -> Result<(), PlayerError> {
        self.call(|reply| Cmd::Seek(pos, reply))
    }

    // A single device has no queue; the controller in `core` handles these.
    fn next(&self) -> Result<(), PlayerError> {
        Err(PlayerError::NotSupported)
    }

    fn prev(&self) -> Result<(), PlayerError> {
        Err(PlayerError::NotSupported)
    }

    fn set_volume(&self, vol: f32) -> Result<(), PlayerError> {
        self.call(|reply| Cmd::SetVolume(vol, reply))
    }

    fn status(&self) -> Result<PlayerStatus, PlayerError> {
        self.call(Cmd::Status)
    }
}

// ---------------- everything below runs on the audio thread ----------------

fn audio_thread(rx: mpsc::Receiver<Cmd>, ready: mpsc::Sender<Result<(), PlayerError>>) {
    let device = match rodio::DeviceSinkBuilder::open_default_sink() {
        Ok(device) => device,
        Err(e) => {
            tracing::error!("could not open audio deciveL {e}");
            let _ = ready.send(Err(PlayerError::NoAudioDevice));
            return;
        }
    };
    let _ = ready.send(Ok(()));

    let mut engine = Engine {
        device,
        player: None,
        track: None,
        volume: 1.0,
        started: false,
    };

    // Ends when every shender is dropped, ie. when LocalPlayer is dropped.
    while let Ok(cmd) = rx.recv() {
        engine.handle(cmd);
    }
}

struct Engine {
    device: rodio::MixerDeviceSink,
    player: Option<RodioPlayer>,
    track: Option<Track>,
    volume: f32,
    started: bool,
}

impl Engine {
    fn handle(&mut self, cmd: Cmd) {
        match cmd {
            Cmd::Load(track, reply) => {
                let _ = reply.send(self.load(track));
            }
            Cmd::Play(reply) => {
                let _ = reply.send(self.play());
            }
            Cmd::Pause(reply) => {
                let _ = reply.send(self.pause());
            }
            Cmd::Seek(pos, reply) => {
                let _ = reply.send(self.seek(pos));
            }
            Cmd::SetVolume(vol, reply) => {
                let _ = reply.send(self.set_volume(vol));
            }
            Cmd::Status(reply) => {
                let _ = reply.send(self.status());
            }
        }
    }

    fn load(&mut self, track: Track) -> Result<(), PlayerError> {
        let source = open_source(&track.path)?;

        let player = RodioPlayer::connect_new(self.device.mixer());
        player.pause(); // start paused; `play()` starts it
        player.set_volume(self.volume);
        player.append(source);

        self.player = Some(player); // the old player is dropped, which stops it
        self.track = Some(track);
        self.started = false;
        Ok(())
    }

    fn play(&mut self) -> Result<(), PlayerError> {
        let player = self.player.as_ref().ok_or_else(no_track)?;
        player.play();
        self.started = true;
        Ok(())
    }

    fn pause(&mut self) -> Result<(), PlayerError> {
        if let Some(player) = &self.player {
            player.pause();
        }
        Ok(())
    }

    fn seek(&mut self, pos: Duration) -> Result<(), PlayerError> {
        let player = self.player.as_ref().ok_or_else(no_track)?;
        let duration = self.track.as_ref().map(|t| t.duration).unwrap_or_default();
        let pos = if duration.is_zero() {
            pos
        } else {
            pos.min(duration)
        };

        player.try_seek(pos).map_err(|e| {
            tracing::warn!("seek failed: {e}");
            PlayerError::SeekUnsupported
        })
    }

    fn set_volume(&mut self, vol: f32) -> Result<(), PlayerError> {
        self.volume = vol.clamp(0.0, 1.0);
        if let Some(player) = &self.player {
            player.set_volume(self.volume);
        }
        Ok(())
    }

    fn status(&self) -> Result<PlayerStatus, PlayerError> {
        let duration = self.track.as_ref().map(|t| t.duration).unwrap_or_default();

        let (state, position) = match &self.player {
            None => (PlaybackState::Stopped, Duration::ZERO),
            Some(p) if p.empty() => (PlaybackState::Finished, duration),
            Some(_) if !self.started => (PlaybackState::Stopped, Duration::ZERO),
            Some(p) => {
                let state = if p.is_paused() {
                    PlaybackState::Paused
                } else {
                    PlaybackState::Playing
                };
                // rodio's position can overshoot the duration, so clamp it.
                let pos = p.get_pos();
                (
                    state,
                    if duration.is_zero() {
                        pos
                    } else {
                        pos.min(duration)
                    },
                )
            }
        };

        Ok(PlayerStatus {
            state,
            position,
            duration,
            volume: self.volume,
            track: self.track.clone(),
        })
    }
}

fn no_track() -> PlayerError {
    PlayerError::NotFound("no track loaded".into())
}

/// Open and decode a file. Maps every failure to a `PlayerError`.
/// `Decoder::try_from(File)` is used because it gives the decoder the file
/// length, which seeking needs for formats like MP3.
fn open_source(path: &Path) -> Result<rodio::Decoder<BufReader<File>>, PlayerError> {
    let file = File::open(path).map_err(|e| match e.kind() {
        io::ErrorKind::NotFound => PlayerError::NotFound(path.display().to_string()),
        _ => PlayerError::Io(format!("{}: {e}", path.display())),
    })?;

    rodio::Decoder::try_from(file)
        .map_err(|e| PlayerError::UnsupportedFormat(format!("{}: {e}", path.display())))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn write_silent_wav(path: &Path, seconds: u32) {
        let sample_rate: u32 = 8000;
        let data_len = sample_rate * 2 * seconds;
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&(36 + data_len).to_le_bytes());
        bytes.extend_from_slice(b"WAVEfmt ");
        bytes.extend_from_slice(&16u32.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&sample_rate.to_le_bytes());
        bytes.extend_from_slice(&(sample_rate * 2).to_le_bytes());
        bytes.extend_from_slice(&2u16.to_le_bytes());
        bytes.extend_from_slice(&16u16.to_le_bytes());
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&data_len.to_le_bytes());
        bytes.extend(std::iter::repeat(0u8).take(data_len as usize));
        fs::write(path, bytes).unwrap();
    }

    #[test]
    fn local_player_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<LocalPlayer>();
    }

    #[test]
    fn missing_file_is_not_found() {
        let err = open_source(Path::new("/nope/missing.mp3")).err().unwrap();
        assert!(matches!(err, PlayerError::NotFound(_)));
    }

    #[test]
    fn garbage_file_is_unsupported_format() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bad.mp3");
        fs::write(&path, "this is not audio").unwrap();

        let err = open_source(&path).err().unwrap();
        assert!(matches!(err, PlayerError::UnsupportedFormat(_)));
    }

    #[test]
    #[ignore = "needs an audio device; run with --ignored"]
    fn plays_pauses_seeks_and_finishes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("silence.wav");
        write_silent_wav(&path, 2);

        let track = Track {
            path,
            title: "silence".into(),
            artist: "Unknown artist".into(),
            album: "Unknown album".into(),
            duration: Duration::from_secs(2),
            track_no: None,
        };

        let p = LocalPlayer::new().unwrap();
        p.load(&track).unwrap();
        assert_eq!(p.status().unwrap().state, PlaybackState::Stopped);

        p.play().unwrap();
        assert_eq!(p.status().unwrap().state, PlaybackState::Playing);

        p.pause().unwrap();
        assert_eq!(p.status().unwrap().state, PlaybackState::Paused);

        p.set_volume(0.5).unwrap();
        assert_eq!(p.status().unwrap().volume, 0.5);

        p.play().unwrap();
        p.seek(Duration::from_millis(1900)).unwrap();
        thread::sleep(Duration::from_millis(500));
        assert_eq!(p.status().unwrap().state, PlaybackState::Finished);
    }
}
