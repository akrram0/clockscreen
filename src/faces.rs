//! Clock face typography and aesthetic definitions.

use crate::settings::{Face, Tint};

pub const BASE_TIME_PT: f32 = 150.0;
pub const BASE_DATE_PT: f32 = 18.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ink {
    pub top: [u8; 3],
    pub bottom: [u8; 3],
    pub opacity: f32,
    pub rim: f32,
}

impl Ink {
    pub const CHROME: Ink = Ink {
        top: [255, 255, 255],
        bottom: [165, 175, 190],
        opacity: 0.95,
        rim: 0.60,
    };
    pub const WARM: Ink = Ink {
        top: [255, 245, 230],
        bottom: [210, 185, 145],
        opacity: 0.95,
        rim: 0.50,
    };
    pub const COOL: Ink = Ink {
        top: [240, 250, 255],
        bottom: [140, 175, 215],
        opacity: 0.95,
        rim: 0.65,
    };
    pub const CYBER: Ink = Ink {
        top: [220, 255, 255],
        bottom: [35, 195, 235],
        opacity: 0.98,
        rim: 0.85,
    };
    pub const GRAPHITE: Ink = Ink {
        top: [70, 75, 88],
        bottom: [22, 26, 32],
        opacity: 0.90,
        rim: 0.30,
    };

    pub fn with_opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity.clamp(0.0, 1.0);
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Fill {
    Gradient,
    Outline { radius: u32 },
}

#[derive(Clone, Copy, Debug)]
pub struct Style {
    pub font_index: usize,
    pub size_pt: f32,
    pub tracking: f32,
    pub digit_cells: bool,
    pub stacked: bool,
    pub fill: Fill,
    pub ink: Ink,
    pub date_pt: f32,
    pub date_tracking: f32,
    pub date_gap: f32,
}

pub fn ink_for_tint(tint: Tint, system_dark: bool, opacity: f32) -> Ink {
    let base = match tint {
        Tint::Auto => {
            if system_dark {
                Ink::CHROME
            } else {
                Ink::GRAPHITE
            }
        }
        Tint::Chrome => Ink::CHROME,
        Tint::Warm => Ink::WARM,
        Tint::Cool => Ink::COOL,
        Tint::Cyber => Ink::CYBER,
    };
    base.with_opacity(opacity)
}

pub fn style_for_face(face: Face, tint: Tint, system_dark: bool, opacity: f32) -> Style {
    let ink = ink_for_tint(tint, system_dark, opacity);
    let base = Style {
        font_index: 0, // Oxanium-Bold
        size_pt: BASE_TIME_PT,
        tracking: 2.0,
        digit_cells: true,
        stacked: false,
        fill: Fill::Gradient,
        ink,
        date_pt: BASE_DATE_PT,
        date_tracking: 5.0,
        date_gap: 12.0,
    };

    match face {
        Face::Glass => base,
        Face::Hairline => Style {
            tracking: 4.0,
            fill: Fill::Outline { radius: 2 },
            ink: Ink { rim: 0.20, ..ink },
            date_tracking: 6.0,
            ..base
        },
        Face::Bold => Style {
            size_pt: BASE_TIME_PT + 6.0,
            tracking: -1.0,
            ink: Ink { rim: 0.30, opacity: (ink.opacity * 1.05).min(1.0), ..ink },
            date_tracking: 4.0,
            ..base
        },
        Face::Stacked => Style {
            size_pt: BASE_TIME_PT * 0.86,
            stacked: true,
            tracking: 2.0,
            date_gap: 14.0,
            ..base
        },
        Face::Condensed => Style {
            font_index: 1, // TitilliumWeb-Bold
            tracking: -3.0,
            ink: Ink { rim: 0.40, ..ink },
            date_tracking: 5.0,
            ..base
        },
        Face::Mono => Style {
            font_index: 2, // JetBrainsMono-Bold
            size_pt: BASE_TIME_PT * 0.88,
            tracking: -1.5,
            ink: Ink { rim: 0.35, ..ink },
            date_pt: BASE_DATE_PT * 0.95,
            date_tracking: 4.0,
            ..base
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tint_resolution() {
        let dark = ink_for_tint(Tint::Auto, true, 1.0);
        let light = ink_for_tint(Tint::Auto, false, 1.0);
        assert_eq!(dark.top, Ink::CHROME.top);
        assert_eq!(light.top, Ink::GRAPHITE.top);
    }

    #[test]
    fn all_faces_have_valid_sizes() {
        for face in Face::ALL {
            let s = style_for_face(face, Tint::Chrome, true, 1.0);
            assert!(s.size_pt >= 100.0);
            assert!(s.font_index <= 2);
        }
    }
}
