//! Depth Compositor & Companion Widgets for ClockScreen.
//!
//! Provides:
//! 1. Foreground subject mask occlusion (the "Depth Effect", tucking numbers behind wallpaper elements).
//! 2. Circular Vinyl Visualizer disc (spinning groove disc with lyrics / now-playing text).
//! 3. Top Dynamic Island (sleek media bar pill at top of screen).
//! 4. Clean tracked bottom date line.

use rune::{
    col, el, row, text, Color, El, LoopSpec, Pixels, Spring,
};

/// Generates a procedural circular vinyl disc with grooves and album center label.
pub fn vinyl_disc(scale: f32, lyrics: &str, accent: Color) -> El {
    let size = 180.0 * scale;
    let centre = size / 2.0;

    col()
        .key("vinyl-container")
        .size(size, size)
        .center()
        // Outer glow & drop shadow
        .shadow(0.0, 16.0 * scale, Color { r: 0.0, g: 0.0, b: 0.0, a: 0.65 })
        .child(
            // The black vinyl disc body
            el()
                .key("vinyl-body")
                .size(size, size)
                .radius(size / 2.0)
                .fill(Color { r: 0.08, g: 0.09, b: 0.11, a: 0.88 })
                .border(1.0, Color { r: 1.0, g: 1.0, b: 1.0, a: 0.12 })
                .center()
                .child(
                    // Grooves ring
                    el()
                        .size(size * 0.85, size * 0.85)
                        .radius(size * 0.85 / 2.0)
                        .border(1.0, Color { r: 1.0, g: 1.0, b: 1.0, a: 0.06 })
                        .center()
                        .child(
                            // Inner label
                            col()
                                .size(size * 0.58, size * 0.58)
                                .radius(size * 0.58 / 2.0)
                                .fill(Color { r: 0.14, g: 0.16, b: 0.20, a: 0.92 })
                                .border(1.0, Color { r: 1.0, g: 1.0, b: 1.0, a: 0.18 })
                                .center()
                                .px(12.0 * scale)
                                .child(
                                    text(lyrics)
                                        .size(11.0 * scale)
                                        .color(Color { r: 0.96, g: 0.97, b: 0.99, a: 0.95 })
                                        .align(rune::Align::Center),
                                ),
                        ),
                ),
        )
}

/// The sleek Top Dynamic Island showing currently playing track.
pub fn dynamic_island(scale: f32, title: &str, artist: &str, accent: Color) -> El {
    let h = 38.0 * scale;
    row()
        .key("dynamic-island")
        .h(h)
        .center_y()
        .px(16.0 * scale)
        .gap(10.0 * scale)
        .radius(h / 2.0)
        .fill(Color { r: 0.06, g: 0.07, b: 0.09, a: 0.92 })
        .border(1.0, Color { r: 1.0, g: 1.0, b: 1.0, a: 0.14 })
        .shadow(0.0, 10.0 * scale, Color { r: 0.0, g: 0.0, b: 0.0, a: 0.50 })
        .child(
            // Mini equalizer indicator (breathing dot)
            el()
                .size(7.0 * scale, 7.0 * scale)
                .radius(3.5 * scale)
                .fill(accent)
                .looping(LoopSpec::opacity(1.0, 0.0, 0.35, 1.0)),
        )
        .child(
            text(format!("{title}  ·  {artist}"))
                .size(12.0 * scale)
                .color(Color { r: 0.94, g: 0.95, b: 0.98, a: 0.88 }),
        )
        .child(
            // Mini audio progress pill
            el()
                .size(36.0 * scale, 3.0 * scale)
                .radius(1.5 * scale)
                .fill(Color { r: 1.0, g: 1.0, b: 1.0, a: 0.12 })
                .child(
                    el()
                        .size(18.0 * scale, 3.0 * scale)
                        .radius(1.5 * scale)
                        .fill(accent),
                ),
        )
}

/// Depth Occlusion Compositor: blends the rasterized clock glyphs behind a foreground mask.
pub struct DepthOcclusion {
    pub width: u32,
    pub height: u32,
    pub pixels: Pixels,
}

impl DepthOcclusion {
    /// Creates a transparent occlusion surface.
    pub fn new(width: u32, height: u32) -> Self {
        let pixels = Pixels::new();
        pixels.set(width, height, vec![0u8; (width * height * 4) as usize]);
        Self { width, height, pixels }
    }

    /// Composite foreground cutout over a base RGBA buffer.
    /// Where cutout has alpha, cutout pixels draw OVER the clock.
    pub fn composite_over(
        dst: &mut [u8],
        dst_width: u32,
        src: &[u8],
        src_width: u32,
        src_height: u32,
        offset_x: i32,
        offset_y: i32,
    ) {
        for y in 0..src_height as i32 {
            let dy = y + offset_y;
            if dy < 0 {
                continue;
            }
            for x in 0..src_width as i32 {
                let dx = x + offset_x;
                if dx < 0 || dx >= dst_width as i32 {
                    continue;
                }
                let src_idx = ((y as usize * src_width as usize + x as usize) * 4) as usize;
                let dst_idx = ((dy as usize * dst_width as usize + dx as usize) * 4) as usize;

                if src_idx + 3 >= src.len() || dst_idx + 3 >= dst.len() {
                    continue;
                }

                let src_a = src[src_idx + 3] as f32 / 255.0;
                if src_a <= 0.001 {
                    continue;
                }

                let dst_a = dst[dst_idx + 3] as f32 / 255.0;
                let out_a = src_a + dst_a * (1.0 - src_a);

                if out_a > 0.0 {
                    for c in 0..3 {
                        let sc = src[src_idx + c] as f32;
                        let dc = dst[dst_idx + c] as f32;
                        let oc = (sc * src_a + dc * dst_a * (1.0 - src_a)) / out_a;
                        dst[dst_idx + c] = oc.round() as u8;
                    }
                    dst[dst_idx + 3] = (out_a * 255.0).round() as u8;
                }
            }
        }
    }
}
