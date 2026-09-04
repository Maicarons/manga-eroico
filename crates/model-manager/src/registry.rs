//! Model registry: every artifact the app may download, with its ModelScope
//! repo and expected checksum. Sizes are approximate (used for UI progress);
//! `sha256` is authoritative and verified after download.

use crate::hardware::Tier;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelRole {
    /// Text-region detection (comic-text-detector or PP-OCR det).
    Detect,
    /// Direction classifier.
    Cls,
    /// Per-language recognizer weights.
    Rec,
    /// Recognizer dictionary.
    Dict,
    /// Text removal / inpainting.
    Inpaint,
    /// Hy-MT2 translation weights (GGUF).
    Llm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Lang {
    Zh,
    En,
    Ja,
    Ko,
    /// Language-independent weights.
    Any,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelSpec {
    pub id: &'static str,
    pub role: ModelRole,
    pub lang: Lang,
    pub modelscope_repo: &'static str,
    pub file: &'static str,
    /// Approximate size in MiB, for the download UI.
    pub size_mib: u64,
    /// SHA256 of the file; empty string = not pinned (dev only).
    pub sha256: &'static str,
}

pub struct Registry;

impl Registry {
    /// OCR models (RapidOCR / PaddleOCR PP-OCRv5, Apache-2.0).
    pub fn ocr_specs() -> Vec<ModelSpec> {
        vec![
            ModelSpec {
                id: "ppocrv5_det",
                role: ModelRole::Detect,
                lang: Lang::Any,
                modelscope_repo: "RapidAI/RapidOCR",
                file: "ch_PP-OCRv5_mobile_det.onnx",
                size_mib: 5,
                sha256: "",
            },
            ModelSpec {
                id: "ppocrv5_cls",
                role: ModelRole::Cls,
                lang: Lang::Any,
                modelscope_repo: "RapidAI/RapidOCR",
                file: "ch_ppocr_mobile_v2.0_cls_infer.onnx",
                size_mib: 1,
                sha256: "",
            },
            ModelSpec {
                id: "rec_ja",
                role: ModelRole::Rec,
                lang: Lang::Ja,
                modelscope_repo: "RapidAI/RapidOCR",
                file: "japan_PP-OCRv5_rec_mobile_infer.onnx",
                size_mib: 11,
                sha256: "",
            },
            ModelSpec {
                id: "dict_ja",
                role: ModelRole::Dict,
                lang: Lang::Ja,
                modelscope_repo: "RapidAI/RapidOCR",
                file: "japan_dict.txt",
                size_mib: 1,
                sha256: "",
            },
            ModelSpec {
                id: "rec_ko",
                role: ModelRole::Rec,
                lang: Lang::Ko,
                modelscope_repo: "RapidAI/RapidOCR",
                file: "korean_PP-OCRv5_rec_mobile_infer.onnx",
                size_mib: 10,
                sha256: "",
            },
            ModelSpec {
                id: "dict_ko",
                role: ModelRole::Dict,
                lang: Lang::Ko,
                modelscope_repo: "RapidAI/RapidOCR",
                file: "ppocrv5_korean_dict.txt",
                size_mib: 1,
                sha256: "",
            },
            ModelSpec {
                id: "rec_zh",
                role: ModelRole::Rec,
                lang: Lang::Zh,
                modelscope_repo: "RapidAI/RapidOCR",
                file: "ch_PP-OCRv5_rec_mobile_infer.onnx",
                size_mib: 11,
                sha256: "",
            },
            ModelSpec {
                id: "rec_en",
                role: ModelRole::Rec,
                lang: Lang::En,
                modelscope_repo: "RapidAI/RapidOCR",
