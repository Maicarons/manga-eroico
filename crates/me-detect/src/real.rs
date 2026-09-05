//! Real ONNX text detection (PP-OCRv5 det mobile) behind the `real` feature.
//!
//! The DBNet post-process here is a pragmatic simplification: probability map
//! → fixed threshold → connected components (BFS) → axis-aligned boxes.
//! Manga bubble text is mostly axis-aligned, so this works well in practice;
//! a full shrink-map polygon regression can replace it later without touching
//! the `DetectProvider` trait.

use crate::anyhow_lite::Result;
use crate::{TextBox, DetectProvider};
use ort::session::builder::GraphOptimizationLevel;
use ort::session::Session;
use ort::value::Value;
use std::path::Path;

pub struct OnnxDetect {
    session: std::sync::Mutex<Session>,
}

impl OnnxDetect {
    pub fn load(model_path: impl AsRef<Path>) -> Result<Self> {
        let session = Session::builder()
            .unwrap()
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .unwrap()
            .with_intra_threads(threads())
            .unwrap()
            .commit_from_file(model_path)?;
        Ok(Self { session: std::sync::Mutex::new(session) })
    }
}

fn threads() -> usize {
    std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4)
}

/// DBNet connected-components post-process on the probability map.
/// Returns axis-aligned boxes in map coordinates.
pub fn components_to_boxes(
    prob: &[f32],
    w: usize,
    h: usize,
    threshold: f32,
    min_pixels: usize,
) -> Vec<[[f32; 2]; 4]> {
    let mut visited = vec![false; w * h];
    let mut boxes = Vec::new();
    let mut stack: Vec<usize> = Vec::new();
    for start in 0..w * h {
        if visited[start] || prob[start] < threshold {
            continue;
        }
        stack.clear();
        stack.push(start);
        visited[start] = true;
        let (mut min_x, mut min_y, mut max_x, mut max_y) = (w, h, 0usize, 0usize);
        let mut count = 0usize;
        while let Some(idx) = stack.pop() {
            let (x, y) = (idx % w, idx / w);
            count += 1;
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x);
            max_y = max_y.max(y);
            for (dx, dy) in [(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
                let nx = x as i32 + dx;
                let ny = y as i32 + dy;
                if nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32 {
                    continue;
                }
                let n = ny as usize * w + nx as usize;
                if !visited[n] && prob[n] >= threshold {
                    visited[n] = true;
                    stack.push(n);
                }
            }
        }
        if count >= min_pixels {
            let pad = 2.0f32;
            let (x0, y0) = ((min_x as f32 - pad).max(0.0), (min_y as f32 - pad).max(0.0));
            let (x1, y1) = ((max_x as f32 + pad).min(w as f32), (max_y as f32 + pad).min(h as f32));
            // DB unclip (PaddleOCR det unclip_ratio = 1.5): expand the polygon
            // outward by area / perimeter * ratio so tight probability islands
            // recover the full glyph height.
            let bw = (x1 - x0).max(1.0);
            let bh = (y1 - y0).max(1.0);
            let offset = (bw * bh) / (2.0 * (bw + bh)) * 1.5;
            let (ux0, uy0) = ((x0 - offset).max(0.0), (y0 - offset).max(0.0));
            let (ux1, uy1) = ((x1 + offset).min(w as f32), (y1 + offset).min(h as f32));
            boxes.push([[ux0, uy0], [ux1, uy0], [ux1, uy1], [ux0, uy1]]);
        }
    }
    boxes
}

impl DetectProvider for OnnxDetect {
    fn detect(&self, image: &[u8]) -> Result<Vec<TextBox>> {
        let img = image::load_from_memory(image)?;
        let (w0, h0) = (img.width() as usize, img.height() as usize);
        // longest side to 960, dims multiple of 32
        let scale = 960.0 / w0.max(h0) as f32;
        let nw = (((w0 as f32 * scale).round() as usize).max(32).next_multiple_of(32)) as u32;
        let nh = (((h0 as f32 * scale).round() as usize).max(32).next_multiple_of(32)) as u32;
        let resized = img
            .resize_exact(nw, nh, image::imageops::FilterType::Lanczos3)
            .to_rgb8();

        // NCHW normalized with PP-OCR ImageNet stats
        let mean = [0.485f32, 0.456, 0.406];
        let std = [0.229f32, 0.224, 0.225];
        let (w, h) = (nw as usize, nh as usize);
        let mut data = vec![0f32; 3 * w * h];
        for (i, px) in resized.pixels().enumerate() {
            let (x, y) = (i % w, i / w);
            for c in 0..3 {
                data[c * w * h + y * w + x] = (px[c] as f32 / 255.0 - mean[c]) / std[c];
            }
        }

        let input = Value::from_array((vec![1usize, 3, h, w], data))?;
        let mut session = self.session.lock().unwrap();
        let outputs = session.run(ort::inputs![input])?;
        // take the first output: [1, 1, H, W] probability map (dims = input)
        let (out_name, _) = outputs.iter().next().ok_or("det model has no outputs")?;
        let raw = outputs[out_name].try_extract_tensor::<f32>()?.1.to_vec();

        let mut boxes = components_to_boxes(&raw, w, h, 0.2, 40);

        // map back to original image coords, drop degenerate boxes
        let inv = 1.0 / scale;
        boxes.retain(|b| {
            let bw = (b[1][0] - b[0][0]).abs();
            let bh = (b[2][1] - b[1][1]).abs();
            bw * scale > 8.0 && bh * scale > 8.0
        });
        Ok(boxes
            .into_iter()
            .map(|b| {
                let points = b.map(|pt| [pt[0] * inv, pt[1] * inv]);
                TextBox { points, score: 1.0 }
            })
            .collect())
    }
}
