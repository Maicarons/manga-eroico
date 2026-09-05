//! M2 acceptance test — mirrors the development-plan §10 criterion:
//! "创建工程 → 整页跑管线出译图 → 保存重开"（当前为 mock 推理档位）。
//!
//! Covers the full loop without the GUI:
//! 1. create a `.mepro` project, import a page, group a chapter, set glossary
//! 2. run the six-node pipeline via pipeline-core's Engine with mock steps
//!    that persist artifacts per node (the same wiring src-tauri uses)
//! 3. assert the event stream (Running/Completed per enabled node, Skipped
//!    for the default-off polish node)
//! 4. save → reopen → verify artifacts and history survived the roundtrip

use me_project::{Lang, Project};
use pipeline_core::engine::{Engine, PageId, PipelineEvent, Step};
use pipeline_core::graph::StepKind;
use std::sync::Arc;

struct ArtifactStep {
    kind: StepKind,
    project: Arc<Project>,
    payload: &'static str,
}

impl Step for ArtifactStep {
    fn kind(&self) -> StepKind {
        self.kind
    }
    fn run(
        &self,
        page: &PageId,
        progress: &(dyn Fn(u8) + Send + Sync),
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        progress(50);
        self.project.put_artifact(self.kind.as_str(), &page.0, self.payload.as_bytes(), "json")?;
        progress(100);
        Ok(())
    }
}

fn mock_steps(project: Arc<Project>) -> Vec<Box<dyn Step>> {
    [
        StepKind::Detect,
        StepKind::Ocr,
        StepKind::Inpaint,
        StepKind::Translate,
        StepKind::Polish,
        StepKind::Render,
    ]
    .into_iter()
    .map(|kind| {
        let payload = match kind {
            StepKind::Ocr => r#"{"text":"テスト"}"#,
            StepKind::Translate => r#"{"text":"测试译文"}"#,
            StepKind::Render => r#"{"plan":"typeset"}"#,
            _ => r#"{"ok":true}"#,
        };
        Box::new(ArtifactStep { kind, project: project.clone(), payload }) as Box<dyn Step>
    })
    .collect()
}

#[tokio::test]
async fn m2_create_run_save_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("M2.mepro");

    // 1. create project + import page + chapter + glossary
    let project = Arc::new(Project::create(&root, "M2验收", Lang::Ja, Lang::Zh).unwrap());
    let mut owned = Project::open(&root).unwrap();
    let page_id = owned.add_page("001.jpg", 1100, 1600);
    owned.add_chapter("第1话", vec![page_id.clone()]);
    owned.set_glossary_term("主人公", "主角");
    owned.log_operation("import", &serde_json::json!({ "pages": 1 })).unwrap();
    owned.save().unwrap();
    drop(owned);

    // 2. run the pipeline for the page (graph mirrored from the project file)
    let reopened_for_graph = Project::open(&root).unwrap();
    let graph = reopened_for_graph.file().pipeline.clone();
    drop(reopened_for_graph);
    let engine = Engine::new(graph, mock_steps(project.clone()));
    let (tx, mut rx) = tokio::sync::mpsc::channel::<PipelineEvent>(64);
    let ok = engine.run_page(&PageId(page_id.clone()), tx).await.unwrap();
    assert!(ok, "pipeline run should succeed");

    // 3. event stream: every non-polish node Running→Completed; polish Skipped
    let mut events = Vec::new();
    while let Ok(ev) = rx.try_recv() {
        events.push(ev);
    }
    for kind in [StepKind::Detect, StepKind::Ocr, StepKind::Inpaint, StepKind::Translate, StepKind::Render] {
        assert!(
            events.iter().any(|e| e.step == kind && e.status == pipeline_core::engine::StepStatus::Completed),
            "missing Completed for {kind:?}"
        );
    }
    assert!(
        events.iter().any(|e| e.step == StepKind::Polish && e.status == pipeline_core::engine::StepStatus::Skipped),
        "polish should be skipped while disabled"
    );

    // 4. save/reopen roundtrip: artifacts + history survive
    let final_project = Project::open(&root).unwrap();
    let (_, ocr_bytes) = final_project.latest_artifact("ocr", &page_id).unwrap().unwrap();
    assert_eq!(ocr_bytes, r#"{"text":"テスト"}"#.as_bytes());
    let (_, tr_bytes) = final_project.latest_artifact("translate", &page_id).unwrap().unwrap();
    assert_eq!(tr_bytes, r#"{"text":"测试译文"}"#.as_bytes());
    let (_, render_bytes) = final_project.latest_artifact("render", &page_id).unwrap().unwrap();
    assert_eq!(render_bytes, r#"{"plan":"typeset"}"#.as_bytes());
    assert_eq!(final_project.file().chapters.len(), 1);
    assert_eq!(final_project.file().glossary.get("主人公").unwrap(), "主角");
    let history = std::fs::read_to_string(root.join("history/log.jsonl")).unwrap();
    assert!(history.contains("\"op\":\"import\""));
}
