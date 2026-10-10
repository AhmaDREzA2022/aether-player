use crate::{error::PlayerError, Track};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlaybackState {
    Playing,
    Paused,
    Stopped,
    Finished,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerStatus {
    pub state: PlaybackState,
    pub position: Duration,
    pub duration: Duration,
    pub volume: f32,
    pub track: Option<Track>,
}

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
