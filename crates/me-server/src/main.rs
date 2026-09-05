//! me-server — headless CLI driving the manga-eroico pipeline.
//!
//! Acceptance loop (development-plan §10 M2): create a project → import pages
//! → run the pipeline page-by-page (real ONNX models with `--real`, default)
//! → artifacts persisted in the `.mepro` project → save & reopen.

use clap::{Parser, Subcommand};
use me_detect::DetectProvider as _;
use me_ocr::OcrProvider as _;
use me_translate::TranslateProvider as _;
use me_render::me_render_provider::RenderProvider as _;
use me_polish::Transport as _;
use me_project::{Lang, Project};
use pipeline_core::engine::{Engine, PageId, PipelineEvent, Step, StepStatus};
use pipeline_core::graph::StepKind;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

#[derive(Parser)]
#[command(name = "me-server", version, about = "manga-eroico headless pipeline")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Create a new .mepro project
    Create {
        path: PathBuf,
        #[arg(long, default_value = "Untitled")]
        name: String,
        #[arg(long, default_value = "ja")]
        from: String,
        #[arg(long, default_value = "zh")]
        to: String,
    },
    /// Import an image file into the project as a page
    Import { project: PathBuf, image: PathBuf },
    /// Import every image in a folder, grouping pages into chapters by
    /// filename prefix (e.g. "ch01_001.png" -> chapter "ch01")
    ImportFolder { project: PathBuf, folder: PathBuf },
    /// Group pages into a chapter
    Chapter {
        project: PathBuf,
        title: String,
        /// Page ids or the literal "all"
        #[arg(default_value = "all")]
        pages: String,
    },
    /// Run the pipeline for one page (or --all pages)
    RunPage {
        project: PathBuf,
        /// Page id (required unless --all)
        #[arg(default_value = "")]
        page: String,
        #[arg(long, default_value = "models")]
        models_dir: PathBuf,
        /// OpenAI-compatible endpoint for translation, e.g. http://127.0.0.1:8990/v1
        #[arg(long)]
        llm_url: Option<String>,
        #[arg(long, default_value = "local-model")]
        llm_model: String,
        /// Skip translation if no LLM endpoint is reachable
        #[arg(long, default_value = "true")]
        llm_optional: bool,
        /// OpenAI-compatible endpoint for the polish node (enables it)
        #[arg(long)]
        polish_url: Option<String>,
        /// Polish style: formal | casual | literary | custom instruction
        #[arg(long)]
        polish_style: Option<String>,
        /// TTF/OTF font file for raster output (auto-detected if omitted)
        #[arg(long)]
        font: Option<PathBuf>,
        #[arg(long)]
        all: bool,
    },
    /// Show project status: pages, chapters, per-node artifact versions
    Status { project: PathBuf },
}

fn parse_lang(s: &str) -> Lang {
    match s.to_ascii_lowercase().as_str() {
        "zh" | "zh-cn" => Lang::Zh,
        "en" => Lang::En,
        "ja" => Lang::Ja,
        "ko" => Lang::Ko,
        other => Lang::Other(other.to_string()),
    }
}

fn main() {
    let cli = Cli::parse();
    match run(cli.cmd) {
        Ok(()) => {}
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    }
}

fn run(cmd: Cmd) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match cmd {
        Cmd::Create { path, name, from, to } => {
            let p = Project::create(&path, &name, parse_lang(&from), parse_lang(&to))?;
            println!("created {} ({})", p.file().name, p.root().display());
        }
        Cmd::Import { project, image } => {
            let mut p = Project::open(&project)?;
            let (id, file_name, w, h) = import_image(&mut p, &image)?;
            p.log_operation("import", &serde_json::json!({ "page": id, "file": file_name }))?;
            p.save()?;
            println!("imported page {id} ({file_name}, {w}x{h})");
        }
        Cmd::ImportFolder { project, folder } => {
            let mut p = Project::open(&project)?;
            let mut images: Vec<PathBuf> = std::fs::read_dir(&folder)?
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| {
                    matches!(
                        p.extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase()),
                        Some(ref e) if e == "png" || e == "jpg" || e == "jpeg"
                    )
                })
                .collect();
            images.sort();
            if images.is_empty() {
                return Err(format!("no images in {}", folder.display()).into());
            }
            // group by filename prefix (chars up to the first '_' or '-')
            let mut chapter_order: Vec<String> = Vec::new();
            let mut groups: std::collections::BTreeMap<String, Vec<PathBuf>> = Default::default();
            for img in &images {
                let stem = img.file_stem().and_then(|s| s.to_str()).unwrap_or("page").to_string();
                let prefix: String = stem
                    .split(['_', '-'])
                    .next()
                    .unwrap_or("page")
                    .to_string();
                if !groups.contains_key(&prefix) {
                    chapter_order.push(prefix.clone());
                }
                groups.entry(prefix).or_default().push(img.clone());
            }
            let mut total = 0;
            for prefix in &chapter_order {
                let mut page_ids = Vec::new();
                for img in &groups[prefix] {
                    let (id, _name, _w, _h) = import_image(&mut p, img)?;
                    page_ids.push(id);
                    total += 1;
                }
                let ch = p.add_chapter(prefix, page_ids);
                println!("chapter {ch}: {prefix} ({} pages)", groups[prefix].len());
            }
            p.log_operation("import_folder", &serde_json::json!({ "pages": total, "chapters": chapter_order.len() }))?;
            p.save()?;
            println!("imported {total} pages, {} chapters", chapter_order.len());
        }
        Cmd::Chapter { project, title, pages } => {
            let mut p = Project::open(&project)?;
            let ids: Vec<String> = if pages == "all" {
                p.file().pages.iter().map(|pg| pg.id.clone()).collect()
            } else {
                pages.split(',').map(|s| s.trim().to_string()).collect()
            };
            let id = p.add_chapter(&title, ids);
            p.save()?;
            println!("chapter {id}: {title}");
        }
        Cmd::Status { project } => {
            let p = Project::open(&project)?;
            let f = p.file();
            println!("project: {} (lang {:?} -> {:?})", f.name, f.source_lang, f.target_lang);
            println!("pages: {}", f.pages.len());
            for pg in &f.pages {
                let done: Vec<&str> = ["detect", "ocr", "inpaint", "translate", "render"]
                    .iter()
                    .filter(|n| p.latest_artifact(n, &pg.id).ok().flatten().is_some())
                    .map(|s| *s)
                    .collect();
                println!("  {} {} [{}/5: {}]", pg.id, pg.file_name, done.len(), done.join(","));
            }
            println!("chapters: {}", f.chapters.len());
        }
        Cmd::RunPage { project, page, models_dir, llm_url, llm_model, llm_optional, polish_url, polish_style, font, all } => {
            run_pages(project, page, models_dir, llm_url, llm_model, llm_optional, polish_url, polish_style, font, all)?
        }
    }
    Ok(())
}

// ---------- pipeline execution ----------

/// Shared per-page scratch state flowing between steps.
#[derive(Default)]
struct PageState {
    boxes: Vec<me_detect_shim::DetBox>,
    texts: Vec<String>,
    cleaned: Option<image::RgbImage>,
}

// tiny struct so the mock build path compiles without me-detect's TextBox
mod me_detect_shim {
    #[derive(Clone, Debug, serde::Serialize)]
    pub struct DetBox {
        pub x0: f32,
        pub y0: f32,
        pub x1: f32,
        pub y1: f32,
    }
}

fn run_pages(
    project: PathBuf,
    page: String,
    models_dir: PathBuf,
    llm_url: Option<String>,
    llm_model: String,
    llm_optional: bool,
    polish_url: Option<String>,
    polish_style: Option<String>,
    font_path: Option<PathBuf>,
    all: bool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    #[cfg(not(feature = "real"))]
    {
        let _ = (&models_dir, &llm_url, &llm_model, &llm_optional);
        anyhow_bail("this build has no real inference; rebuild with default features");
    }

    #[cfg(feature = "real")]
    {
        let mut p = Project::open(&project)?;
        let mut graph = p.file().pipeline.clone();
        if polish_url.is_some() {
            graph.set_enabled(StepKind::Polish, true);
        }
        let page_ids: Vec<String> = if all {
            p.file().pages.iter().map(|pg| pg.id.clone()).collect()
        } else {
            if page.is_empty() {
                return Err("provide <PAGE> or --all".into());
            }
            vec![page]
        };
        let page_count = page_ids.len();

        // --- load real providers once ---
        let det = me_detect::real::OnnxDetect::load(
            models_dir.join("ppocrv5_det").join("ch_PP-OCRv5_det_mobile.onnx"),
        )?;
        let ocr = me_ocr::real::OnnxOcr::load(
            models_dir.join("rec_mixed").join("ch_PP-OCRv5_rec_mobile.onnx"),
            models_dir.join("dict_mixed").join("ppocrv5_dict.txt"),
        )?;

        // translation: OpenAI-compatible endpoint if reachable, else echo
        let use_llm = match &llm_url {
            Some(url) => {
                let probe = me_translate::openai::OpenAiCompatTranslate::new(
                    url,
                    &llm_model,
                    std::env::var("ME_LLM_API_KEY").ok(),
                    120,
                )?;
                let alive = tokio::runtime::Handle::try_current()
                    .ok()
                    .and_then(|h| tokio::task::block_in_place(|| h.block_on(probe.ping())).ok())
                    .unwrap_or_else(|| {
                        let rt = tokio::runtime::Builder::new_current_thread()
                            .enable_all()
                            .build()
                            .unwrap();
                        rt.block_on(probe.ping()).unwrap_or(false)
                    });
                if alive {
                    println!("[llm] endpoint alive: {url}");
                    true
                } else if llm_optional {
                    eprintln!("[llm] endpoint unreachable, falling back to echo translation");
                    false
                } else {
                    return Err("llm endpoint unreachable and --llm-optional=false".into());
                }
            }
            None => {
                eprintln!("[llm] no --llm-url given, translation will echo source text");
                false
            }
        };
        let det = std::sync::Arc::new(det);
        let ocr = std::sync::Arc::new(ocr);

        // raster font for the render step
        let font_path = match font_path {
            Some(f) => Some(f),
            None => detect_cjk_font(),
        };
        let font_bytes: std::sync::Arc<Vec<u8>> = match &font_path {
            Some(f) => std::sync::Arc::new(std::fs::read(f)?),
            None => std::sync::Arc::new(Vec::new()),
        };
        if font_path.is_none() {
            eprintln!("[render] no system font found; render step will skip raster output");
        }

        // optional polish node
        let polisher = polish_url.map(|url| {
            std::sync::Arc::new(me_polish::Polisher::new(me_polish::PolishConfig {
                base_url: url,
                model: llm_model.clone(),
                api_key: std::env::var("ME_LLM_API_KEY").ok(),
                temperature: 0.3,
                max_retries: 2,
                style: polish_style.clone(),
            }))
        });
        let translator = std::sync::Arc::new(match (&use_llm, &llm_url) {
            (true, Some(url)) => me_translate::openai::OpenAiCompatTranslate::new(
                url,
                &llm_model,
                std::env::var("ME_LLM_API_KEY").ok(),
                120,
            )?,
            _ => me_translate::openai::OpenAiCompatTranslate::new(
                "http://127.0.0.1:1/v1",
                "echo",
                None,
                1,
            )?, // unreachable; echo path never calls it
        });

        for pid in page_ids {
            let page_meta = p
                .file()
                .pages
                .iter()
                .find(|pg| pg.id == pid)
                .ok_or_else(|| format!("page {pid} not found"))?
                .clone();
            let img_path = p.root().join("pages").join(&page_meta.file_name);
            let png = std::fs::read(&img_path)?;
            let state: Arc<Mutex<PageState>> = Arc::new(Mutex::new(PageState::default()));

            struct Ctx {
                png: Vec<u8>,
                det: std::sync::Arc<me_detect::real::OnnxDetect>,
                ocr: std::sync::Arc<me_ocr::real::OnnxOcr>,
                state: Arc<Mutex<PageState>>,
                project: Arc<Mutex<Project>>,
                use_llm: bool,
                translator: std::sync::Arc<me_translate::openai::OpenAiCompatTranslate>,
                font_bytes: std::sync::Arc<Vec<u8>>,
                polisher: Option<std::sync::Arc<me_polish::Polisher<me_polish::HttpTransport>>>,
            }
            impl Ctx {
                fn put(&self, node: &str, page: &str, bytes: &[u8]) {
                    self.project.lock().unwrap().put_artifact(node, page, bytes, "json").unwrap();
                }
            }

            let project_arc = Arc::new(Mutex::new(Project::open(&project)?));
            let ctx = Arc::new(Ctx {
                png,
                det: det.clone(),
                ocr: ocr.clone(),
                state: state.clone(),
                project: project_arc.clone(),
                use_llm,
                translator: translator.clone(),
                font_bytes: font_bytes.clone(),
                polisher: polisher.clone(),
            });

            struct DetectStep2(Arc<Ctx>);
            impl Step for DetectStep2 {
                fn kind(&self) -> StepKind { StepKind::Detect }
                fn run(&self, page: &PageId, progress: &(dyn Fn(u8) + Send + Sync)) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
                    progress(30);
                    let boxes = self.0.det.detect(&self.0.png)?;
                    let mut st = self.0.state.lock().unwrap();
                    st.boxes = boxes.iter().map(|b| {
                        let (x0, y0, x1, y1) = b.bbox();
                        me_detect_shim::DetBox { x0, y0, x1, y1 }
                    }).collect();
                    drop(st);
                    let bytes = serde_json::to_vec(&boxes)?;
                    self.0.put("detect", &page.0, &bytes);
                    progress(100);
                    Ok(())
                }
            }

            struct OcrStep2(Arc<Ctx>);
            impl Step for OcrStep2 {
                fn kind(&self) -> StepKind { StepKind::Ocr }
                fn run(&self, page: &PageId, progress: &(dyn Fn(u8) + Send + Sync)) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
                    let img = image::load_from_memory(&self.0.png)?;
                    let boxes = self.0.state.lock().unwrap().boxes.clone();
                    let mut texts = Vec::new();
                    let total = boxes.len().max(1);
                    for (i, b) in boxes.iter().enumerate() {
                        let crop = crop_box(&img, b.x0, b.y0, b.x1, b.y1)?;
                        let lines = self.0.ocr.recognize(&crop, me_ocr::OcrLang::Ja)?;
                        texts.push(lines.into_iter().map(|l| l.text).collect::<Vec<_>>().join(""));
                        progress((i as u8 + 1) * 100 / total as u8);
                    }
                    self.0.state.lock().unwrap().texts = texts.clone();
                    let bytes = serde_json::to_vec(&texts)?;
                    self.0.put("ocr", &page.0, &bytes);
                    Ok(())
                }
            }

            struct InpaintStep2(Arc<Ctx>);
            impl Step for InpaintStep2 {
                fn kind(&self) -> StepKind { StepKind::Inpaint }
                fn run(&self, page: &PageId, progress: &(dyn Fn(u8) + Send + Sync)) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
                    progress(30);
                    let boxes = self.0.state.lock().unwrap().boxes.clone();
                    // CPU inpaint: fill each detected text region with the
                    // surrounding background color (flat bubble backgrounds).
                    // AI inpainting (AOT/LaMa) remains behind future features.
                    let mut img = image::load_from_memory(&self.0.png)?.to_rgb8();
                    let rects: Vec<(f32, f32, f32, f32)> = boxes.iter()
                        .map(|b| (b.x0, b.y0, b.x1, b.y1)).collect();
                    me_render::draw::fill_regions(&mut img, &rects, 3);
                    progress(70);
                    self.0.state.lock().unwrap().cleaned = Some(img);
                    let bytes = serde_json::to_vec(&rects)?;
                    self.0.put("inpaint", &page.0, &bytes);
                    Ok(())
                }
            }

            struct TranslateStep2(Arc<Ctx>);
            impl Step for TranslateStep2 {
                fn kind(&self) -> StepKind { StepKind::Translate }
                fn run(&self, page: &PageId, progress: &(dyn Fn(u8) + Send + Sync)) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
                    let texts = self.0.state.lock().unwrap().texts.clone();
                    let prompt = me_translate::build_prompt("ja", "zh", &Default::default(), &[]);
                    let numbered: String = texts.iter().enumerate()
                        .map(|(i, t)| format!("[{}]{}\n", i + 1, t))
                        .collect();
                    let full = format!("{prompt}\n{numbered}");
                    let out = if self.0.use_llm {
                        let mut translated = String::new();
                        for line in full.lines() {
                            if line.starts_with('[') {
                                translated.push_str(line);
                                translated.push('\n');
                            }
                        }
                        self.0.translator.translate_batch(&full)?
                    } else {
                        // echo fallback keeps the loop closed without an LLM
                        texts.iter().enumerate()
                            .map(|(i, t)| format!("[{}] <{}>", i + 1, t))
                            .collect::<Vec<_>>()
                            .join("\n")
                    };
                    let _ = prompt;
                    progress(60);
                    let bytes = serde_json::to_vec(&out)?;
                    self.0.put("translate", &page.0, &bytes);
                    Ok(())
                }
            }

            struct RenderStep2(Arc<Ctx>);
            impl Step for RenderStep2 {
                fn kind(&self) -> StepKind { StepKind::Render }
                fn run(&self, page: &PageId, progress: &(dyn Fn(u8) + Send + Sync)) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
                    progress(30);
                    let (texts, boxes, cleaned) = {
                        let st = self.0.state.lock().unwrap();
                        (st.texts.clone(), st.boxes.clone(), st.cleaned.clone())
                    };
                    let mut img = cleaned.unwrap_or_else(|| image::load_from_memory(&self.0.png).unwrap().to_rgb8());
                    let mut plans = Vec::new();
                    if !self.0.font_bytes.is_empty() {
                        let items: Vec<me_render::draw::DrawItem<'_>> = texts.iter().zip(boxes.iter())
                            .map(|(t, b)| me_render::draw::DrawItem {
                                x: b.x0, y: b.y0, w: (b.x1 - b.x0).max(10.0), h: (b.y1 - b.y0).max(10.0),
                                text: t,
                            })
                            .collect();
                        img = me_render::draw::render_translated_page(img, &items, &self.0.font_bytes, [17, 17, 20])?;
                    }
                    progress(60);
                    // keep the layout plan for the editor (Konva re-typesetting)
                    for (t, b) in texts.iter().zip(boxes.iter()) {
                        plans.push(me_render::LayoutInput {
                            text: t.clone(),
                            box_w: (b.x1 - b.x0).max(10.0) as u32,
                            box_h: (b.y1 - b.y0).max(10.0) as u32,
                            style: Default::default(),
                        });
                    }
                    let bytes = serde_json::to_vec(&plans)?;
                    self.0.put("render", &page.0, &bytes);
                    // the translated page itself
                    let mut png = std::io::Cursor::new(Vec::new());
                    image::DynamicImage::ImageRgb8(img)
                        .write_to(&mut png, image::ImageFormat::Png)?;
                    let pid = &page.0;
                    let dir = self.0.project.lock().unwrap().root().join("artifacts").join("render").join(pid);
                    std::fs::create_dir_all(&dir)?;
                    let n = std::fs::read_dir(&dir)?.filter_map(|e| e.ok())
                        .filter(|e| e.path().extension().map(|x| x == "png").unwrap_or(false)).count();
                    std::fs::write(dir.join(format!("translated_v{n:04}.png")), png.into_inner())?;
                    progress(100);
                    Ok(())
                }
            }

            struct PolishStep2(Arc<Ctx>);
            impl Step for PolishStep2 {
                fn kind(&self) -> StepKind { StepKind::Polish }
                fn run(&self, page: &PageId, progress: &(dyn Fn(u8) + Send + Sync)) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
                    let Some(polisher) = &self.0.polisher else { return Ok(()) };
                    progress(30);
                    let texts = self.0.state.lock().unwrap().texts.clone();
                    let ctx = me_polish::ChapterContext {
                        chapter_title: "page".into(),
                        source_lang: "ja".into(),
                        target_lang: "zh".into(),
                        glossary: Default::default(),
                        bubbles: texts.iter().enumerate()
                            .map(|(i, t)| me_polish::Bubble {
                                id: format!("b{i:03}"),
                                page: 1,
                                position: (i + 1) as u32,
                                source_text: String::new(), // OCR source not carried here
                                machine_translation: t.clone(),
                            })
                            .collect(),
                    };
                    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build()?;
                    let result = rt.block_on(polisher.polish_chapter(&ctx))?;
                    progress(70);
                    let mut updated = texts.clone();
                    for item in &result.items {
                        if let Ok(idx) = item.id.trim_start_matches('b').parse::<usize>() {
                            if idx < updated.len() {
                                updated[idx] = item.polished.clone();
                            }
                        }
                    }
                    self.0.state.lock().unwrap().texts = updated;
                    let _ = page;
                    Ok(())
                }
            }

            let steps: Vec<Box<dyn Step>> = vec![
                Box::new(DetectStep2(ctx.clone())),
                Box::new(OcrStep2(ctx.clone())),
                Box::new(InpaintStep2(ctx.clone())),
                Box::new(TranslateStep2(ctx.clone())),
                Box::new(PolishStep2(ctx.clone())),
                Box::new(RenderStep2(ctx)),
            ];

            let engine = Engine::new(graph.clone(), steps);
            let (tx, mut rx) = tokio::sync::mpsc::channel::<PipelineEvent>(128);
            let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build()?;
            let ok = rt.block_on(async {
                let handle = tokio::spawn(async move {
                    let mut events = Vec::new();
                    while let Some(ev) = rx.recv().await {
                        events.push(ev);
                    }
                    events
                });
                let ok = engine.run_page(&PageId(pid.clone()), tx).await.unwrap_or(false);
                let events = handle.await.unwrap_or_default();
                for ev in &events {
                    if ev.status == StepStatus::Failed {
                        eprintln!("  {:?} failed: {}", ev.step, ev.message.clone().unwrap_or_default());
                    }
                }
                ok
            });
            if !ok {
                return Err(format!("pipeline failed for page {pid}").into());
            }
            println!("page {pid}: pipeline complete");
        }

        // persist glossary/history changes from the run
        let mut final_p = Project::open(&project)?;
        final_p.log_operation("run_pages", &serde_json::json!({ "pages": page_count }))?;
        final_p.save()?;
    }
    Ok(())
}

fn crop_box(img: &image::DynamicImage, x0: f32, y0: f32, x1: f32, y1: f32) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    let (w, h) = (img.width(), img.height());
    let (x0, y0) = (x0.max(0.0) as u32, y0.max(0.0) as u32);
    let (x1, y1) = (x1.min(w as f32) as u32, y1.min(h as f32) as u32);
    let crop = img.crop_imm(x0, y0, x1.saturating_sub(x0).max(1), y1.saturating_sub(y0).max(1));
    let mut buf = std::io::Cursor::new(Vec::new());
    crop.write_to(&mut buf, image::ImageFormat::Png)?;
    Ok(buf.into_inner())
}

fn import_image(p: &mut Project, image: &Path) -> Result<(String, String, u32, u32), Box<dyn std::error::Error + Send + Sync>> {
    let (w, h) = image_dimensions(image)?;
    let hash = content_hash(image)?;
    let ext = image
        .extension()
        .map(|e| e.to_string_lossy().to_string())
        .unwrap_or_else(|| "png".into());
    let file_name = format!("{hash}.{ext}");
    std::fs::copy(image, p.root().join("pages").join(&file_name))?;
    let id = p.add_page(&file_name, w, h);
    Ok((id, file_name, w, h))
}

fn image_dimensions(path: &Path) -> Result<(u32, u32), Box<dyn std::error::Error + Send + Sync>> {
    let img = image::ImageReader::open(path)?.into_dimensions()?;
    Ok(img)
}

fn content_hash(path: &Path) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    use sha2::{Digest, Sha256};
    let bytes = std::fs::read(path)?;
    let digest = Sha256::digest(&bytes);
    Ok(digest.iter().take(8).map(|b| format!("{b:02x}")).collect::<Vec<_>>().join(""))
}

fn anyhow_bail(msg: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    Err(msg.into())
}



/// Best-effort system CJK font detection across platforms.
fn detect_cjk_font() -> Option<PathBuf> {
    let candidates: &[&str] = &[
        "C:\\Windows\\Fonts\\msyh.ttc",
        "C:\\Windows\\Fonts\\msyh.ttf",
        "C:\\Windows\\Fonts\\simhei.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        "/System/Library/Fonts/Hiragino Sans GB.ttc",
        "/System/Library/Fonts/PingFang.ttc",
    ];
    candidates.iter().map(PathBuf::from).find(|p| p.exists())
}
