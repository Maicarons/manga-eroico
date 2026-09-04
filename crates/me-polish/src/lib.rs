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
        "bubbles": bubbles,
    });

    serde_json::json!({
        "model": cfg.model,
        "temperature": cfg.temperature,
        "response_format": { "type": "json_object" },
        "messages": [
            { "role": "system", "content": system },
            { "role": "user", "content": user.to_string() },
        ],
    })
}

/// Extracts [`PolishResult`] from an OpenAI chat-completions response.
pub fn parse_response(body: &serde_json::Value) -> Result<PolishResult, PolishError> {
    let content = body
        .pointer("/choices/0/message/content")
        .and_then(|v| v.as_str())
        .ok_or_else(|| PolishError::BadResponse("missing choices[0].message.content".into()))?;
    // Models sometimes wrap JSON in code fences; strip them.
    let trimmed = content.trim();
    let trimmed = trimmed
        .strip_prefix("```json")
        .or_else(|| trimmed.strip_prefix("```"))
        .unwrap_or(trimmed)
        .strip_suffix("```")
        .unwrap_or(trimmed)
        .trim();
    let result: PolishResult =
        serde_json::from_str(trimmed).map_err(|e| PolishError::BadResponse(format!("invalid JSON payload: {e}")))?;
    Ok(result)
}

#[derive(Debug, thiserror::Error)]
pub enum PolishError {
    #[error("config error: {0}")]
    Config(String),
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("unexpected API response: {0}")]
    BadResponse(String),
    #[error("all {0} attempts failed; last error: {1}")]
    RetriesExhausted(u32, String),
}

/// Transport abstraction so tests can fake the endpoint without HTTP.
#[async_trait::async_trait]
pub trait Transport: Send + Sync {
    async fn post_chat(&self, cfg: &PolishConfig, body: serde_json::Value) -> Result<serde_json::Value, PolishError>;
}

pub struct HttpTransport;

#[async_trait::async_trait]
impl Transport for HttpTransport {
    async fn post_chat(&self, cfg: &PolishConfig, body: serde_json::Value) -> Result<serde_json::Value, PolishError> {
        if cfg.base_url.is_empty() || cfg.model.is_empty() {
            return Err(PolishError::Config("base_url and model must be configured".into()));
        }
        let url = format!("{}/chat/completions", cfg.base_url.trim_end_matches('/'));
        let mut req = reqwest::Client::new().post(url).json(&body);
        if let Some(key) = &cfg.api_key {
            req = req.bearer_auth(key);
        }
        let resp = req.send().await?.error_for_status()?;
        Ok(resp.json().await?)
    }
}

/// The polish step. Retries transient failures; after exhausting retries the
/// caller keeps machine translations (graceful degradation, never blocks a run).
pub struct Polisher<T: Transport> {
    cfg: PolishConfig,
    transport: T,
}

impl Polisher<HttpTransport> {
    pub fn new(cfg: PolishConfig) -> Self {
        Self { cfg, transport: HttpTransport }
    }
}

impl<T: Transport> Polisher<T> {
    pub fn with_transport(cfg: PolishConfig, transport: T) -> Self {
        Self { cfg, transport }
    }

    pub async fn polish_chapter(&self, ctx: &ChapterContext) -> Result<PolishResult, PolishError> {
        if ctx.bubbles.is_empty() {
            return Ok(PolishResult { analysis: String::new(), items: vec![] });
        }
        let body = build_request_body(ctx, &self.cfg);
        let attempts = self.cfg.max_retries + 1;
        let mut last_err: Option<PolishError> = None;
        for attempt in 0..attempts {
            if attempt > 0 {
                tokio::time::sleep(std::time::Duration::from_millis(200 * u64::from(attempt))).await;
            }
            match self.transport.post_chat(&self.cfg, body.clone()).await {
                Ok(resp) => return parse_response(&resp),
                Err(e @ PolishError::Config(_)) => return Err(e),
                Err(e) => {
                    tracing::warn!(attempt, error = %e, "polish attempt failed");
                    last_err = Some(e);
                }
            }
        }
        Err(PolishError::RetriesExhausted(attempts, last_err.map(|e| e.to_string()).unwrap_or_default()))
    }
}

