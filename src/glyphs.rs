//! Rasterises the time into an RGBA buffer.
//!
//! Rune paints text with a single solid colour and cannot clip a rectangle, so a gradient inside
//! the glyphs is impossible with `text()`. Rasterising here instead buys the things the design
//! actually asks for: a vertical white→grey ramp, a bright rim on the up-facing edges, and equal
//! advance widths for the digits so `HH:MM` never shifts as the minutes change.

use fontdue::Font;
use rune::Pixels;

/// How the ink is coloured. Straight alpha, sRGB.
#[derive(Clone, Copy, Debug)]
pub struct Ink {
    /// Colour at the top of the line box.
    pub top: [u8; 3],
    /// Colour at the bottom.
    pub bottom: [u8; 3],
    /// Multiplied into every alpha.
    pub opacity: f32,
    /// 0..1: how much the up-facing edges are pushed towards white.
    pub rim: f32,
}

impl Ink {
    pub const LIGHT: Ink =
        Ink { top: [255, 255, 255], bottom: [166, 174, 186], opacity: 0.90, rim: 0.55 };
    pub const WARM: Ink =
        Ink { top: [255, 249, 240], bottom: [206, 186, 160], opacity: 0.90, rim: 0.50 };
    pub const COOL: Ink =
        Ink { top: [244, 250, 255], bottom: [150, 168, 190], opacity: 0.90, rim: 0.60 };
    /// For light wallpapers: graphite glass, so the clock reads instead of disappearing.
    pub const GRAPHITE: Ink =
        Ink { top: [64, 70, 80], bottom: [18, 21, 26], opacity: 0.86, rim: 0.30 };

    pub fn with_opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity.clamp(0.0, 1.0);
        self
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Spec {
    /// Point size, before the display scale is applied.
    pub px: f32,
    /// Physical pixels per point. `cx.env().scale`.
    pub dpi: f32,
    /// Extra points between glyphs.
    pub tracking: f32,
    /// Give every digit the width of the widest one, so the time cannot shift.
    pub digit_cells: bool,
    pub fill: Fill,
    pub ink: Ink,
}

/// What the coverage becomes before it is coloured.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fill {
    /// Solid glyphs carrying the vertical ramp.
    Gradient,
    /// Only the rim of each stroke, `radius` device pixels thick: a hairline face.
    Outline { radius: u32 },
}

/// Thickness of a stroke edge: ink where a neighbourhood of `radius` is empty.
fn outline(coverage: &[f32], width: u32, height: u32, radius: u32) -> Vec<f32> {
    let mut edge = vec![0.0f32; coverage.len()];
    let radius = radius.max(1) as i64;
    for y in 0..height as i64 {
        for x in 0..width as i64 {
            let centre = coverage[(y * width as i64 + x) as usize];
            if centre <= 0.002 {
                continue;
            }
            let mut lowest = 1.0f32;
            for dy in -radius..=radius {
                for dx in -radius..=radius {
                    if dx * dx + dy * dy > radius * radius {
                        continue;
                    }
                    let nx = x + dx;
                    let ny = y + dy;
                    let value = if nx < 0 || ny < 0 || nx >= width as i64 || ny >= height as i64 {
                        0.0
                    } else {
                        coverage[(ny * width as i64 + nx) as usize]
                    };
                    lowest = lowest.min(value);
                }
            }
            edge[(y * width as i64 + x) as usize] = (centre - lowest).max(0.0);
        }
    }
    edge
}

/// A rasterised run, ready to hand to `El::pixels`.
pub struct Run {
    pub points: (f32, f32),
    pub pixels: Pixels,
}

/// The raw raster, kept separate from [`Run`] so it can be asserted on without a GPU.
pub struct Mask {
    pub width: u32,
    pub height: u32,
    /// `width * height * 4` straight-alpha RGBA.
    pub rgba: Vec<u8>,
    /// Size in points, i.e. physical size divided by the display scale.
    pub points: (f32, f32),
}

impl Mask {
    pub fn alpha_at(&self, x: u32, y: u32) -> u8 {
        let at = ((y * self.width + x) * 4 + 3) as usize;
        self.rgba.get(at).copied().unwrap_or(0)
    }

    /// Brightest channel in a row, for checking that the fill actually ramps.
    pub fn brightest_in_row(&self, y: u32) -> u8 {
        (0..self.width)
            .map(|x| {
                let at = ((y * self.width + x) * 4) as usize;
                self.rgba[at..at + 3].iter().copied().max().unwrap_or(0)
            })
            .max()
            .unwrap_or(0)
    }

    pub fn ink_pixels(&self) -> usize {
        self.rgba.chunks_exact(4).filter(|pixel| pixel[3] > 0).count()
    }
}

/// Width of the widest digit, so `1` and `0` occupy the same cell.
fn digit_cell(font: &Font, px: f32) -> f32 {
    ('0'..='9').map(|digit| font.metrics(digit, px).advance_width).fold(0.0f32, f32::max)
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

pub fn mask(font: &Font, text: &str, spec: &Spec) -> Option<Mask> {
    if text.is_empty() {
        return None;
    }
    let dpi = if spec.dpi.is_finite() && spec.dpi > 0.05 { spec.dpi } else { 1.0 };
    let px = (spec.px * dpi).max(1.0);
    let lines = font.horizontal_line_metrics(px)?;
    let ascent = lines.ascent;
    let descent = lines.descent;
    let tracking = spec.tracking * dpi;
    let cell = if spec.digit_cells { digit_cell(font, px) } else { 0.0 };

    // Lay the run out on a baseline at `ascent`, one advance at a time.
    let mut pen = 0.0f32;
    let mut glyphs: Vec<(char, f32)> = Vec::with_capacity(text.chars().count());
    let count = text.chars().count();
    for (index, character) in text.chars().enumerate() {
        let advance = font.metrics(character, px).advance_width;
        let width = if spec.digit_cells && character.is_ascii_digit() { cell } else { advance };
        // Digits are centred in their cell; everything else sits on its own advance.
        glyphs.push((character, pen + (width - advance) * 0.5));
        pen += width;
        if index + 1 < count {
            pen += tracking;
        }
    }

    let width = pen.ceil().max(1.0) as u32;
    let height = (ascent - descent).ceil().max(1.0) as u32;
    let mut coverage = vec![0.0f32; (width * height) as usize];

    for (character, origin) in glyphs {
        let (metrics, bitmap) = font.rasterize(character, px);
        if metrics.width == 0 || metrics.height == 0 {
            continue;
        }
        let left = origin + metrics.xmin as f32;
        let top = ascent - metrics.ymin as f32 - metrics.height as f32;
        for row in 0..metrics.height {
            for column in 0..metrics.width {
                let value = bitmap[row * metrics.width + column] as f32 / 255.0;
                if value <= 0.0 {
                    continue;
                }
                let x = left.round() as i64 + column as i64;
                let y = top.round() as i64 + row as i64;
                if x < 0 || y < 0 || x >= width as i64 || y >= height as i64 {
                    continue;
                }
                let at = (y as u32 * width + x as u32) as usize;
                // `max`, not `+`: overlapping anti-aliased edges must not darken.
                if value > coverage[at] {
                    coverage[at] = value;
                }
            }
        }
    }

    let coverage = match spec.fill {
        Fill::Gradient => coverage,
        Fill::Outline { radius } => outline(&coverage, width, height, radius),
    };

    let mut rgba = vec![0u8; (width * height * 4) as usize];
    for y in 0..height {
        let t = if height > 1 { y as f32 / (height - 1) as f32 } else { 0.0 };
        let ramp = [
            lerp(spec.ink.top[0] as f32, spec.ink.bottom[0] as f32, t),
            lerp(spec.ink.top[1] as f32, spec.ink.bottom[1] as f32, t),
            lerp(spec.ink.top[2] as f32, spec.ink.bottom[2] as f32, t),
        ];
        for x in 0..width {
            let alpha = coverage[(y * width + x) as usize];
            if alpha <= 0.002 {
                continue;
            }
            let above = if y > 0 { coverage[((y - 1) * width + x) as usize] } else { 0.0 };
            // An up-facing edge: ink here, wallpaper above it. That is the rim light.
            let mut colour = ramp;
            if spec.ink.rim > 0.0 && above < 0.35 {
                let lift = (spec.ink.rim * alpha).clamp(0.0, 1.0);
                for channel in 0..3 {
                    colour[channel] += (255.0 - colour[channel]) * lift;
                }
            }
            let at = ((y * width + x) * 4) as usize;
            rgba[at] = colour[0].clamp(0.0, 255.0).round() as u8;
            rgba[at + 1] = colour[1].clamp(0.0, 255.0).round() as u8;
            rgba[at + 2] = colour[2].clamp(0.0, 255.0).round() as u8;
            rgba[at + 3] = (alpha * spec.ink.opacity).clamp(0.0, 1.0).mul_add(255.0, 0.0).round() as u8;
        }
    }

    Some(Mask {
        width,
        height,
        rgba,
        points: (width as f32 / dpi, height as f32 / dpi),
    })
}

/// Rasterise and upload. `None` when the font has no metrics for this text.
pub fn run(font: &Font, text: &str, spec: &Spec) -> Option<Run> {
    let mask = mask(font, text, spec)?;
    let pixels = Pixels::new();
    pixels.set(mask.width, mask.height, mask.rgba.clone());
    Some(Run { points: mask.points, pixels })
}

/// The font table, in the order faces refer to it. Also handed to `AppSpec::fonts`.
/// Every entry is SIL OFL and has identical advance widths for `0`–`9`.
pub static FONT_BYTES: &[&[u8]] = &[
    include_bytes!("../assets/Oxanium-Bold.ttf"),
    include_bytes!("../assets/TitilliumWeb-Bold.ttf"),
    include_bytes!("../assets/JetBrainsMono-Bold.ttf"),
];

/// Parse the table once. An index that does not exist falls back to the first font, so a face can
/// name a font that is not bundled without taking the clock down.
pub fn font(index: usize) -> Option<&'static Font> {
    static PARSED: std::sync::OnceLock<Vec<Option<Font>>> = std::sync::OnceLock::new();
    let parsed = PARSED.get_or_init(|| {
        FONT_BYTES
            .iter()
            .map(|bytes| Font::from_bytes(*bytes, fontdue::FontSettings::default()).ok())
            .collect()
    });
    parsed.get(index).and_then(|font| font.as_ref()).or_else(|| parsed.first()?.as_ref())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn font() -> Font {
        Font::from_bytes(
            include_bytes!("../assets/Oxanium-Bold.ttf").as_slice(),
            fontdue::FontSettings::default(),
        )
        .expect("the bundled font must parse")
    }

    fn spec() -> Spec {
        Spec {
            px: 120.0,
            dpi: 1.0,
            tracking: 0.0,
            digit_cells: true,
            fill: Fill::Gradient,
            ink: Ink::LIGHT,
        }
    }

    #[test]
    fn a_digit_has_ink() {
        let mask = mask(&font(), "0", &spec()).expect("one digit must rasterise");
        assert!(mask.ink_pixels() > 0, "a rendered digit must cover pixels");
        assert!(mask.width > 0 && mask.height > 0);
    }

    #[test]
    fn tabular_cells_give_every_digit_the_same_width() {
        let font = font();
        let one = mask(&font, "1", &spec()).expect("1");
        let zero = mask(&font, "0", &spec()).expect("0");
        assert_eq!(one.width, zero.width, "1 and 0 must occupy the same cell");

        // ...and the whole time is the same width whatever the digits are.
        let a = mask(&font, "11:11", &spec()).expect("11:11");
        let b = mask(&font, "08:59", &spec()).expect("08:59");
        assert_eq!(a.width, b.width);
    }

    fn ink_row(mask: &Mask, y: u32) -> bool {
        (0..mask.width).any(|x| mask.alpha_at(x, y) > 0)
    }

    #[test]
    fn the_fill_ramps_from_top_to_bottom() {
        let mask = mask(&font(), "8", &spec()).expect("8");
        let first_ink = (0..mask.height).find(|y| ink_row(&mask, *y));
        let last_ink = (0..mask.height).rev().find(|y| ink_row(&mask, *y));
        let (Some(first), Some(last)) = (first_ink, last_ink) else {
            panic!("the digit must have ink rows");
        };
        assert!(
            mask.brightest_in_row(first) > mask.brightest_in_row(last),
            "the top of the glyph must be brighter than the bottom"
        );
    }

    #[test]
    fn opacity_scales_alpha() {
        let mut faint = spec();
        faint.ink.opacity = 0.5;
        let solid = mask(&font(), "8", &spec()).expect("8");
        let faded = mask(&font(), "8", &faint).expect("8");
        let peak = |m: &Mask| m.rgba.chunks_exact(4).map(|p| p[3]).max().unwrap_or(0);
        assert!(peak(&faded) < peak(&solid));
        assert!(peak(&faded) > 0);
    }

    #[test]
    fn the_scale_factor_buys_resolution_not_size() {
        let mut dense = spec();
        dense.dpi = 2.0;
        let single = mask(&font(), "12", &spec()).expect("12");
        let double = mask(&font(), "12", &dense).expect("12");
        assert!(double.width > single.width, "a 2x display needs a 2x buffer");
        assert!(
            (double.points.0 - single.points.0).abs() < 1.5,
            "but the same size in points, got {:?} vs {:?}",
            double.points,
            single.points
        );
    }

    #[test]
    fn an_outline_keeps_the_rim_and_drops_the_core() {
        let mut hollow = spec();
        hollow.fill = Fill::Outline { radius: 3 };
        let solid = mask(&font(), "8", &spec()).expect("8");
        let edge = mask(&font(), "8", &hollow).expect("8");
        assert!(edge.ink_pixels() > 0, "an outline must still have ink");
        assert!(
            edge.ink_pixels() * 2 < solid.ink_pixels(),
            "an outline should be mostly empty: {} vs {}",
            edge.ink_pixels(),
            solid.ink_pixels()
        );
    }

    #[test]
    fn empty_text_is_none() {
        assert!(mask(&font(), "", &spec()).is_none());
    }
}
