//! me-ocr: text recognition built on RapidOCR's PP-OCRv5 ONNX models.
//! Real inference is behind the `onnx` feature (via `ort`); everything else —
//! CTC decoding, confidence aggregation, language package selection — is pure
//! Rust and fully tested.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OcrLang {
    Zh,
    En,
    Ja,
    Ko,
}

/// One recognized line inside a detected box.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrLine {
    pub text: String,
    pub confidence: f32,
}

/// Greedy CTC decoding: collapse repeated symbols, drop blanks, map indices
/// through the charset. `charset[i]` is the label for class index `i+1`
/// (class 0 is the CTC blank).
pub fn ctc_greedy_decode(probs: &[Vec<f32>], blank: usize, charset: &[&str]) -> OcrLine {
    let mut text = String::new();
    let mut confidences: Vec<f32> = Vec::new();
    let mut prev = blank;
    for frame in probs {
        let (best_idx, best_p) = frame
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap_or((blank, &0.0));
        if best_idx != blank && best_idx != prev {
            if let Some(label) = charset.get(best_idx - 1) {
                text.push_str(label);
                confidences.push(*best_p);
            }
        }
        prev = best_idx;
    }
    let confidence = if confidences.is_empty() {
        0.0
    } else {
        confidences.iter().sum::<f32>() / confidences.len() as f32
    };
    OcrLine { text, confidence }
}

/// Decides which language package to use for a page given user hint and a
/// quick script sniff over already-recognized text (used to auto-correct the
/// source language when the user guess was wrong).
pub fn sniff_lang(text: &str) -> Option<OcrLang> {
    let (mut hangul, mut kana, mut han, mut latin) = (0, 0, 0, 0);
    for c in text.chars() {
        let u = c as u32;
        if (0xAC00..=0xD7AF).contains(&u) {
            hangul += 1;
        } else if (0x3040..=0x30FF).contains(&u) {
            kana += 1;
        } else if (0x4E00..=0x9FFF).contains(&u) || (0x3400..=0x4DBF).contains(&u) {
            han += 1;
        } else if c.is_ascii_alphabetic() {
            latin += 1;
        }
    }
    let max = hangul.max(kana).max(han).max(latin);
    if max == 0 {
        return None;
    }
    Some(if max == hangul {
        OcrLang::Ko
    } else if max == kana {
        OcrLang::Ja
    } else if max == latin && latin > han {
        OcrLang::En
    } else {
        OcrLang::Zh
    })
}

/// Provider trait; the `onnx` feature binds this to RapidOCR ONNX sessions.
pub trait OcrProvider: Send + Sync {
    fn recognize(&self, image: &[u8], lang: OcrLang) -> anyhow_lite::Result<Vec<OcrLine>>;
}

pub mod anyhow_lite {
    pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
}

/// Mock used by tests / web preview.
pub struct MockOcr;

impl OcrProvider for MockOcr {
    fn recognize(&self, _image: &[u8], _lang: OcrLang) -> anyhow_lite::Result<Vec<OcrLine>> {
        Ok(vec![OcrLine { text: "こんにちは".into(), confidence: 0.93 }])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ctc_collapses_repeats_and_blanks() {
        // charset: 0 blank, then "あ","い","う"
        let charset = ["あ", "い", "う"];
        let f = |idx: usize, p: f32| {
            let mut v = vec![0.0f32; 4];
            v[idx] = p;
            v
        };
        // frames: あ あ blank い い い blank う
        let probs = vec![
            f(1, 0.9), f(1, 0.8), f(0, 0.9), f(2, 0.7), f(2, 0.6), f(2, 0.9), f(0, 0.8), f(3, 0.95),
        ];
        let line = ctc_greedy_decode(&probs, 0, &charset);
        assert_eq!(line.text, "あいう");
        // confidence = mean over emitted chars: (0.9 + 0.7 + 0.95)/3
        assert!((line.confidence - (0.9 + 0.7 + 0.95) / 3.0).abs() < 1e-6);
    }

    #[test]
    fn ctc_blank_between_same_chars_keeps_both() {
        let charset = ["a", "b"];
        let f = |idx: usize, p: f32| {
            let mut v = vec![0.0f32; 3];
            v[idx] = p;
            v
        };
        // "a blank a" must decode to "aa" — blank separates repeated glyphs
        let probs = vec![f(1, 0.9), f(0, 0.9), f(1, 0.9)];
        assert_eq!(ctc_greedy_decode(&probs, 0, &charset).text, "aa");
    }

    #[test]
    fn ctc_empty_frames() {
        let probs = vec![vec![1.0f32, 0.0, 0.0], vec![1.0f32, 0.0, 0.0]];
        let line = ctc_greedy_decode(&probs, 0, &["x", "y"]);
        assert_eq!(line.text, "");
        assert_eq!(line.confidence, 0.0);
    }

    #[test]
    fn sniffing() {
        assert_eq!(sniff_lang("こんにちは世界"), Some(OcrLang::Ja));
        assert_eq!(sniff_lang("안녕하세요"), Some(OcrLang::Ko));
        assert_eq!(sniff_lang("你好世界"), Some(OcrLang::Zh));
        assert_eq!(sniff_lang("Hello, world!"), Some(OcrLang::En));
        assert_eq!(sniff_lang("...123"), None);
    }
}
