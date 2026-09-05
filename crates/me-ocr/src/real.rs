//! Real ONNX text recognition (PP-OCRv5 `ch` mobile rec — mixed zh/en/ja/ko)
//! behind the `real` feature.

use crate::anyhow_lite::Result;
use crate::{ctc_greedy_decode, OcrLang, OcrLine};
use ort::session::builder::GraphOptimizationLevel;
use ort::session::Session;
use ort::value::Value;
use std::path::Path;
use std::sync::Mutex;

pub struct OnnxOcr {
    session: Mutex<Session>,
    /// CTC charset: index 0 is blank, then dict lines, then a space token.
    charset: Vec<String>,
}

impl OnnxOcr {
    /// `model_path` = PP-OCRv5 rec onnx, `dict_path` = matching ppocrv5 dict.
    pub fn load(model_path: impl AsRef<Path>, dict_path: impl AsRef<Path>) -> Result<Self> {
        let mut builder = Session::builder()
            .unwrap()
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .unwrap()
            .with_intra_threads(threads())
            .unwrap();
        #[cfg(feature = "cuda")]
        let builder = {
            use ort::execution_providers::CUDAExecutionProvider;
            eprintln!("[gpu] requesting CUDA execution provider (falls back to CPU)");
            builder.with_execution_providers([CUDAExecutionProvider::default().build()]).unwrap()
        };
        let session = builder.commit_from_file(model_path)?;
        let dict = std::fs::read_to_string(dict_path)?;
        // PaddleOCR CTCLabelEncode layout: ['blank'] + dict + [' ']
        let mut charset: Vec<String> = Vec::with_capacity(dict.lines().count() + 2);
        charset.push(String::new()); // 0 = CTC blank
        for line in dict.lines() {
            charset.push(line.trim_end_matches('\r').to_string());
        }
        charset.push(" ".to_string()); // space class
        Ok(Self { session: Mutex::new(session), charset })
    }

    pub fn charset_len(&self) -> usize {
        self.charset.len()
    }
}

fn threads() -> usize {
    std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4)
}

/// Prepends blank + appends space, returning `&str` views for CTC decoding.
fn charset_refs(c: &[String]) -> Vec<&str> {
    c.iter().map(|s| s.as_str()).collect()
}

impl OnnxOcr {
    fn run_tensor(&self, chw: Vec<f32>, width: usize) -> Result<Vec<Vec<f32>>> {
        let input = Value::from_array((vec![1usize, 3, 48usize, width], chw))?;
        let mut session = self.session.lock().unwrap();
        let outputs = session.run(ort::inputs![input])?;
        let (out_name, _) = outputs.iter().next().ok_or("rec model has no outputs")?;
        let (shape, raw) = outputs[out_name].try_extract_tensor::<f32>()?;
        let raw = raw.to_vec();
        let dims: Vec<i64> = shape.iter().copied().collect();
        // shape [1, T, C]
        let (t, c) = (dims[1] as usize, dims[2] as usize);
        Ok((0..t)
            .map(|ti| raw[ti * c..(ti + 1) * c].to_vec())
            .collect())
    }
}

impl crate::OcrProvider for OnnxOcr {
    fn recognize(&self, png: &[u8], lang: OcrLang) -> Result<Vec<OcrLine>> {
        let _ = lang; // the mixed model handles zh/ja/ko/en in one pass
        let img = image::load_from_memory(png)?.to_rgb8();
        // resize to height 48, keep aspect, clamp width to [16, 1280]
        let (w0, h0) = (img.width() as usize, img.height() as usize);
        let scale = 48.0 / h0 as f32;
        let width = ((w0 as f32 * scale).round() as usize).clamp(16, 1280);
        let resized =
            image::imageops::resize(&img, width as u32, 48, image::imageops::FilterType::Lanczos3);

        // CHW, mean=std=0.5
        let mut chw = vec![0f32; 3 * 48 * width];
        for (i, px) in resized.pixels().enumerate() {
            let (x, y) = (i % width, i / width);
            // PaddleOCR ONNX exports expect BGR channel order
            for (c, ch) in [2usize, 1, 0].iter().enumerate() {
                chw[c * 48 * width + y * width + x] = px[*ch] as f32 / 255.0 / 0.5 - 1.0;
            }
        }

        let probs = self.run_tensor(chw, width)?;
        let charset = charset_refs(&self.charset);
        let line = ctc_greedy_decode(&probs, 0, &charset);
        Ok(vec![line])
    }
}
