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

/// Full language names for Hy-MT2 prompts: the model's docs require the
/// complete name ("英语"), not the ISO code ("en"). Chinese names are used
/// because the canonical prompt template is Chinese.
pub fn lang_full_name(code: &str) -> String {
    const NAMES: &[(&str, &str)] = &[
        ("zh", "中文（简体）"), ("zh-cn", "中文（简体）"), ("zh-hant", "中文（繁体）"),
        ("zh-tw", "中文（繁体）"), ("zh-hk", "中文（繁体）"), ("yue", "粤语"),
        ("en", "英语"), ("ja", "日语"), ("ko", "韩语"), ("fr", "法语"),
        ("pt", "葡萄牙语"), ("es", "西班牙语"), ("it", "意大利语"), ("de", "德语"),
        ("nl", "荷兰语"), ("pl", "波兰语"), ("cs", "捷克语"), ("ru", "俄语"),
        ("uk", "乌克兰语"), ("ar", "阿拉伯语"), ("he", "希伯来语"), ("fa", "波斯语"),
        ("tr", "土耳其语"), ("th", "泰语"), ("vi", "越南语"), ("id", "印尼语"),
        ("ms", "马来语"), ("tl", "菲律宾语"), ("fil", "菲律宾语"), ("hi", "印地语"),
        ("bn", "孟加拉语"), ("ta", "泰米尔语"), ("te", "泰卢固语"), ("mr", "马拉地语"),
        ("gu", "古吉拉特语"), ("ur", "乌尔都语"), ("km", "高棉语"), ("my", "缅甸语"),
        ("bo", "藏语"), ("kk", "哈萨克语"), ("mn", "蒙古语"), ("ug", "维吾尔语"),
    ];
    let lower = code.trim().to_ascii_lowercase();
    match NAMES.iter().find(|(c, _)| *c == lower) {
        Some((_, name)) => (*name).to_string(),
        None => code.to_string(),
    }
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
    let src = lang_full_name(source_lang);
    let tgt = lang_full_name(target_lang);
    format!(
        "将以下漫画对白从{src}翻译成{tgt}。\
保留每一行的含义、语气和情绪强度；输出长度适合漫画排字。\n{glossary_block}输出格式：每行一条，格式为\"[id] 译文\"，不要任何额外说明。\n\n{lines}"
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

pub mod openai;

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
        assert!(p.contains("从日语翻译成英语"), "{p}");
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
