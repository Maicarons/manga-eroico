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

#[tauri::command]
pub async fn open_project(state: State<'_, AppState>, root: String) -> Result<me_project::ProjectFile, String> {
    let project = Project::open(PathBuf::from(&root)).map_err(|e| e.to_string())?;
    *state.open_project.lock() = Some(PathBuf::from(&root));
    Ok(project.file().clone())
}

#[tauri::command]
pub async fn save_project(state: State<'_, AppState>) -> Result<(), String> {
    let path = state.open_project.lock().clone().ok_or("no project open")?;
    Project::open(path).map_err(|e| e.to_string())?.save().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn add_page(state: State<'_, AppState>, file_name: String, width: u32, height: u32) -> Result<String, String> {
    let path = state.open_project.lock().clone().ok_or("no project open")?;
    let mut p = Project::open(path).map_err(|e| e.to_string())?;
    let id = p.add_page(&file_name, width, height);
    p.log_operation("add_page", &serde_json::json!({ "id": id })).map_err(|e| e.to_string())?;
    p.save().map_err(|e| e.to_string())?;
    Ok(id)
}

#[tauri::command]
pub async fn add_chapter(state: State<'_, AppState>, title: String, page_ids: Vec<String>) -> Result<String, String> {
    let path = state.open_project.lock().clone().ok_or("no project open")?;
    let mut p = Project::open(path).map_err(|e| e.to_string())?;
    let id = p.add_chapter(&title, page_ids);
    p.save().map_err(|e| e::s(&e))?;
    Ok(id)
}

// helper shim (kept tiny)
mod e {
    pub fn s(_: &dyn std::fmt::Debug) -> String {
        "save failed".into()
    }
}

#[tauri::command]
pub async fn set_glossary_term(state: State<'_, AppState>, term: String, translation: String) -> Result<(), String> {
    let path = state.open_project.lock().clone().ok_or("no project open")?;
    let mut p = Project::open(path).map_err(|e| e.to_string())?;
    p.set_glossary_term(&term, &translation);
    p.save().map_err(|e| e.to_string())
}

// ---------- pipeline ----------

#[tauri::command]
pub async fn set_node_enabled(state: State<'_, AppState>, node: String, enabled: bool) -> Result<(), String> {
    let kind = match node.as_str() {
        "detect" => StepKind::Detect,
        "ocr" => StepKind::Ocr,
        "inpaint" => StepKind::Inpaint,
        "translate" => StepKind::Translate,
        "polish" => StepKind::Polish,
        "render" => StepKind::Render,
        _ => return Err(format!("unknown node {node}")),
    };
    let path = state.open_project.lock().clone().ok_or("no project open")?;
    let mut p = Project::open(&path).map_err(|e| e.to_string())?;
    let mut graph = p.file().pipeline.clone();
    if !graph.set_enabled(kind, enabled) {
        return Err(format!("node {node} not in pipeline"));
    }
    p.set_pipeline(graph);
    p.log_operation("set_node_enabled", &serde_json::json!({ "node": node, "enabled": enabled }))
        .map_err(|e| e.to_string())?;
    p.save().map_err(|e| e.to_string())
}

/// Runs the full six-step pipeline (mock providers until the `onnx` /
/// `local-llm` features ship enabled builds) for one page, streaming
/// [`PipelineEvent`]s on the `pipeline-event` channel.
#[tauri::command]
pub async fn run_pipeline_page(
    app: AppHandle,
    state: State<'_, AppState>,
    page_id: String,
) -> Result<bool, String> {
    let path = state.open_project.lock().clone().ok_or("no project open")?;
    let project = Project::open(path).map_err(|e| e.to_string())?;
    let graph: PipelineGraph = project.file().pipeline.clone();

    let steps: Vec<Box<dyn Step>> = vec![
        Box::new(DetectStep),
        Box::new(OcrStep),
        Box::new(InpaintStep),
        Box::new(TranslateStep),
        Box::new(NoopPolish),
        Box::new(RenderStep),
    ];
    let engine = Engine::new(graph, steps);
    let (tx, mut rx) = tokio::sync::mpsc::channel::<PipelineEvent>(256);
    let page = PageId(page_id);

    let runner = engine.run_page(&page, tx);
    let forwarder = async move {
        while let Some(ev) = rx.recv().await {
            let _ = app.emit("pipeline-event", &ev);
        }
    };
    tokio::try_join!(async move { runner.await.map_err(|e| e.to_string()) }, async {
        forwarder.await;
        Ok::<(), String>(())
    })
