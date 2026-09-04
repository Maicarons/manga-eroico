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
