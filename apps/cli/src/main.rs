use aether_audio::LocalPlayer;
use aether_common::{PlaybackState, Player, PlayerError};
use aether_core::{spawn_ticker, PlaybackController, DEFAULT_TICK_INTERVAL};
use aether_metadata::scan_folder;
use std::{
    error::Error,
    io::{self, BufRead},
    path::PathBuf,
    process::ExitCode,
    sync::Arc,
    thread,
    time::Duration,
};
use tracing_subscriber::EnvFilter;

const SEEK_STEP: Duration = Duration::from_secs(10);

const HELP: &str = "\
Commands (type a letter, then Enter):
  p       pause / resume
  n       next track
  b       previous track (restarts the track if more than 3s in)
  f / r   seek forward / back 10 seconds
  + / -   volume up / down
  i       show status
  h       show this help
  q       quit";

enum Flow {
    Continue,
    Quit,
}

fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn")),
        )
        .init();

    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let list_only = args.iter().any(|a| a == "--list");
    let Some(path) = args
        .iter()
        .find(|a| !a.starts_with("--"))
        .map(PathBuf::from)
    else {
        return Err("usage: aether-cli [--list] <music folder>".into());
    };

    let tracks = scan_folder(&path);
    if tracks.is_empty() {
        return Err(format!("no supported audio files found in {}", path.display()).into());
    }
    println!("Found {} tracks", tracks.len());

    if list_only {
        for (i, t) in tracks.iter().enumerate() {
            println!(
                "{:>3}. {} - {} [{}] {}",
                i + 1,
                t.artist,
                t.title,
                t.album,
                format_time(t.duration)
            );
        }
        return Ok(());
    }

    let player = LocalPlayer::new()?;
    let controller = Arc::new(PlaybackController::new(Arc::new(player)));
    controller.enqueue(tracks);

    let _ticker = spawn_ticker(&controller, DEFAULT_TICK_INTERVAL)?;
    spawn_announcer(&controller);

    controller.play()?;
    println!("{HELP}");

    for line in io::stdin().lock().lines() {
        match handle(&controller, line?.trim()) {
            Ok(Flow::Continue) => {}
            Ok(Flow::Quit) => break,
            Err(e) => println!("error: {e}"),
        }
    }
    Ok(())
}

fn handle(c: &PlaybackController, cmd: &str) -> Result<Flow, PlayerError> {
    match cmd {
        "" => {}
        "p" => {
            if c.status()?.state == PlaybackState::Playing {
                c.pause()?;
            } else {
                c.play()?;
            }
        }
        "n" => c.next()?,
        "b" => c.prev()?,
        "f" => {
            let pos = c.status()?.position;
            c.seek(pos + SEEK_STEP)?;
        }
        "r" => {
            let pos = c.status()?.position;
            c.seek(pos.saturating_sub(SEEK_STEP))?;
        }
        "+" | "-" => {
            let delta = if cmd == "+" { 0.1 } else { -0.1 };
            let volume = step_volume(c.status()?.volume, delta);
            c.set_volume(volume)?;
            println!("Volume: {:.0}%", volume * 100.0);
        }
        "i" => print_status(c),
        "h" => println!("{HELP}"),
        "q" => return Ok(Flow::Quit),
        other => println!("unknown command '{other}' (h for help)"),
    }
    Ok(Flow::Continue)
}

/// Prints a line whenever the current track changes, including when the
/// ticker auto-advances. It runs on its own thread because the main thread
/// is busy waiting for keyboard input.
fn spawn_announcer(controller: &Arc<PlaybackController>) {
    let weak = Arc::downgrade(controller);
    thread::spawn(move || {
        let mut last: Option<PathBuf> = None;
        loop {
            thread::sleep(Duration::from_millis(250));
            let Some(c) = weak.upgrade() else { break };
            let Ok(status) = c.status() else { continue };

            if let Some(track) = status.track {
                if last.as_ref() != Some(&track.path) {
                    println!(
                        "▶ {} - {} ({})",
                        track.artist,
                        track.title,
                        format_time(track.duration)
                    );
                    last = Some(track.path.clone());
                }
            }
        }
    });
}

fn print_status(c: &PlaybackController) {
    match c.status() {
        Ok(s) => {
            let title = s
                .track
                .as_ref()
                .map(|t| format!("{} - {}", t.artist, t.title))
                .unwrap_or_else(|| "(nothing loaded)".to_string());
            println!(
                "{:?}  {}  {} / {}  volume {:.0}%",
                s.state,
                title,
                format_time(s.position),
                format_time(s.duration),
                s.volume * 100.0
            );
        }
        Err(e) => println!("error: {e}"),
    }
}

fn format_time(d: Duration) -> String {
    let secs = d.as_secs();
    format!("{}:{:02}", secs / 60, secs % 60)
}

/// Change the volume by `delta`, rounded to whole percent and kept in 0..=1.
fn step_volume(volume: f32, delta: f32) -> f32 {
    (((volume + delta) * 100.0).round() / 100.0).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_time_shows_minutes_and_padded_seconds() {
        assert_eq!(format_time(Duration::ZERO), "0:00");
        assert_eq!(format_time(Duration::from_secs(125)), "2:05");
    }

    #[test]
    fn step_volume_stays_between_zero_and_one() {
        assert!((step_volume(0.5, 0.1) - 0.6).abs() < 1e-6);
        assert_eq!(step_volume(0.95, 0.1), 1.0);
        assert_eq!(step_volume(0.05, -0.1), 0.0);
    }
}
