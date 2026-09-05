//! OpenAI-compatible Chat Completions translation backend (LM Studio,
//! Ollama, vLLM, or any cloud endpoint) — always available, `reqwest` based.

use crate::anyhow_lite::Result;
use crate::TranslateProvider;
use std::time::Duration;

pub struct OpenAiCompatTranslate {
    client: reqwest::Client,
    base_url: String,
    model: String,
    api_key: Option<String>,
}

impl OpenAiCompatTranslate {
    /// `base_url` like `http://127.0.0.1:8990/v1` (no trailing slash).
    pub fn new(base_url: &str, model: &str, api_key: Option<String>, timeout_secs: u64) -> Result<Self> {
        let mut builder = reqwest::Client::builder()
            .user_agent("manga-eroico/0.1")
            .timeout(Duration::from_secs(timeout_secs));
        if let Ok(proxy) = std::env::var("ME_NO_PROXY") {
            if proxy == "1" {
                builder = builder.no_proxy();
            }
        }
        Ok(Self {
            client: builder.build()?,
            base_url: base_url.trim_end_matches('/').to_string(),
            model: model.to_string(),
            api_key,
        })
    }

    /// Probes `GET /models` to verify the endpoint is alive.
    pub async fn ping(&self) -> Result<bool> {
        let mut req = self.client.get(format!("{}/models", self.base_url));
        if let Some(k) = &self.api_key {
            req = req.bearer_auth(k);
        }
        Ok(req.send().await?.status().is_success())
    }
}

impl TranslateProvider for OpenAiCompatTranslate {
    fn translate_batch(&self, prompt: &str) -> Result<String> {
        let client = self.client.clone();
        let base = self.base_url.clone();
        let model = self.model.clone();
        let api_key = self.api_key.clone();
        let body = serde_json::json!({
            "model": model,
            "messages": [
                { "role": "system", "content": "You are a professional manga translator. Translate every numbered line and reply with the same [id] markers, nothing else." },
                { "role": "user", "content": prompt }
            ],
            "temperature": 0.3,
            "stream": false
        });
        tokio_block(async move {
            let mut req = client
                .post(format!("{base}/chat/completions"))
                .json(&body);
            if let Some(k) = &api_key {
                req = req.bearer_auth(k);
            }
            let resp = req.send().await?;
            let status = resp.status();
            let json: serde_json::Value = resp.json().await?;
            if !status.is_success() {
                return Err(format!("llm endpoint returned {status}: {json}").into());
            }
            json.pointer("/choices/0/message/content")
                .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .ok_or_else(|| "llm response missing choices[0].message.content".into())
        })
    }
}

/// Runs an async block to completion on a scratch runtime (the provider trait
/// is sync; the Tauri host wraps providers in its own async tasks instead).
fn tokio_block<F: std::future::Future>(fut: F) -> F::Output {
    match tokio::runtime::Handle::try_current() {
        Ok(h) => tokio::task::block_in_place(|| h.block_on(fut)),
        Err(_) => {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("scratch tokio runtime");
            rt.block_on(fut)
        }
    }
}
