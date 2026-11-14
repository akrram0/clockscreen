//! Media Session & Track Information provider.
//!
//! Feeds track metadata (title, artist, lyrics, playback progress)
//! to the Top Dynamic Island and Circular Vinyl Visualizer.

use std::sync::{Mutex, OnceLock};

#[derive(Clone, Debug)]
pub struct TrackInfo {
    pub title: String,
    pub artist: String,
    pub lyrics: String,
    pub is_playing: bool,
    pub progress: f32,
}

impl Default for TrackInfo {
    fn default() -> Self {
        Self {
            title: "The Box".to_string(),
            artist: "Roddy Ricch".to_string(),
            lyrics: "Shawty call me Crisco\n'cause I pop my shit".to_string(),
            is_playing: true,
            progress: 0.45,
        }
    }
}

static CURRENT_TRACK: OnceLock<Mutex<TrackInfo>> = OnceLock::new();

pub fn get_current_track() -> TrackInfo {
    CURRENT_TRACK
        .get_or_init(|| Mutex::new(TrackInfo::default()))
        .lock()
        .map(|guard| guard.clone())
        .unwrap_or_default()
}

pub fn set_track(title: String, artist: String, lyrics: String) {
    if let Ok(mut guard) = CURRENT_TRACK.get_or_init(|| Mutex::new(TrackInfo::default())).lock() {
        guard.title = title;
        guard.artist = artist;
        guard.lyrics = lyrics;
    }
}
