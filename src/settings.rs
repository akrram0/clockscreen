//! `%APPDATA%\clockscreen\settings.json`: where the clock sits and how big it is.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

pub const SCALE_MIN: f32 = 0.5;
pub const SCALE_MAX: f32 = 2.5;

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Settings {
    /// Physical desktop coordinates. `None` means "centre it on the primary display".
    #[serde(default)]
    pub x: Option<i32>,
    #[serde(default)]
    pub y: Option<i32>,
    #[serde(default = "one")]
    pub scale: f32,
}

fn one() -> f32 {
    1.0
}

impl Default for Settings {
    fn default() -> Self {
        Self { x: None, y: None, scale: 1.0 }
    }
}

impl Settings {
    /// Settings are user-editable, so nothing in the file is trusted.
    pub fn sanitized(mut self) -> Self {
        if !self.scale.is_finite() {
            self.scale = 1.0;
        }
        self.scale = self.scale.clamp(SCALE_MIN, SCALE_MAX);
        if self.x.is_some() != self.y.is_some() {
            self.x = None;
            self.y = None;
        }
        self
    }
}

pub fn path() -> PathBuf {
    let base = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("clockscreen").join("settings.json")
}

pub fn load() -> Settings {
    let Ok(text) = std::fs::read_to_string(path()) else {
        return Settings::default();
    };
    serde_json::from_str::<Settings>(&text)
        .unwrap_or_default()
        .sanitized()
}

pub fn save(settings: Settings) {
    let file = path();
    if let Some(dir) = file.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(text) = serde_json::to_string_pretty(&settings) {
        let _ = std::fs::write(&file, text + "\n");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn out_of_range_scale_is_clamped() {
        assert_eq!(Settings { scale: 9.0, ..Default::default() }.sanitized().scale, SCALE_MAX);
        assert_eq!(Settings { scale: 0.01, ..Default::default() }.sanitized().scale, SCALE_MIN);
        assert_eq!(Settings { scale: f32::NAN, ..Default::default() }.sanitized().scale, 1.0);
    }

    #[test]
    fn a_half_position_is_dropped() {
        let s = Settings { x: Some(10), y: None, scale: 1.0 }.sanitized();
        assert_eq!(s.x, None);
        assert_eq!(s.y, None);
    }

    #[test]
    fn missing_fields_fall_back() {
        let s: Settings = serde_json::from_str("{}").unwrap();
        assert_eq!(s.scale, 1.0);
        assert_eq!(s.x, None);
    }
}
