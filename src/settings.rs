//! `%APPDATA%\clockscreen\settings.json`: where the clock sits, how big it is, and which face
//! it wears.
//!
//! Everything here is user-editable, so nothing in the file is trusted: every field is
//! `#[serde(default)]`-guarded, unknown names fall back to their default instead of failing the
//! whole file, and `sanitized()` clamps what the view depends on.

use std::borrow::Cow;
use std::path::PathBuf;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

pub const SCALE_MIN: f32 = 0.5;
pub const SCALE_MAX: f32 = 2.5;
pub const OPACITY_MIN: f32 = 0.25;
pub const OPACITY_MAX: f32 = 1.0;

/// Serialises as a stable kebab-case id, and reads an unknown id as `Default::default()`.
///
/// A hand-edited typo should cost you one setting, not the whole file.
macro_rules! named_enum {
    ($ty:ty { $($variant:ident => $id:literal),+ $(,)? }) => {
        impl $ty {
            pub fn id(self) -> &'static str {
                match self { $(<$ty>::$variant => $id),+ }
            }
            pub fn from_id(raw: &str) -> Option<Self> {
                match raw { $($id => Some(<$ty>::$variant),)+ _ => None }
            }
        }

        impl Serialize for $ty {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.serialize_str(self.id())
            }
        }

        impl<'de> Deserialize<'de> for $ty {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                let raw = Cow::<'de, str>::deserialize(deserializer)?;
                Ok(<$ty>::from_id(&raw).unwrap_or_default())
            }
        }
    };
}

/// The clock faces, in the order the settings window lists them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Face {
    /// Translucent glass with a vertical sheen and a bright top edge.
    Glass,
    /// Thin strokes, wide tracking: the quiet one.
    Hairline,
    /// Heavy squared digits, tight tracking.
    Bold,
    /// Hours above minutes, right-aligned.
    Stacked,
    /// Narrow digits for small corners.
    Condensed,
    /// Mono, monospaced by construction.
    Mono,
}

named_enum!(Face {
    Glass => "glass",
    Hairline => "hairline",
    Bold => "bold",
    Stacked => "stacked",
    Condensed => "condensed",
    Mono => "mono",
});

impl Default for Face {
    fn default() -> Self {
        Self::Glass
    }
}

impl Face {
    pub const ALL: [Face; 6] =
        [Face::Glass, Face::Hairline, Face::Bold, Face::Stacked, Face::Condensed, Face::Mono];

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

    /// The next face, for cycling from the tray.
    pub fn next(self) -> Self {
        let index = Self::ALL.iter().position(|face| *face == self).unwrap_or(0);
        Self::ALL[(index + 1) % Self::ALL.len()]
    }

    /// One line for the settings card. Short enough to fit under the name at 11 pt.
    pub fn hint(self) -> &'static str {
        match self {
            Face::Glass => "Frosted, rim-lit",
            Face::Hairline => "Thin outlines",
            Face::Bold => "Heavy and tight",
            Face::Stacked => "Hours over minutes",
            Face::Condensed => "Narrow block",
            Face::Mono => "Monospaced",
        }
    }
}

/// How the fill is tinted. `Auto` follows the wallpaper luminance under the clock.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tint {
    Auto,
    Light,
    Warm,
    Cool,
}

named_enum!(Tint {
    Auto => "auto",
    Light => "light",
    Warm => "warm",
    Cool => "cool",
});

impl Default for Tint {
    fn default() -> Self {
        Self::Auto
    }
}

impl Tint {
    pub const ALL: [Tint; 4] = [Tint::Auto, Tint::Light, Tint::Warm, Tint::Cool];

    pub fn label(self) -> &'static str {
        match self {
            Tint::Auto => "Auto",
            Tint::Light => "Light",
            Tint::Warm => "Warm",
            Tint::Cool => "Cool",
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Settings {
    /// Physical desktop coordinates. `None` means "centre it on the primary display".
    #[serde(default)]
    pub x: Option<i32>,
    #[serde(default)]
    pub y: Option<i32>,
    #[serde(default = "one")]
    pub scale: f32,
    #[serde(default)]
    pub face: Face,
    #[serde(default = "yes")]
    pub hour24: bool,
    #[serde(default = "one")]
    pub opacity: f32,
    #[serde(default)]
    pub tint: Tint,
}

fn one() -> f32 {
    1.0
}

fn yes() -> bool {
    true
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            x: None,
            y: None,
            scale: 1.0,
            face: Face::default(),
            hour24: true,
            opacity: 1.0,
            tint: Tint::default(),
        }
    }
}

impl Settings {
    /// Settings are user-editable, so nothing in the file is trusted.
    pub fn sanitized(mut self) -> Self {
        if !self.scale.is_finite() {
            self.scale = 1.0;
        }
        self.scale = self.scale.clamp(SCALE_MIN, SCALE_MAX);
        if !self.opacity.is_finite() {
            self.opacity = 1.0;
        }
        self.opacity = self.opacity.clamp(OPACITY_MIN, OPACITY_MAX);
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
    fn opacity_is_clamped() {
        assert_eq!(Settings { opacity: 0.0, ..Default::default() }.sanitized().opacity, OPACITY_MIN);
        assert_eq!(Settings { opacity: 4.0, ..Default::default() }.sanitized().opacity, OPACITY_MAX);
        assert_eq!(
            Settings { opacity: f32::INFINITY, ..Default::default() }.sanitized().opacity,
            1.0
        );
    }

    #[test]
    fn a_half_position_is_dropped() {
        let s = Settings { x: Some(10), y: None, scale: 1.0, ..Default::default() }.sanitized();
        assert_eq!(s.x, None);
        assert_eq!(s.y, None);
    }

    #[test]
    fn missing_fields_fall_back() {
        let s: Settings = serde_json::from_str("{}").unwrap();
        assert_eq!(s.scale, 1.0);
        assert_eq!(s.x, None);
        assert_eq!(s.face, Face::Glass);
        assert!(s.hour24);
        assert_eq!(s.opacity, 1.0);
        assert_eq!(s.tint, Tint::Auto);
    }

    #[test]
    fn a_v1_settings_file_still_loads() {
        // Written by v1.0.0, before faces existed.
        let s: Settings =
            serde_json::from_str(r#"{"x":120,"y":340,"scale":1.4}"#).unwrap().sanitized();
        assert_eq!(s.x, Some(120));
        assert_eq!(s.scale, 1.4);
        assert_eq!(s.face, Face::Glass);
        assert_eq!(s.tint, Tint::Auto);
    }

    #[test]
    fn an_unknown_face_costs_one_setting_not_the_file() {
        let s: Settings =
            serde_json::from_str(r#"{"x":120,"y":340,"scale":1.4,"face":"neon"}"#).unwrap();
        assert_eq!(s.face, Face::Glass);
        assert_eq!(s.scale, 1.4, "the rest of the file must survive a typo");
        assert_eq!(s.x, Some(120));
    }

    #[test]
    fn enums_round_trip_through_json() {
        for face in Face::ALL {
            let text = serde_json::to_string(&face).unwrap();
            assert_eq!(serde_json::from_str::<Face>(&text).unwrap(), face);
        }
        for tint in Tint::ALL {
            let text = serde_json::to_string(&tint).unwrap();
            assert_eq!(serde_json::from_str::<Tint>(&text).unwrap(), tint);
        }
        assert_eq!(serde_json::to_string(&Face::Hairline).unwrap(), "\"hairline\"");
        assert_eq!(serde_json::to_string(&Tint::Cool).unwrap(), "\"cool\"");
    }

    #[test]
    fn cycling_visits_every_face_once() {
        let mut seen = Vec::new();
        let mut face = Face::Glass;
        for _ in 0..Face::ALL.len() {
            seen.push(face);
            face = face.next();
        }
        assert_eq!(face, Face::Glass);
        for expected in Face::ALL {
            assert!(seen.contains(&expected));
        }
    }
}
