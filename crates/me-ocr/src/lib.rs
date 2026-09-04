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
