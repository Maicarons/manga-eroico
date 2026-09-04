//! me-polish: the optional "polish" workflow node.
//!
//! Takes machine-translated bubbles for a whole chapter, asks an
//! OpenAI-compatible chat endpoint to (1) analyse chapter-level style and
//! consistency, then (2) return a polished variant of every bubble. Design
//! rules (development-plan §3.3):
//! - request aggregates the entire chapter (page/position hints + glossary)
//! - API keys never live in project files; the caller injects them
//! - failures degrade gracefully: bubbles keep their machine translation

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolishConfig {
    /// e.g. "https://api.openai.com/v1" or "http://127.0.0.1:1234/v1"
    pub base_url: String,
    pub model: String,
    /// Bearer token; injected at call time from the system credential store.
    pub api_key: Option<String>,
    pub temperature: f32,
    #[serde(default = "default_retries")]
    pub max_retries: u32,
}

fn default_retries() -> u32 {
    2
}

impl Default for PolishConfig {
    fn default() -> Self {
        Self {
            base_url: String::new(),
            model: String::new(),
            api_key: None,
            temperature: 0.3,
            max_retries: 2,
        }
    }
}

/// One translated bubble awaiting polish.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bubble {
    /// Stable id for mapping results back (e.g. "ch1_p003_b02").
    pub id: String,
    pub page: u32,
    /// 1-based reading position inside the page (manga: right-to-left).
    pub position: u32,
    pub source_text: String,
    pub machine_translation: String,
}

/// Chapter context handed to the node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterContext {
    pub chapter_title: String,
    pub source_lang: String,
    pub target_lang: String,
    /// term -> preferred translation
    pub glossary: std::collections::BTreeMap<String, String>,
    pub bubbles: Vec<Bubble>,
}

/// Result per bubble.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PolishedBubble {
    pub id: String,
    pub polished: String,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PolishResult {
    pub analysis: String,
    pub items: Vec<PolishedBubble>,
}

/// Builds the chat-completions request body for a chapter.
pub fn build_request_body(ctx: &ChapterContext, cfg: &PolishConfig) -> serde_json::Value {
    let glossary = if ctx.glossary.is_empty() {
        "(none)".to_string()
    } else {
        ctx.glossary
            .iter()
            .map(|(k, v)| format!("- {k} -> {v}"))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let bubbles = ctx
        .bubbles
        .iter()
        .map(|b| {
            serde_json::json!({
                "id": b.id, "page": b.page, "position": b.position,
                "source": b.source_text, "translation": b.machine_translation,
            })
        })
        .collect::<Vec<_>>();

    let system = "You are a professional comic/manga localization editor. \
You will receive machine-translated dialogue bubbles from one chapter, in reading order. \
First analyse chapter-level consistency (character names, tone, recurring phrases), \
then polish every bubble's translation for naturalness while preserving meaning, \
speaker's voice and length constraints of comic lettering. \
Reply ONLY with a JSON object: {\"analysis\": string, \"items\": [{\"id\": string, \"polished\": string, \"note\": string|null}]} \
covering EVERY input id.";

    let user = serde_json::json!({
        "chapter": ctx.chapter_title,
        "source_language": ctx.source_lang,
        "target_language": ctx.target_lang,
        "glossary": glossary,
