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
    /// Full download URL for non-ModelScope sources (e.g. GitHub releases);
    /// empty = resolve via ModelScope from `modelscope_repo` + `file`.
    pub url_override: &'static str,
}

pub struct Registry;

impl Registry {
    /// OCR models (RapidOCR / PaddleOCR PP-OCRv5, Apache-2.0).
    pub fn ocr_specs() -> Vec<ModelSpec> {
        // NOTE: PP-OCRv5 `ch` recognizer is a mixed zh/en/ja/ko model, so
        // Japanese and Chinese share one recognizer + dict (verified against
        // the RapidAI/RapidOCR repo tree on ModelScope, 2026-09).
        vec![
            ModelSpec {
                id: "ppocrv5_det",
                role: ModelRole::Detect,
                lang: Lang::Any,
                modelscope_repo: "RapidAI/RapidOCR",
                file: "onnx/PP-OCRv5/det/ch_PP-OCRv5_det_mobile.onnx",
                size_mib: 5,
                sha256: "",
                url_override: "",
            },
            ModelSpec {
                id: "ppocrv5_cls",
                role: ModelRole::Cls,
                lang: Lang::Any,
                modelscope_repo: "RapidAI/RapidOCR",
                file: "onnx/PP-OCRv5/cls/ch_PP-LCNet_x0_25_textline_ori_cls_mobile.onnx",
                size_mib: 2,
                sha256: "",
                url_override: "",
            },
            ModelSpec {
                id: "rec_mixed",
                role: ModelRole::Rec,
                lang: Lang::Any,
                modelscope_repo: "RapidAI/RapidOCR",
                file: "onnx/PP-OCRv5/rec/ch_PP-OCRv5_rec_mobile.onnx",
                size_mib: 12,
                sha256: "",
                url_override: "",
            },
            ModelSpec {
                id: "dict_mixed",
                role: ModelRole::Dict,
                lang: Lang::Any,
                modelscope_repo: "RapidAI/RapidOCR",
                file: "paddle/PP-OCRv5/rec/ch_PP-OCRv5_rec_mobile/ppocrv5_dict.txt",
                size_mib: 1,
                sha256: "",
                url_override: "",
            },
            ModelSpec {
                id: "rec_en",
                role: ModelRole::Rec,
                lang: Lang::En,
                modelscope_repo: "RapidAI/RapidOCR",
                file: "onnx/PP-OCRv5/rec/en_PP-OCRv5_rec_mobile.onnx",
                size_mib: 11,
                sha256: "",
                url_override: "",
            },
            ModelSpec {
                id: "dict_en",
                role: ModelRole::Dict,
                lang: Lang::En,
                modelscope_repo: "RapidAI/RapidOCR",
                file: "paddle/PP-OCRv5/rec/en_PP-OCRv5_rec_mobile/ppocrv5_en_dict.txt",
                size_mib: 1,
                sha256: "",
                url_override: "",
            },
            ModelSpec {
                id: "rec_ko",
                role: ModelRole::Rec,
                lang: Lang::Ko,
                modelscope_repo: "RapidAI/RapidOCR",
                file: "onnx/PP-OCRv5/rec/korean_PP-OCRv5_rec_mobile.onnx",
                size_mib: 11,
                sha256: "",
                url_override: "",
            },
            ModelSpec {
                id: "dict_ko",
                role: ModelRole::Dict,
                lang: Lang::Ko,
                modelscope_repo: "RapidAI/RapidOCR",
                file: "paddle/PP-OCRv5/rec/korean_PP-OCRv5_rec_mobile/ppocrv5_korean_dict.txt",
                size_mib: 1,
                sha256: "",
                url_override: "",
            },
        ]
    }

    /// Translation models (Tencent Hunyuan Hy-MT2, GGUF quantizations).
    pub fn llm_specs() -> Vec<ModelSpec> {
        vec![
            ModelSpec {
                id: "hymt2_1.8b_iq2_m",
                role: ModelRole::Llm,
                lang: Lang::Any,
                modelscope_repo: "unsloth/Hy-MT2-1.8B-GGUF",
                file: "Hy-MT2-1.8B-UD-IQ2_M.gguf",
                size_mib: 723,
                sha256: "",
                url_override: "",
            },
            ModelSpec {
                id: "hymt2_7b_q4",
                role: ModelRole::Llm,
                lang: Lang::Any,
                modelscope_repo: "unsloth/Hy-MT2-7B-GGUF",
                file: "Hy-MT2-7B-UD-Q4_K_XL.gguf",
                size_mib: 4783,
                sha256: "",
                url_override: "",
            },
            ModelSpec {
                id: "hymt2_30b_a3b_q4",
                role: ModelRole::Llm,
                lang: Lang::Any,
                modelscope_repo: "alphaZimuth/Hy-MT2-30B-A3B-APEX-GGUF",
                file: "Hy-MT2-30B-A3B-APEX-Imatrix-I-Nano.gguf",
                size_mib: 12448,
                sha256: "",
                url_override: "",
            },
        ]
    }

    /// Inpainting models (AOT fast / LaMa-mpe quality).
    pub fn inpaint_specs() -> Vec<ModelSpec> {
        vec![
            ModelSpec {
                id: "aot_inpainter",
                role: ModelRole::Inpaint,
                lang: Lang::Any,
                modelscope_repo: "zyddnys/manga-image-translator",
                file: "inpainting.ckpt",
                url_override: "https://github.com/zyddnys/manga-image-translator/releases/latest/download/inpainting.ckpt",
                size_mib: 90,
                sha256: "",
            },
            ModelSpec {
                id: "lama_mpe",
                role: ModelRole::Inpaint,
                lang: Lang::Any,
                modelscope_repo: "zyddnys/manga-image-translator",
                file: "inpainting_lama_mpe.ckpt",
                size_mib: 200,
                sha256: "",
                url_override: "https://github.com/zyddnys/manga-image-translator/releases/latest/download/inpainting_lama_mpe.ckpt",
            },
        ]
    }

    pub fn all() -> Vec<ModelSpec> {
        let mut v = Self::ocr_specs();
        v.extend(Self::llm_specs());
        v.extend(Self::inpaint_specs());
        v
    }

    pub fn find(id: &str) -> Option<ModelSpec> {
        Self::all().into_iter().find(|m| m.id == id)
    }

    /// The default LLM for a tier (see development-plan §4.2 matrix).
    pub fn llm_for_tier(tier: Tier) -> &'static str {
        match tier {
            Tier::Lite => "hymt2_1.8b_iq2_m",
            Tier::Standard => "hymt2_7b_q4",
            Tier::Pro => "hymt2_30b_a3b_q4",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique() {
        let all = Registry::all();
        let mut ids: Vec<_> = all.iter().map(|m| m.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), Registry::all().len());
    }

    #[test]
    fn find_works() {
        let m = Registry::find("rec_mixed").unwrap();
        assert_eq!(m.role, ModelRole::Rec);
        assert_eq!(m.lang, Lang::Any);
        assert!(Registry::find("nope").is_none());
    }

    #[test]
    fn tier_llm_mapping() {
        assert_eq!(Registry::llm_for_tier(Tier::Lite), "hymt2_1.8b_iq2_m");
        assert_eq!(Registry::llm_for_tier(Tier::Standard), "hymt2_7b_q4");
        assert_eq!(Registry::llm_for_tier(Tier::Pro), "hymt2_30b_a3b_q4");
    }

    #[test]
    fn all_repos_are_modelscope() {
        for m in Registry::all() {
            assert!(m.modelscope_repo.contains('/'), "{} repo malformed", m.id);
            assert!(!m.file.is_empty());
            assert!(m.size_mib > 0);
        }
    }
}
