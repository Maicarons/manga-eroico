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

/// Returns the latest translated page for `page_id` as a base64 data URL,
/// or None when the project has no render artifact yet.
#[tauri::command]
pub fn get_translated_page(state: State<'_, AppState>, page_id: String) -> Result<Option<String>, String> {
    let path = state.open_project.lock().clone().ok_or("no project open")?;
    let p = Project::open(&path).map_err(|e| e.to_string())?;
    let Some((_, bytes)) = p.latest_artifact("render", &page_id).map_err(|e| e.to_string())? else {
        return Ok(None);
    };
    if bytes.is_empty() {
        return Ok(None);
    }
    Ok(Some(format!("data:image/png;base64,{}", base64_encode(&bytes))))
}

/// Minimal standard base64 (no external dep needed for this one use).
fn base64_encode(data: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity((data.len() + 2) / 3 * 4);
    for chunk in data.chunks(3) {
        let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        out.push(TABLE[(n >> 18) as usize & 63] as char);
        out.push(TABLE[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 { TABLE[(n >> 6) as usize & 63] as char } else { '=' });
        out.push(if chunk.len() > 2 { TABLE[n as usize & 63] as char } else { '=' });
    }
    out
}

/// Chapter tree + page-by-node completion matrix for the open project.
#[tauri::command]
pub fn get_project_overview(state: State<'_, AppState>) -> Result<Option<serde_json::Value>, String> {
    let path = state.open_project.lock().clone().ok_or("no project open")?;
    let p = Project::open(&path).map_err(|e| e.to_string())?;
    let nodes = ["detect", "ocr", "inpaint", "translate", "render"];
    let chapters: Vec<serde_json::Value> = p
        .file()
        .chapters
        .iter()
        .map(|ch| {
            let pages: Vec<serde_json::Value> = ch
                .page_ids
                .iter()
                .filter_map(|pid| p.page(pid))
                .map(|pg| {
                    let node_status: serde_json::Map<String, serde_json::Value> = nodes
                        .iter()
                        .map(|n| {
                            let done = p.latest_artifact(n, &pg.id).ok().flatten().is_some();
                            (n.to_string(), serde_json::Value::Bool(done))
                        })
                        .collect();
                    serde_json::json!({ "id": pg.id, "file": pg.file_name, "nodes": node_status })
                })
                .collect();
            serde_json::json!({ "title": ch.title, "pages": pages })
        })
        .collect();
    Ok(Some(serde_json::json!({
        "name": p.file().name,
        "nodes": nodes,
        "chapters": chapters,
    })))
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
    .map(|(ok, _)| ok)
}

struct NoopPolish;
impl Step for NoopPolish {
    fn kind(&self) -> StepKind {
        StepKind::Polish
    }
    fn run(&self, _: &PageId, _: &(dyn Fn(u8) + Send + Sync)) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }
}

fn ignore_progress(_: u8) {}

/// Mock-backed pipeline steps (real ONNX/llama backends land behind features).
struct DetectStep;
impl Step for DetectStep {
    fn kind(&self) -> StepKind {
        StepKind::Detect
    }
    fn run(&self, _: &PageId, progress: &(dyn Fn(u8) + Send + Sync)) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        progress(50);
        me_detect::MockDetect.detect(b"mock-png").map(|_| ())
    }
}

struct OcrStep;
impl Step for OcrStep {
    fn kind(&self) -> StepKind {
        StepKind::Ocr
    }
    fn run(&self, _: &PageId, progress: &(dyn Fn(u8) + Send + Sync)) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        progress(50);
        me_ocr::MockOcr.recognize(b"mock-png", me_ocr::OcrLang::Ja).map(|_| ())
    }
}

struct InpaintStep;
impl Step for InpaintStep {
    fn kind(&self) -> StepKind {
        StepKind::Inpaint
    }
    fn run(&self, _: &PageId, progress: &(dyn Fn(u8) + Send + Sync)) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        progress(50);
        let img = image::DynamicImage::new_rgb8(16, 16);
        let mask = image::GrayImage::new(16, 16);
        me_inpaint::MockInpaint
            .inpaint(&img, &mask, me_inpaint::InpaintModel::Aot)
            .map(|_| ())
    }
}

struct TranslateStep;
impl Step for TranslateStep {
    fn kind(&self) -> StepKind {
        StepKind::Translate
    }
    fn run(&self, _: &PageId, progress: &(dyn Fn(u8) + Send + Sync)) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        progress(50);
        let prompt = me_translate::build_prompt("ja", "zh", &Default::default(), &[]);
        me_translate::MockTranslate.translate_batch(&prompt).map(|_| ())
    }
}

struct RenderStep;
impl Step for RenderStep {
    fn kind(&self) -> StepKind {
        StepKind::Render
    }
    fn run(&self, _: &PageId, progress: &(dyn Fn(u8) + Send + Sync)) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        progress(50);
        let input = me_render::LayoutInput {
            text: "mock".into(),
            box_w: 100,
            box_h: 50,
            style: Default::default(),
        };
        let _ = me_render::MockRender.render_plan(&input);
        Ok(())
    }
}

// keep ignore_progress referenced
#[allow(dead_code)]
fn _keep() {
    ignore_progress(0);
}

// ---------- polish ----------

/// Dry-run of the polish node against the configured endpoint; used by the
/// settings page "test connection" button.
#[tauri::command]
pub async fn polish_preview(
    state: State<'_, AppState>,
    bubbles: Vec<me_polish::Bubble>,
) -> Result<me_polish::PolishResult, String> {
    let cfg = state.polish.lock().clone();
    let ctx = ChapterContext {
        chapter_title: "preview".into(),
        source_lang: "ja".into(),
        target_lang: "zh".into(),
        glossary: Default::default(),
        bubbles,
    };
    Polisher::new(cfg).polish_chapter(&ctx).await.map_err(|e| e.to_string())
}
