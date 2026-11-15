//! High-precision glyph rasterization with metallic gradient and specular rim lighting.
//! Outputs a 32-bit BGRA buffer with pre-multiplied alpha for Windows UpdateLayeredWindow.

use fontdue::Font;
use crate::faces::{Fill, Ink, Style};

pub static FONT_BYTES: &[&[u8]] = &[
    include_bytes!("../assets/Oxanium-Bold.ttf"),
    include_bytes!("../assets/TitilliumWeb-Bold.ttf"),
    include_bytes!("../assets/JetBrainsMono-Bold.ttf"),
];

pub fn get_font(index: usize) -> &'static Font {
    static PARSED: std::sync::OnceLock<Vec<Option<Font>>> = std::sync::OnceLock::new();
    let parsed = PARSED.get_or_init(|| {
        FONT_BYTES
            .iter()
            .map(|bytes| Font::from_bytes(*bytes, fontdue::FontSettings::default()).ok())
            .collect()
    });
    parsed.get(index).and_then(|f| f.as_ref()).or_else(|| parsed.first()?.as_ref()).unwrap()
}

pub struct RasterBuffer {
    pub width: u32,
    pub height: u32,
    /// 32-bit BGRA with pre-multiplied alpha, direct memory format for Win32 DIB.
    pub bgra: Vec<u8>,
}

impl RasterBuffer {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            bgra: vec![0u8; (width * height * 4) as usize],
        }
    }

    pub fn blend_over(&mut self, src_x: i32, src_y: i32, overlay: &RasterBuffer) {
        for y in 0..overlay.height as i32 {
            let dst_y = src_y + y;
            if dst_y < 0 || dst_y >= self.height as i32 {
                continue;
            }
            for x in 0..overlay.width as i32 {
                let dst_x = src_x + x;
                if dst_x < 0 || dst_x >= self.width as i32 {
                    continue;
                }

                let src_idx = ((y as u32 * overlay.width + x as u32) * 4) as usize;
                let src_b = overlay.bgra[src_idx] as u32;
                let src_g = overlay.bgra[src_idx + 1] as u32;
                let src_r = overlay.bgra[src_idx + 2] as u32;
                let src_a = overlay.bgra[src_idx + 3] as u32;

                if src_a == 0 {
                    continue;
                }

                let dst_idx = ((dst_y as u32 * self.width + dst_x as u32) * 4) as usize;
                let dst_b = self.bgra[dst_idx] as u32;
                let dst_g = self.bgra[dst_idx + 1] as u32;
                let dst_r = self.bgra[dst_idx + 2] as u32;
                let dst_a = self.bgra[dst_idx + 3] as u32;

                // Pre-multiplied alpha "over" blending:
                // out_rgb = src_rgb + dst_rgb * (1 - src_a)
                // out_a   = src_a   + dst_a   * (1 - src_a)
                let inv_a = 255 - src_a;
                self.bgra[dst_idx]     = (src_b + (dst_b * inv_a) / 255).min(255) as u8;
                self.bgra[dst_idx + 1] = (src_g + (dst_g * inv_a) / 255).min(255) as u8;
                self.bgra[dst_idx + 2] = (src_r + (dst_r * inv_a) / 255).min(255) as u8;
                self.bgra[dst_idx + 3] = (src_a + (dst_a * inv_a) / 255).min(255) as u8;
            }
        }
    }
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

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

fn digit_cell_width(font: &Font, px: f32) -> f32 {
    ('0'..='9')
        .map(|d| font.metrics(d, px).advance_width)
        .fold(0.0f32, f32::max)
}

pub fn render_text(
    font: &Font,
    text: &str,
    px: f32,
    tracking: f32,
    digit_cells: bool,
    fill: Fill,
    ink: Ink,
) -> Option<RasterBuffer> {
    if text.is_empty() {
        return None;
    }

    let lines = font.horizontal_line_metrics(px)?;
    let ascent = lines.ascent;
    let descent = lines.descent;
    let cell_w = if digit_cells { digit_cell_width(font, px) } else { 0.0 };

    let mut pen = 0.0f32;
    let mut glyphs = Vec::with_capacity(text.chars().count());
    let count = text.chars().count();

    for (i, c) in text.chars().enumerate() {
        let advance = font.metrics(c, px).advance_width;
        let width = if digit_cells && c.is_ascii_digit() { cell_w } else { advance };
        glyphs.push((c, pen + (width - advance) * 0.5));
        pen += width;
        if i + 1 < count {
            pen += tracking;
        }
    }

    let width = pen.ceil().max(1.0) as u32;
    let height = (ascent - descent).ceil().max(1.0) as u32;
    let mut coverage = vec![0.0f32; (width * height) as usize];

    for (c, origin) in glyphs {
        let (metrics, bitmap) = font.rasterize(c, px);
        if metrics.width == 0 || metrics.height == 0 {
            continue;
        }
        let left = origin + metrics.xmin as f32;
        let top = ascent - metrics.ymin as f32 - metrics.height as f32;

        for r in 0..metrics.height {
            for col in 0..metrics.width {
                let val = bitmap[r * metrics.width + col] as f32 / 255.0;
                if val <= 0.0 {
                    continue;
                }
                let x = left.round() as i64 + col as i64;
                let y = top.round() as i64 + r as i64;
                if x < 0 || y < 0 || x >= width as i64 || y >= height as i64 {
                    continue;
                }
                let at = (y as u32 * width + x as u32) as usize;
                if val > coverage[at] {
                    coverage[at] = val;
                }
            }
        }
    }

    let coverage = match fill {
        Fill::Gradient => coverage,
        Fill::Outline { radius } => outline(&coverage, width, height, radius),
    };

    let mut bgra = vec![0u8; (width * height * 4) as usize];
    for y in 0..height {
        let t = if height > 1 { y as f32 / (height - 1) as f32 } else { 0.0 };
        let ramp = [
            lerp(ink.top[0] as f32, ink.bottom[0] as f32, t), // R
            lerp(ink.top[1] as f32, ink.bottom[1] as f32, t), // G
            lerp(ink.top[2] as f32, ink.bottom[2] as f32, t), // B
        ];

        for x in 0..width {
            let cov = coverage[(y * width + x) as usize];
            if cov <= 0.002 {
                continue;
            }

            let above = if y > 0 { coverage[((y - 1) * width + x) as usize] } else { 0.0 };
            let mut rgb = ramp;

            // Specular rim light on up-facing surfaces:
            if ink.rim > 0.0 && above < 0.35 {
                let lift = (ink.rim * cov).clamp(0.0, 1.0);
                for ch in 0..3 {
                    rgb[ch] += (255.0 - rgb[ch]) * lift;
                }
            }

            let alpha = (cov * ink.opacity).clamp(0.0, 1.0);
            let at = ((y * width + x) * 4) as usize;

            // Win32 Pre-multiplied BGRA:
            let r_pm = (rgb[0].clamp(0.0, 255.0) * alpha).round() as u8;
            let g_pm = (rgb[1].clamp(0.0, 255.0) * alpha).round() as u8;
            let b_pm = (rgb[2].clamp(0.0, 255.0) * alpha).round() as u8;
            let a_pm = (alpha * 255.0).round() as u8;

            bgra[at]     = b_pm; // Blue
            bgra[at + 1] = g_pm; // Green
            bgra[at + 2] = r_pm; // Red
            bgra[at + 3] = a_pm; // Alpha
        }
    }

    Some(RasterBuffer { width, height, bgra })
}

pub fn render_hairline(width: u32, ink: Ink) -> RasterBuffer {
    let mut bgra = vec![0u8; (width * 1 * 4) as usize];
    let a = (ink.opacity * 0.25).clamp(0.0, 1.0);
    let r_pm = (ink.top[0] as f32 * a).round() as u8;
    let g_pm = (ink.top[1] as f32 * a).round() as u8;
    let b_pm = (ink.top[2] as f32 * a).round() as u8;
    let a_pm = (a * 255.0).round() as u8;

    for x in 0..width as usize {
        let at = x * 4;
        bgra[at]     = b_pm;
        bgra[at + 1] = g_pm;
        bgra[at + 2] = r_pm;
        bgra[at + 3] = a_pm;
    }

    RasterBuffer { width, height: 1, bgra }
}

/// Renders the complete clock layout: Time Digits + Date Row.
pub fn render_clock_layout(
    time_str: &str,
    date_str: &str,
    style: &Style,
    scale: f32,
    show_date: bool,
) -> Option<RasterBuffer> {
    let font = get_font(style.font_index);
    let time_px = (style.size_pt * scale).max(12.0);
    let time_tracking = style.tracking * scale;

    let time_buf = render_text(
        font,
        time_str,
        time_px,
        time_tracking,
        style.digit_cells,
        style.fill,
        style.ink,
    )?;

    if !show_date || date_str.is_empty() {
        return Some(time_buf);
    }

    let date_px = (style.date_pt * scale).max(10.0);
    let date_tracking = style.date_tracking * scale;
    let date_ink = Ink {
        opacity: (style.ink.opacity * 0.65).min(0.75),
        ..style.ink
    };

    let date_buf = render_text(
        font,
        date_str,
        date_px,
        date_tracking,
        true,
        Fill::Gradient,
        date_ink,
    )?;

    let hairline_w = (32.0 * scale).round() as u32;
    let hairline_buf = render_hairline(hairline_w, date_ink);
    let gap_hairline = (10.0 * scale).round() as i32;
    let date_row_w = (hairline_w as i32 * 2 + gap_hairline * 2 + date_buf.width as i32) as u32;
    let date_gap = (style.date_gap * scale).round() as u32;

    let total_w = time_buf.width.max(date_row_w);
    let total_h = time_buf.height + date_gap + date_buf.height;

    let mut composite = RasterBuffer::new(total_w, total_h);

    // Center time digits at top:
    let time_x = (total_w as i32 - time_buf.width as i32) / 2;
    composite.blend_over(time_x, 0, &time_buf);

    // Center date row at bottom:
    let date_row_x = (total_w as i32 - date_row_w as i32) / 2;
    let date_y = (time_buf.height + date_gap) as i32;

    // Left hairline:
    composite.blend_over(date_row_x, date_y + (date_buf.height as i32 / 2), &hairline_buf);

    // Date text:
    composite.blend_over(date_row_x + hairline_w as i32 + gap_hairline, date_y, &date_buf);

    // Right hairline:
    composite.blend_over(
        date_row_x + hairline_w as i32 + gap_hairline + date_buf.width as i32 + gap_hairline,
        date_y + (date_buf.height as i32 / 2),
        &hairline_buf,
    );

    Some(composite)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::faces::style_for_face;
    use crate::settings::{Face, Tint};

    #[test]
    fn render_digits_produces_premultiplied_bgra() {
        let font = get_font(0);
        let style = style_for_face(Face::Glass, Tint::Chrome, true, 1.0);
        let buf = render_text(font, "10:44", 120.0, 2.0, true, Fill::Gradient, style.ink).unwrap();
        assert!(buf.width > 0 && buf.height > 0);
        assert_eq!(buf.bgra.len(), (buf.width * buf.height * 4) as usize);

        // Verify pre-multiplied invariant: R, G, B <= A
        for chunk in buf.bgra.chunks_exact(4) {
            let b = chunk[0];
            let g = chunk[1];
            let r = chunk[2];
            let a = chunk[3];
            assert!(r <= a, "Red {r} must be <= Alpha {a}");
            assert!(g <= a, "Green {g} must be <= Alpha {a}");
            assert!(b <= a, "Blue {b} must be <= Alpha {a}");
        }
    }
}
