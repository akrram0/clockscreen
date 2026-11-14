//! The face library: six treatments of the same measured layout.
//!
//! A face changes the font, the fill, the tracking, the layout and the ink — not the geometry
//! rules. Every face keeps tabular digit cells, so the time never shifts when a minute changes.
//!
//! Fonts, all SIL OFL, verified to have identical advance widths for `0`–`9`:
//! * 0 — Oxanium Bold (squared techno, the hero face)
//! * 1 — Titillium Web Bold (DIN-adjacent, narrower)
//! * 2 — JetBrains Mono Bold (monospaced)

use crate::glyphs::{Fill, Ink, Spec};
use crate::settings::{Face, Tint};

/// The base point size of the time, before the user's scale.
pub const TIME_PT: f32 = 160.0;
pub const DATE_PT: f32 = 18.0;

#[derive(Clone, Copy, Debug)]
pub struct Style {
    /// Index into the font table.
    pub font: usize,
    pub size_pt: f32,
    pub tracking: f32,
    pub digit_cells: bool,
    /// Hours above minutes instead of side by side.
    pub stacked: bool,
    pub fill: Fill,
    pub ink: Ink,
    /// The date line: smaller, wider tracking, dimmer ink.
    pub date_px: f32,
    pub date_tracking: f32,
    /// Space between the time and the date line, in points at scale 1.
    pub date_gap: f32,
}

/// The ink for a tint, resolved against the desktop.
///
/// `Auto` follows the system theme: a light theme usually means a light wallpaper, so the clock
/// turns to graphite glass rather than white on white.
pub fn ink(tint: Tint, system_dark: bool, opacity: f32) -> Ink {
    let base = match tint {
        Tint::Light => Ink::LIGHT,
        Tint::Warm => Ink::WARM,
        Tint::Cool => Ink::COOL,
        Tint::Auto => {
            if system_dark {
                Ink::LIGHT
            } else {
                Ink::GRAPHITE
            }
        }
    };
    base.with_opacity(opacity)
}

pub fn style(face: Face, tint: Tint, system_dark: bool, opacity: f32) -> Style {
    let ink = ink(tint, system_dark, opacity);
    let base = Style {
        font: 0,
        size_pt: TIME_PT,
        tracking: 2.0,
        digit_cells: true,
        stacked: false,
        fill: Fill::Gradient,
        ink,
        date_px: DATE_PT,
        date_tracking: 5.0,
        date_gap: 12.0,
    };
    match face {
        // Frosted glass: the vertical ramp plus a rim light is the whole idea.
        Face::Glass => base,
        // Hairline: only the rim of each stroke survives, so the digits read as thin outlines.
        Face::Hairline => Style {
            tracking: 4.0,
            fill: Fill::Outline { radius: 2 },
            ink: Ink { rim: 0.15, ..ink },
            date_tracking: 6.0,
            ..base
        },
        // Bold: heavier presence through tighter tracking and a harder fill.
        Face::Bold => Style {
            size_pt: TIME_PT + 6.0,
            tracking: -1.0,
            ink: Ink { rim: 0.22, opacity: (ink.opacity * 1.06).min(1.0), ..ink },
            date_tracking: 4.0,
            ..base
        },
        // Stacked: hours over minutes, for tall and narrow wallpaper areas.
        Face::Stacked => Style {
            size_pt: TIME_PT * 0.86,
            stacked: true,
            tracking: 2.0,
            date_gap: 16.0,
            ..base
        },
        // Condensed: Titillium is narrower, and the tracking pulls it tighter still.
        Face::Condensed => Style {
            font: 1,
            tracking: -3.0,
            ink: Ink { rim: 0.35, ..ink },
            date_tracking: 5.0,
            ..base
        },
        // Mono: JetBrains Mono, which is monospaced by construction.
        Face::Mono => Style {
            font: 2,
            size_pt: TIME_PT * 0.88,
            tracking: -1.5,
            ink: Ink { rim: 0.30, ..ink },
            date_px: DATE_PT * 0.95,
            date_tracking: 4.0,
            ..base
        },
    }
}

/// The spec for one rasterised run.
pub fn spec(style: &Style, dpi: f32, px: f32) -> Spec {
    Spec {
        px,
        dpi,
        tracking: style.tracking,
        digit_cells: style.digit_cells,
        fill: style.fill,
        ink: style.ink,
    }
}

/// The spec for the date line, at 60% of the time's alpha.
pub fn date_spec(style: &Style, dpi: f32, scale: f32) -> Spec {
    Spec {
        px: style.date_px * scale,
        dpi,
        tracking: style.date_tracking * scale,
        digit_cells: true,
        fill: Fill::Gradient,
        ink: style.ink.with_opacity((style.ink.opacity * 0.62).min(0.72)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_flips_to_graphite_on_a_light_desktop() {
        let dark = ink(Tint::Auto, true, 1.0);
        let light = ink(Tint::Auto, false, 1.0);
        assert_eq!(dark.top, Ink::LIGHT.top);
        assert_eq!(light.top, Ink::GRAPHITE.top);
        assert!(light.top[0] < dark.top[0], "graphite must be darker than white");
    }

    #[test]
    fn every_face_keeps_tabular_cells_and_a_usable_size() {
        for face in Face::ALL {
            let style = style(face, Tint::Light, true, 1.0);
            assert!(style.digit_cells, "{face:?} must not let the time shift");
            assert!(style.size_pt > 100.0, "{face:?} is too small to be a clock face");
            assert!(style.ink.opacity > 0.0);
            assert!(style.font < crate::glyphs::FONT_BYTES.len(), "{face:?} names a missing font");
        }
    }

    #[test]
    fn opacity_reaches_the_ink() {
        let faint = style(Face::Glass, Tint::Light, true, 0.4);
        assert!((faint.ink.opacity - 0.4).abs() < 1e-6);
    }

    #[test]
    fn hairline_is_an_outline_and_glass_is_a_fill() {
        let glass = style(Face::Glass, Tint::Light, true, 1.0);
        let hairline = style(Face::Hairline, Tint::Light, true, 1.0);
        assert_eq!(glass.fill, Fill::Gradient);
        assert!(matches!(hairline.fill, Fill::Outline { .. }));
    }

    #[test]
    fn the_date_line_is_dimmer_than_the_time() {
        let style = style(Face::Glass, Tint::Light, true, 1.0);
        let date = date_spec(&style, 1.0, 1.0);
        assert!(date.ink.opacity < style.ink.opacity);
    }
}
