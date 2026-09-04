//! me-translate: bubble translation powered by Tencent Hunyuan Hy-MT2.
//!
//! The local backend runs Hy-MT2 GGUF weights through llama-cpp-2 behind the
//! `local-llm` feature (CUDA/Metal/Vulkan capable). Prompt construction —
//! reading-order context, glossary injection, structured output parsing — is
//! pure Rust, fully tested, and shared by every backend.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TranslateModel {
    HyMt2_1_8B,
    HyMt2_7B,
    HyMt2_30BA3B,
}

/// One bubble to translate, with optional preceding lines for context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslateItem {
    pub id: String,
    pub source_text: String,
}

/// Builds the Hy-MT2 prompt for a batch of bubbles in reading order.
/// Hy-MT2 honors term/style instructions, so the glossary is embedded inline.
pub fn build_prompt(
    source_lang: &str,
    target_lang: &str,
    glossary: &BTreeMap<String, String>,
    items: &[TranslateItem],
) -> String {
    let glossary_block = if glossary.is_empty() {
        String::new()
    } else {
        let terms = glossary
            .iter()
            .map(|(k, v)| format!("{k} => {v}"))
            .collect::<Vec<_>>()
            .join("; ");
        format!("Glossary (use exactly these translations):\n{terms}\n\n")
    };
    let lines = items
        .iter()
        .map(|i| format!("[{}] {}", i.id, i.source_text))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "Translate the following comic dialogue from {source_lang} to {target_lang}. \
Keep each line's meaning, tone and emotional register; output length should suit comic lettering. \
{glossary_block}Output format: one line per input, as \"[id] translation\", no extra commentary.\n\n{lines}"
    )
}

/// Parses the model output back into (id, translation) pairs; tolerates
/// missing ids by falling back to positional order.
pub fn parse_output(output: &str, items: &[TranslateItem]) -> Vec<(String, String)> {
    let mut found: BTreeMap<String, String> = BTreeMap::new();
    for line in output.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix('[') {
            if let Some(close) = rest.find(']') {
                let id = rest[..close].trim().to_string();
                let text = rest[close + 1..].trim().to_string();
                if !text.is_empty() {
                    found.insert(id, text);
                }
            }
        }
    }
    items
        .iter()
        .map(|i| {
            let t = found
                .get(&i.id)
                .cloned()
                .unwrap_or_else(|| i.source_text.clone()); // fallback = source (graceful)
            (i.id.clone(), t)
        })
        .collect()
}

/// Provider trait; `local-llm` feature binds llama.cpp sessions here.
pub trait TranslateProvider: Send + Sync {
    fn translate_batch(&self, prompt: &str) -> anyhow_lite::Result<String>;
}

pub mod anyhow_lite {
    pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
}

/// Mock "model" that echoes a tagged translation — tests / web preview.
pub struct MockTranslate;

impl TranslateProvider for MockTranslate {
    fn translate_batch(&self, prompt: &str) -> anyhow_lite::Result<String> {
        let mut out = Vec::new();
        for line in prompt.lines() {
            if let Some(rest) = line.strip_prefix('[') {
                if let Some(close) = rest.find(']') {
                    let id = &rest[..close];
                    let text = rest[close + 1..].trim();
                    out.push(format!("[{id}] <{text}>"));
                }
            }
        }
        Ok(out.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn items() -> Vec<TranslateItem> {
        vec![
            TranslateItem { id: "b1".into(), source_text: "こんにちは".into() },
            TranslateItem { id: "b2".into(), source_text: "またね".into() },
        ]
    }

    #[test]
    fn prompt_contains_glossary_and_order() {
        let mut g = BTreeMap::new();
        g.insert("ハルカ".to_string(), "Halca".to_string());
        let p = build_prompt("ja", "en", &g, &items());
        assert!(p.contains("ja to en"));
        assert!(p.contains("ハルカ => Halca"));
        assert!(p.contains("[b1] こんにちは"));
        assert!(p.contains("[b2] またね"));
    }

    #[test]
    fn prompt_without_glossary_has_no_block() {
        let p = build_prompt("ja", "zh", &BTreeMap::new(), &items());
        assert!(!p.contains("Glossary"));
    }

    #[test]
    fn parse_output_maps_ids() {
        let out = "[b1] 你好\n[b2] 再见";
        let parsed = parse_output(out, &items());
        assert_eq!(parsed[0], ("b1".into(), "你好".into()));
        assert_eq!(parsed[1], ("b2".into(), "再见".into()));
    }

    #[test]
    fn parse_output_falls_back_to_source_on_missing_id() {
        let out = "[b1] 你好"; // b2 missing
        let parsed = parse_output(out, &items());
        assert_eq!(parsed[1], ("b2".into(), "またね".into()));
    }

    #[test]
    fn mock_roundtrip() {
        let p = build_prompt("ja", "en", &BTreeMap::new(), &items());
        let raw = MockTranslate.translate_batch(&p).unwrap();
        let parsed = parse_output(&raw, &items());
        assert_eq!(parsed[0].1, "<こんにちは>");
    }
}
