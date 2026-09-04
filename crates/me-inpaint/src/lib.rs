//! me-inpaint: text removal. Two backends behind the `onnx` feature —
//! AOT (fast) and LaMa-mpe (quality) — selectable per tier. Mask preparation
//! (binarize + dilate) is pure Rust and always tested.

use image::{DynamicImage, GrayImage, Luma};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InpaintModel {
    /// Fast, lighter — lite tier default.
    Aot,
    /// Higher quality — standard/pro tiers.
    LamaMpe,
}

/// Binarizes a raw grayscale mask into 0/255 and dilates it by `radius` px
/// (square structuring element) so inpainting covers antialiased glyph edges.
pub fn prepare_mask(raw: &[u8], width: u32, height: u32, threshold: u8, radius: u32) -> GrayImage {
    let mut img = GrayImage::from_raw(width, height, raw.to_vec())
        .expect("raw buffer size must equal width*height");
    for px in img.pixels_mut() {
        px.0[0] = if px.0[0] >= threshold { 255 } else { 0 };
    }
    dilate(&img, radius)
}

/// Square-kernel dilation on a binary mask.
pub fn dilate(img: &GrayImage, radius: u32) -> GrayImage {
    let (w, h) = img.dimensions();
    if radius == 0 {
        return img.clone();
    }
    let src: Vec<u8> = img.pixels().map(|p| p.0[0]).collect();
    let mut out = GrayImage::new(w, h);
    let r = radius as i64;
    for y in 0..h as i64 {
        for x in 0..w as i64 {
            let mut hit = 0u8;
            'search: for dy in -r..=r {
                for dx in -r..=r {
                    let nx = x + dx;
                    let ny = y + dy;
                    if nx < 0 || ny < 0 || nx >= w as i64 || ny >= h as i64 {
                        continue;
                    }
                    if src[(ny as u32 * w + nx as u32) as usize] == 255 {
                        hit = 255;
                        break 'search;
                    }
                }
            }
            out.put_pixel(x as u32, y as u32, Luma([hit]));
        }
    }
    out
}

/// Coverage ratio of white pixels — the UI uses this to warn about
/// suspiciously huge masks (e.g. detection leaked onto artwork).
pub fn mask_coverage(img: &GrayImage) -> f32 {
    let (w, h) = img.dimensions();
    if w == 0 || h == 0 {
        return 0.0;
    }
    let white = img.pixels().filter(|p| p.0[0] == 255).count();
    white as f32 / (w * h) as f32
}

/// Provider trait; `onnx` feature binds AOT/LaMa sessions to this.
pub trait InpaintProvider: Send + Sync {
    fn inpaint(&self, image: &DynamicImage, mask: &GrayImage, model: InpaintModel) -> anyhow_lite::Result<DynamicImage>;
}

pub mod anyhow_lite {
    pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
}

/// Mock: returns the input unchanged (web preview / tests).
pub struct MockInpaint;

impl InpaintProvider for MockInpaint {
    fn inpaint(&self, image: &DynamicImage, _mask: &GrayImage, _model: InpaintModel) -> anyhow_lite::Result<DynamicImage> {
        Ok(image.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mask(w: u32, h: u32, paint: impl Fn(u32, u32) -> u8) -> GrayImage {
        let mut img = GrayImage::new(w, h);
        for y in 0..h {
            for x in 0..w {
                img.put_pixel(x, y, Luma([paint(x, y)]));
            }
        }
        img
    }

    #[test]
    fn dilation_grows_region() {
        let m = mask(9, 9, |x, y| if x == 4 && y == 4 { 255 } else { 0 });
        let d = dilate(&m, 1);
        assert_eq!(mask_coverage(&d), 9.0 / 81.0); // 3x3 square
        let d2 = dilate(&m, 2);
        assert_eq!(mask_coverage(&d2), 25.0 / 81.0); // 5x5 square
    }

    #[test]
    fn binarize_with_threshold() {
        let raw: Vec<u8> = (0..25u32).map(|i| (i * 10) as u8).collect(); // 0..=240, no overflow
        let m = prepare_mask(&raw, 5, 5, 128, 0);
        let white = m.pixels().filter(|p| p.0[0] == 255).count();
        // values >= 128: i*10 >= 128 -> i >= 13 (130..=240) -> 12 values
        assert_eq!(white, 12);
    }

    #[test]
    fn coverage_bounds() {
        let m = mask(4, 4, |_, _| 255);
        assert_eq!(mask_coverage(&m), 1.0);
        let m = mask(4, 4, |_, _| 0);
        assert_eq!(mask_coverage(&m), 0.0);
    }
}
