pub mod mpris;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlaybackStatus {
    Playing,
    Paused,
    Stopped,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct TrackMetadata {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration_sec: u64,
}

#[derive(Debug, Clone)]
pub struct PlayerState {
    pub metadata: TrackMetadata,
    pub position_sec: f64,
}

impl Default for PlayerState {
    fn default() -> Self {
        Self {
            metadata: TrackMetadata::default(),
            position_sec: 0.0,
        }
    }
}
