//! Depth Compositor: Blends foreground subject cutout masks over clock numbers
//! to achieve true 3D wallpaper depth occlusion (iOS 16 / r/unixporn lockscreen style).

use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use crate::glyphs::RasterBuffer;

pub struct DepthMask {
    pub width: u32,
    pub height: u32,
    /// 32-bit pre-multiplied BGRA
    pub bgra: Vec<u8>,
}

impl DepthMask {
    pub fn load_from_png<P: AsRef<Path>>(path: P) -> Option<Self> {
        let file = File::open(path).ok()?;
        let reader = BufReader::new(file);
        let decoder = png::Decoder::new(reader);
        let mut reader = decoder.read_info().ok()?;
        let buf_len = reader.output_buffer_size()?;
        let mut buf = vec![0u8; buf_len];
        let info = reader.next_frame(&mut buf).ok()?;

        let width = info.width;
        let height = info.height;
        let mut bgra = vec![0u8; (width * height * 4) as usize];

        match info.color_type {
            png::ColorType::Rgba => {
                for y in 0..height {
                    for x in 0..width {
                        let src_idx = ((y * width + x) * 4) as usize;
                        let r = buf[src_idx] as u32;
                        let g = buf[src_idx + 1] as u32;
                        let b = buf[src_idx + 2] as u32;
                        let a = buf[src_idx + 3] as u32;

                        let dst_idx = src_idx;
                        bgra[dst_idx]     = ((b * a) / 255) as u8;
                        bgra[dst_idx + 1] = ((g * a) / 255) as u8;
                        bgra[dst_idx + 2] = ((r * a) / 255) as u8;
                        bgra[dst_idx + 3] = a as u8;
                    }
                }
            }
            png::ColorType::Rgb => {
                for y in 0..height {
                    for x in 0..width {
                        let src_idx = ((y * width + x) * 3) as usize;
                        let r = buf[src_idx];
                        let g = buf[src_idx + 1];
                        let b = buf[src_idx + 2];

                        let dst_idx = ((y * width + x) * 4) as usize;
                        bgra[dst_idx]     = b;
                        bgra[dst_idx + 1] = g;
                        bgra[dst_idx + 2] = r;
                        bgra[dst_idx + 3] = 255;
                    }
                }
            }
            _ => return None,
        }

        Some(DepthMask { width, height, bgra })
    }
}

/// Applies the depth mask over the clock raster buffer.
/// Where the mask has opacity, it renders on top of the clock numbers, tucking the digits behind.
pub fn apply_depth_occlusion(clock_buf: &mut RasterBuffer, mask: &DepthMask, offset_x: i32, offset_y: i32) {
    for y in 0..clock_buf.height as i32 {
        let mask_y = y - offset_y;
        if mask_y < 0 || mask_y >= mask.height as i32 {
            continue;
        }
        for x in 0..clock_buf.width as i32 {
            let mask_x = x - offset_x;
            if mask_x < 0 || mask_x >= mask.width as i32 {
                continue;
            }

            let mask_idx = ((mask_y as u32 * mask.width + mask_x as u32) * 4) as usize;
            let m_b = mask.bgra[mask_idx] as u32;
            let m_g = mask.bgra[mask_idx + 1] as u32;
            let m_r = mask.bgra[mask_idx + 2] as u32;
            let m_a = mask.bgra[mask_idx + 3] as u32;

            if m_a == 0 {
                continue;
            }

            let clk_idx = ((y as u32 * clock_buf.width + x as u32) * 4) as usize;
            let c_b = clock_buf.bgra[clk_idx] as u32;
            let c_g = clock_buf.bgra[clk_idx + 1] as u32;
            let c_r = clock_buf.bgra[clk_idx + 2] as u32;
            let c_a = clock_buf.bgra[clk_idx + 3] as u32;

            // Pre-multiplied source-over:
            let inv_m = 255 - m_a;
            clock_buf.bgra[clk_idx]     = (m_b + (c_b * inv_m) / 255).min(255) as u8;
            clock_buf.bgra[clk_idx + 1] = (m_g + (c_g * inv_m) / 255).min(255) as u8;
            clock_buf.bgra[clk_idx + 2] = (m_r + (c_r * inv_m) / 255).min(255) as u8;
            clock_buf.bgra[clk_idx + 3] = (m_a + (c_a * inv_m) / 255).min(255) as u8;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn depth_occlusion_blends_over() {
        let mut clock = RasterBuffer::new(10, 10);
        // Fill clock with white (255, 255, 255, 255):
        for pixel in clock.bgra.chunks_exact_mut(4) {
            pixel[0] = 255;
            pixel[1] = 255;
            pixel[2] = 255;
            pixel[3] = 255;
        }

        // Mask with half-width red opaque block (B: 0, G: 0, R: 255, A: 255):
        let mut mask_bgra = vec![0u8; 10 * 10 * 4];
        for y in 0..10 {
            for x in 0..5 {
                let idx = ((y * 10 + x) * 4) as usize;
                mask_bgra[idx] = 0;     // Blue
                mask_bgra[idx + 1] = 0; // Green
                mask_bgra[idx + 2] = 255;// Red
                mask_bgra[idx + 3] = 255;// Alpha
            }
        }
        let mask = DepthMask { width: 10, height: 10, bgra: mask_bgra };

        apply_depth_occlusion(&mut clock, &mask, 0, 0);

        // x < 5 should now be Red (the mask):
        let pixel_0 = &clock.bgra[0..4];
        assert_eq!(pixel_0[0], 0);   // Blue
        assert_eq!(pixel_0[1], 0);   // Green
        assert_eq!(pixel_0[2], 255); // Red

        // x >= 5 should remain White (the clock):
        let pixel_6 = &clock.bgra[(6 * 4)..(6 * 4 + 4)];
        assert_eq!(pixel_6[0], 255);
        assert_eq!(pixel_6[1], 255);
        assert_eq!(pixel_6[2], 255);
    }
}
