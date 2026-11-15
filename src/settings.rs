//! `%APPDATA%\clockscreen\settings.json`: Configuration persistence and Windows shell integration.

use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};
use serde::{Deserialize, Serialize};

pub const SCALE_MIN: f32 = 0.5;
pub const SCALE_MAX: f32 = 2.5;
pub const OPACITY_MIN: f32 = 0.20;
pub const OPACITY_MAX: f32 = 1.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Face {
    Glass,
    Hairline,
    Bold,
    Stacked,
    Condensed,
    Mono,
}

impl Default for Face {
    fn default() -> Self {
        Self::Glass
    }
}

impl Face {
    pub const ALL: [Face; 6] = [
        Face::Glass,
        Face::Hairline,
        Face::Bold,
        Face::Stacked,
        Face::Condensed,
        Face::Mono,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Face::Glass => "Glass",
            Face::Hairline => "Hairline",
            Face::Bold => "Bold",
            Face::Stacked => "Stacked",
            Face::Condensed => "Condensed",
            Face::Mono => "Mono",
        }
    }

    pub fn hint(self) -> &'static str {
        match self {
            Face::Glass => "Frosted chrome ramp & specular rim",
            Face::Hairline => "Crisp stroke outlines with wide tracking",
            Face::Bold => "Heavy squared techno digits",
            Face::Stacked => "Hours above minutes",
            Face::Condensed => "Narrow DIN-style block",
            Face::Mono => "Monospaced developer face",
        }
    }

    pub fn next(self) -> Self {
        let index = Self::ALL.iter().position(|f| *f == self).unwrap_or(0);
        Self::ALL[(index + 1) % Self::ALL.len()]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Tint {
    Auto,
    Chrome,
    Warm,
    Cool,
    Cyber,
}

impl Default for Tint {
    fn default() -> Self {
        Self::Chrome
    }
}

impl Tint {
    pub const ALL: [Tint; 5] = [
        Tint::Auto,
        Tint::Chrome,
        Tint::Warm,
        Tint::Cool,
        Tint::Cyber,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Tint::Auto => "Auto",
            Tint::Chrome => "Chrome",
            Tint::Warm => "Warm Gold",
            Tint::Cool => "Ice Blue",
            Tint::Cyber => "Cyber Cyan",
        }
    }

    pub fn next(self) -> Self {
        let index = Self::ALL.iter().position(|t| *t == self).unwrap_or(0);
        Self::ALL[(index + 1) % Self::ALL.len()]
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub x: Option<i32>,
    #[serde(default)]
    pub y: Option<i32>,
    #[serde(default = "default_scale")]
    pub scale: f32,
    #[serde(default)]
    pub face: Face,
    #[serde(default)]
    pub tint: Tint,
    #[serde(default = "default_true")]
    pub hour24: bool,
    #[serde(default = "default_opacity")]
    pub opacity: f32,
    #[serde(default)]
    pub click_through: bool,
    #[serde(default = "default_true")]
    pub depth_enabled: bool,
    #[serde(default)]
    pub mask_path: Option<String>,
    #[serde(default = "default_true")]
    pub show_date: bool,
    #[serde(default = "default_true")]
    pub show_island: bool,
    #[serde(default)]
    pub show_vinyl: bool,
}

fn default_scale() -> f32 {
    1.0
}
fn default_opacity() -> f32 {
    0.95
}
fn default_true() -> bool {
    true
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            x: None,
            y: None,
            scale: 1.0,
            face: Face::Glass,
            tint: Tint::Chrome,
            hour24: true,
            opacity: 0.95,
            click_through: false,
            depth_enabled: true,
            mask_path: None,
            show_date: true,
            show_island: true,
            show_vinyl: false,
        }
    }
}

impl Settings {
    pub fn sanitized(mut self) -> Self {
        if !self.scale.is_finite() {
            self.scale = 1.0;
        }
        self.scale = self.scale.clamp(SCALE_MIN, SCALE_MAX);
        if !self.opacity.is_finite() {
            self.opacity = 0.95;
        }
        self.opacity = self.opacity.clamp(OPACITY_MIN, OPACITY_MAX);
        if self.x.is_some() != self.y.is_some() {
            self.x = None;
            self.y = None;
        }
        self
    }
}

pub fn settings_path() -> PathBuf {
    if let Ok(app_data) = std::env::var("APPDATA") {
        PathBuf::from(app_data).join("clockscreen").join("settings.json")
    } else {
        PathBuf::from("settings.json")
    }
}

static CACHED_SETTINGS: OnceLock<Arc<Mutex<Settings>>> = OnceLock::new();

fn get_cache() -> &'static Arc<Mutex<Settings>> {
    CACHED_SETTINGS.get_or_init(|| {
        let loaded = load_from_disk();
        Arc::new(Mutex::new(loaded))
    })
}

fn load_from_disk() -> Settings {
    let path = settings_path();
    let contents = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return Settings::default(),
    };
    serde_json::from_str::<Settings>(&contents)
        .map(|s| s.sanitized())
        .unwrap_or_default()
}

pub fn load() -> Settings {
    get_cache().lock().map(|s| s.clone()).unwrap_or_default()
}

pub fn save(settings: Settings) {
    let sanitized = settings.sanitized();
    if let Ok(mut held) = get_cache().lock() {
        *held = sanitized.clone();
    }
    let path = settings_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(serialized) = serde_json::to_string_pretty(&sanitized) {
        let _ = std::fs::write(&path, serialized);
    }
}

const RUN_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
const RUN_VALUE: &str = "clockscreen";

pub fn start_with_windows() -> bool {
    std::process::Command::new("reg")
        .args(["query", RUN_KEY, "/v", RUN_VALUE])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

pub fn set_start_with_windows(enabled: bool) -> bool {
    let mut command = std::process::Command::new("reg");
    if enabled {
        let exe = std::env::current_exe().unwrap_or_default();
        let target = format!("\"{}\"", exe.display());
        command.args([
            "add", RUN_KEY, "/v", RUN_VALUE, "/t", "REG_SZ", "/d", target.as_str(), "/f",
        ]);
    } else {
        command.args(["delete", RUN_KEY, "/v", RUN_VALUE, "/f"]);
    }
    command
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_and_sanitization() {
        let mut s = Settings::default();
        s.scale = 999.0;
        s.opacity = -5.0;
        s = s.sanitized();
        assert_eq!(s.scale, SCALE_MAX);
        assert_eq!(s.opacity, OPACITY_MIN);
    }

    #[test]
    fn face_cycling() {
        let f = Face::Glass;
        assert_eq!(f.next(), Face::Hairline);
    }
}
