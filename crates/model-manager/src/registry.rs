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
                file: "en_PP-OCRv5_rec_mobile_infer.onnx",
                size_mib: 11,
                sha256: "",
            },
        ]
    }

    /// Translation models (Tencent Hunyuan Hy-MT2, GGUF quantizations).
    pub fn llm_specs() -> Vec<ModelSpec> {
        vec![
            ModelSpec {
                id: "hymt2_1.8b_q4",
                role: ModelRole::Llm,
                lang: Lang::Any,
                modelscope_repo: "Tencent-Hunyuan/Hy-MT2",
                file: "hy-mt2-1.8b-q4_0.gguf",
                size_mib: 1100,
                sha256: "",
            },
            ModelSpec {
                id: "hymt2_7b_q4",
                role: ModelRole::Llm,
                lang: Lang::Any,
                modelscope_repo: "Tencent-Hunyuan/Hy-MT2",
                file: "hy-mt2-7b-q4_0.gguf",
                size_mib: 4200,
                sha256: "",
            },
            ModelSpec {
                id: "hymt2_30b_a3b_q4",
                role: ModelRole::Llm,
                lang: Lang::Any,
                modelscope_repo: "Tencent-Hunyuan/Hy-MT2",
                file: "hy-mt2-30b-a3b-q4_0.gguf",
                size_mib: 17000,
                sha256: "",
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
                file: "aot_inpainter.onnx",
                size_mib: 90,
                sha256: "",
            },
            ModelSpec {
                id: "lama_mpe",
                role: ModelRole::Inpaint,
                lang: Lang::Any,
                modelscope_repo: "zyddnys/manga-image-translator",
                file: "lama_mpe.onnx",
                size_mib: 200,
                sha256: "",
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
            Tier::Lite => "hymt2_1.8b_q4",
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
        let m = Registry::find("rec_ja").unwrap();
        assert_eq!(m.role, ModelRole::Rec);
        assert_eq!(m.lang, Lang::Ja);
        assert!(Registry::find("nope").is_none());
    }

    #[test]
    fn tier_llm_mapping() {
        assert_eq!(Registry::llm_for_tier(Tier::Lite), "hymt2_1.8b_q4");
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
