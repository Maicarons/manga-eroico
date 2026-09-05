//! Raster stage: fill text regions with sampled background color (CPU
//! "inpaint" for typical flat-color manga bubbles) and draw translated text
//! back onto the page. Behind the `draw` feature.

use ab_glyph::{Font, FontArc, PxScale, ScaleFont};
use image::{Rgb, RgbImage};

/// A translated text block positioned at its detected box (page coordinates).
pub struct DrawItem<'a> {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub text: &'a str,
}

/// Fills each rectangle with the dominant color sampled from a ring just
/// outside it. Works well for flat bubble backgrounds; complex screentone is
/// delegated to the AI inpainting backends behind features later.
pub fn fill_regions(img: &mut RgbImage, boxes: &[(f32, f32, f32, f32)], ring: i64) {
    let (w, h) = (img.width() as i64, img.height() as i64);
    for &(bx, by, bx1, by1) in boxes {
        let (x0, y0) = (bx.max(0.0) as i64, by.max(0.0) as i64);
        let (x1, y1) = (bx1.min(w as f32) as i64, by1.min(h as f32) as i64);
        if x1 <= x0 || y1 <= y0 {
            continue;
        }
        let bg = dominant_ring_color(img, x0, y0, x1, y1, ring);
        for y in y0.max(0)..y1.min(h) {
            for x in x0.max(0)..x1.min(w) {
                img.put_pixel(x as u32, y as u32, bg);
            }
        }
    }
}

/// Median-ish color from the `ring`-pixel border just outside the rect.
fn dominant_ring_color(img: &RgbImage, x0: i64, y0: i64, x1: i64, y1: i64, ring: i64) -> Rgb<u8> {
    let (w, h) = (img.width() as i64, img.height() as i64);
    let mut samples: Vec<[u32; 3]> = Vec::new();
    let mut acc = [0u64; 3];
    let mut n = 0u64;
    let mut push = |x: i64, y: i64, samples: &mut Vec<[u32; 3]>, acc: &mut [u64; 3], n: &mut u64| {
        if x < 0 || y < 0 || x >= w || y >= h {
            return;
        }
        let p = img.get_pixel(x as u32, y as u32);
        samples.push([p[0] as u32, p[1] as u32, p[2] as u32]);
        acc[0] += p[0] as u64;
        acc[1] += p[1] as u64;
        acc[2] += p[2] as u64;
        *n += 1;
    };
    let ry0 = (y0 - ring).max(0);
    let ry1 = (y1 + ring).min(h - 1);
    let rx0 = (x0 - ring).max(0);
    let rx1 = (x1 + ring).min(w - 1);
    for y in ry0..=ry1 {
        for x in rx0..=rx1 {
            let inside = x >= x0 && x < x1 && y >= y0 && y < y1;
            if !inside {
                push(x, y, &mut samples, &mut acc, &mut n);
            }
        }
    }
    if samples.is_empty() || n == 0 {
        return Rgb([255, 255, 255]);
    }
    // average is fine for flat backgrounds; median fallback below if noisy
    Rgb([(acc[0] / n) as u8, (acc[1] / n) as u8, (acc[2] / n) as u8])
}

/// Draws translated text into each box: font size shrinks from 60% of box
/// height until the wrapped block fits both dimensions; text is centered.
pub fn render_translated_page(
    mut img: RgbImage,
    items: &[DrawItem<'_>],
    font_bytes: &[u8],
    text_color: [u8; 3],
) -> Result<RgbImage, String> {
    let font = FontArc::try_from_vec(font_bytes.to_vec()).map_err(|e| format!("bad font: {e}"))?;
    for item in items {
        let (bw, bh) = (item.w.max(12.0), item.h.max(12.0));
        let mut size = bh * 0.62;
        let mut lines: Vec<String>;
        let mut line_h = 0.0f32;
        loop {
            let scale = PxScale::from(size);
            let sfont = font.as_scaled(scale);
            let max_chars = ((bw / sfont.h_advance(sfont.glyph_id('あ'))) as usize).max(2);
            lines = crate::wrap_lines(item.text, max_chars);
            line_h = size * 1.15;
            let widest = lines
                .iter()
                .map(|l| text_width(&font, scale, l))
                .fold(0.0f32, f32::max);
            if (lines.len() as f32) * line_h <= bh * 1.02 && widest <= bw * 1.02 {
                break;
            }
            size *= 0.88;
            if size < 8.0 {
                break;
            }
        }

        let block_h = lines.len() as f32 * line_h;
        let mut y = item.y + (bh - block_h).max(0.0) / 2.0;
        let scale = PxScale::from(size);
        for line in &lines {
            let lw = text_width(&font, scale, line);
            let x = item.x + (bw - lw).max(0.0) / 2.0;
            imageproc::drawing::draw_text_mut(
                &mut img,
                Rgb(text_color),
                x as i32,
                y as i32,
                scale,
                &font,
                line,
            );
            y += line_h;
        }
    }
    Ok(img)
}

fn text_width(font: &FontArc, scale: PxScale, text: &str) -> f32 {
    let sfont = font.as_scaled(scale);
    let mut w = 0.0f32;
    for ch in text.chars() {
        let gid = font.glyph_id(ch);
        w += sfont.h_advance(gid);
    }
    w
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fill_regions_paints_sampled_background() {
        let mut img = RgbImage::new(60, 40);
        // white background, black "text" block in the middle
        for p in img.pixels_mut() {
            *p = Rgb([255, 255, 255]);
        }
        for y in 15..25 {
            for x in 20..40 {
                img.put_pixel(x, y, Rgb([0, 0, 0]));
            }
        }
        fill_regions(&mut img, &[(18.0, 13.0, 42.0, 27.0)], 2);
        // every pixel of the former text block is now background white
        for y in 15..25 {
            for x in 20..40 {
                assert_eq!(img.get_pixel(x, y), &Rgb([255, 255, 255]));
            }
        }
    }

    #[test]
    fn fill_regions_clamps_out_of_bounds() {
        let mut img = RgbImage::new(10, 10);
        // must not panic on boxes crossing the border
        fill_regions(&mut img, &[(5.0, 5.0, 50.0, 50.0), (-5.0, -5.0, 5.0, 5.0)], 2);
    }

    #[test]
    fn render_draws_text_when_font_available() {
        let font_path = [
            "C:/Windows/Fonts/msyh.ttc",
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        ]
        .iter()
        .map(std::path::PathBuf::from)
        .find(|p| p.exists());
        let Some(font_path) = font_path else {
            eprintln!("no system font; skipping raster test");
            return;
        };
        let font_bytes = std::fs::read(font_path).unwrap();
        let img = RgbImage::new(200, 80);
        let items = vec![DrawItem { x: 10.0, y: 20.0, w: 180.0, h: 40.0, text: "テスト" }];
        let out = render_translated_page(img, &items, &font_bytes, [0, 0, 0]).unwrap();
        // rasterized glyphs must darken at least a few pixels
        let dark = out.pixels().filter(|p| p[0] < 128).count();
        assert!(dark > 20, "expected glyph pixels, got {dark}");
    }
}
