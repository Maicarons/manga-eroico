//! me-detect: text-region detection over manga pages.
//!
//! The default backend is comic-text-detector (ONNX) executed via `ort`;
//! it is gated behind the `onnx` feature so CI and the web preview run with
//! zero model dependencies. Geometry helpers are pure and always available.

use serde::{Deserialize, Serialize};

/// A quadrilateral text region (clockwise, pixel coords).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextBox {
    pub points: [[f32; 2]; 4],
    pub score: f32,
}

impl TextBox {
    pub fn bbox(&self) -> (f32, f32, f32, f32) {
        let xs: Vec<f32> = self.points.iter().map(|p| p[0]).collect();
        let ys: Vec<f32> = self.points.iter().map(|p| p[1]).collect();
        (
            xs.iter().cloned().fold(f32::MAX, f32::min),
            ys.iter().cloned().fold(f32::MAX, f32::min),
            xs.iter().cloned().fold(f32::MIN, f32::max),
            ys.iter().cloned().fold(f32::MIN, f32::max),
        )
    }
}

/// Shoelace area of the polygon.
pub fn polygon_area(points: &[[f32; 2]; 4]) -> f32 {
    let mut area = 0.0;
    for i in 0..4 {
        let j = (i + 1) % 4;
        area += points[i][0] * points[j][1] - points[j][0] * points[i][1];
    }
    (area / 2.0).abs()
}

/// Intersection-over-union of two axis-aligned bboxes `(x0,y0,x1,y1)`.
pub fn bbox_iou(a: (f32, f32, f32, f32), b: (f32, f32, f32, f32)) -> f32 {
    let ix0 = a.0.max(b.0);
    let iy0 = a.1.max(b.1);
    let ix1 = a.2.min(b.2);
    let iy1 = a.3.min(b.3);
    let iw = (ix1 - ix0).max(0.0);
    let ih = (iy1 - iy0).max(0.0);
    let inter = iw * ih;
    let area_a = (a.2 - a.0).max(0.0) * (a.3 - a.1).max(0.0);
    let area_b = (b.2 - b.0).max(0.0) * (b.3 - b.1).max(0.0);
    let union = area_a + area_b - inter;
    if union <= 0.0 {
        0.0
    } else {
        inter / union
    }
}

/// Drops overlapping boxes (IoU above `threshold`, keeping the higher score)
/// and low-confidence ones.
pub fn filter_boxes(mut boxes: Vec<TextBox>, score_threshold: f32, iou_threshold: f32) -> Vec<TextBox> {
    boxes.retain(|b| b.score >= score_threshold);
    boxes.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    let mut kept: Vec<TextBox> = Vec::new();
    for b in boxes {
        let bb = b.bbox();
        if kept
            .iter()
            .all(|k| bbox_iou(k.bbox(), bb) <= iou_threshold)
        {
            kept.push(b);
        }
    }
    kept
}

/// Provider trait implemented by the ONNX backend (`onnx` feature) and mocks.
pub trait DetectProvider: Send + Sync {
    /// Runs detection over raw image bytes (PNG/JPEG) and returns filtered
    /// text boxes.
    fn detect(&self, image: &[u8]) -> anyhow_lite::Result<Vec<TextBox>>;
}

/// Minimal Result alias without pulling full `anyhow` into the API.
#[cfg(feature = "real")]
pub mod real;

pub mod anyhow_lite {
    pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
}

/// Always-available mock used in tests and the web preview.
pub struct MockDetect;

impl DetectProvider for MockDetect {
    fn detect(&self, _image: &[u8]) -> anyhow_lite::Result<Vec<TextBox>> {
        Ok(vec![TextBox {
            points: [[10.0, 10.0], [200.0, 10.0], [200.0, 60.0], [10.0, 60.0]],
            score: 0.95,
        }])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn box_at(x0: f32, y0: f32, x1: f32, y1: f32, score: f32) -> TextBox {
        TextBox { points: [[x0, y0], [x1, y0], [x1, y1], [x0, y1]], score }
    }

    #[test]
    fn area_of_rect() {
        let b = box_at(0.0, 0.0, 10.0, 5.0, 1.0);
        assert_eq!(polygon_area(&b.points), 50.0);
    }

    #[test]
    fn iou_basics() {
        let a = (0.0, 0.0, 10.0, 10.0);
        assert_eq!(bbox_iou(a, a), 1.0);
        assert_eq!(bbox_iou(a, (20.0, 20.0, 30.0, 30.0)), 0.0);
        let half = (0.0, 0.0, 10.0, 5.0);
        let iou = bbox_iou(a, half);
        assert!((iou - 0.5).abs() < 1e-6);
    }

    #[test]
    fn filter_drops_low_score_and_duplicates() {
        let boxes = vec![
            box_at(0.0, 0.0, 10.0, 10.0, 0.9),
            box_at(1.0, 1.0, 11.0, 11.0, 0.95), // near-duplicate, higher score
            box_at(100.0, 100.0, 120.0, 120.0, 0.5),
            box_at(200.0, 200.0, 210.0, 210.0, 0.3), // below threshold
        ];
        let kept = filter_boxes(boxes, 0.4, 0.5);
        assert_eq!(kept.len(), 2);
        assert_eq!(kept[0].score, 0.95); // duplicate resolved to best score
    }

    #[test]
    fn mock_returns_one_box() {
        let boxes = MockDetect.detect(b"fakepng").unwrap();
        assert_eq!(boxes.len(), 1);
    }
}
