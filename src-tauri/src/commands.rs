//! Tauri IPC commands. Thin layer: logic lives in the crates.

use crate::state::AppState;
use me_project::{Lang, Project};
use me_polish::{ChapterContext, Polisher};
use model_manager::{hardware::HardwareInfo, registry::Registry, Tier};
use pipeline_core::engine::{Engine, PageId, PipelineEvent, Step};
use pipeline_core::graph::{PipelineGraph, StepKind};
use serde::Serialize;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, State};
use me_detect::DetectProvider as _;
use me_ocr::OcrProvider as _;
use me_inpaint::InpaintProvider as _;
use me_translate::TranslateProvider as _;
use me_render::me_render_provider::RenderProvider as _;

// ---------- meta ----------

#[tauri::command]
pub fn greet(name: &str) -> String {
    format!("Hello, {name}! Welcome to manga-eroico.")
}

// ---------- hardware & models ----------

#[derive(Serialize)]
pub struct HardwareReport {
    pub info: HardwareInfo,
    pub tier: Tier,
}

#[tauri::command]
pub fn get_hardware_info() -> HardwareReport {
    let info = model_manager::hardware::probe();
    let tier = info.decide_tier();
    HardwareReport { info, tier }
}

#[tauri::command]
pub fn decide_tier(info: HardwareInfo) -> Tier {
    info.decide_tier()
}

#[tauri::command]
pub fn list_models() -> Vec<model_manager::ModelSpec> {
    Registry::all()
}

#[tauri::command]
pub fn get_llm_for_tier(tier: Tier) -> String {
    Registry::llm_for_tier(tier).to_string()
}

/// Downloads a model from ModelScope into the app data dir, streaming progress
/// events on the `model-download` channel.
#[tauri::command]
pub async fn download_model(app: AppHandle, spec_id: String, dest_dir: String) -> Result<String, String> {
    let spec = Registry::find(&spec_id).ok_or_else(|| format!("unknown model {spec_id}"))?;
    // ModelScope raw-file endpoint.
    let url = format!(
        "https://modelscope.cn/models/{}/resolve/master/{}",
        spec.modelscope_repo, spec.file
    );
    let dest = PathBuf::from(dest_dir).join(spec.file);
    let client = reqwest::Client::builder().user_agent("manga-eroico/0.1").build().map_err(|e| e.to_string())?;
    model_manager::download_file(&client, &url, dest.clone(), &|downloaded, total| {
        if let Some(t) = total {
            let pct = ((downloaded as f64 / t as f64) * 100.0).min(100.0) as u8;
            let _ = app.emit("model-download", serde_json::json!({
                "id": spec_id, "downloaded": downloaded, "total": t, "percent": pct,
            }));
        }
    })
    .await
    .map_err(|e| e.to_string())?;
    // Verify when pinned; fail loudly on corruption.
    model_manager::verify_sha256(&dest, spec.sha256).map_err(|e| e.to_string())?;
    Ok(dest.to_string_lossy().into_owned())
}

// ---------- project management ----------

fn lang_from_str(s: &str) -> Lang {
    match s {
        "zh" => Lang::Zh,
        "en" => Lang::En,
        "ja" => Lang::Ja,
        "ko" => Lang::Ko,
        other => Lang::Other(other.to_string()),
    }
}

#[tauri::command]
pub async fn create_project(
    state: State<'_, AppState>,
    root: String,
    name: String,
    source_lang: String,
    target_lang: String,
) -> Result<me_project::ProjectFile, String> {
    let project = Project::create(PathBuf::from(&root), &name, lang_from_str(&source_lang), lang_from_str(&target_lang))
        .map_err(|e| e.to_string())?;
    *state.open_project.lock() = Some(PathBuf::from(&root));
    Ok(project.file().clone())
}

