//! me-project: every translation job in manga-eroico is a `.mepro` project —
//! a directory on disk that owns its pages, chapters, glossary, per-node
//! artifacts and history. See docs/development-plan.md §3.4.
//!
//! Layout:
//! ```text
//! MyManga.mepro/
//! ├─ project.json     # metadata (this crate's ProjectFile)
//! ├─ pages/           # imported source images (content-addressed names)
//! ├─ artifacts/       # per node x page x version outputs
//! ├─ glossary.json
//! └─ history/         # append-only operation log for rollback
//! ```

use chrono::Utc;
use pipeline_core::graph::PipelineGraph;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

pub const CURRENT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, thiserror::Error)]
pub enum ProjectError {
    #[error("io error: {0}")]
    Io(#[from] io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("not a .mepro project: {0}")]
    NotAProject(PathBuf),
    #[error("unsupported project schema version {found} (supported: <= {supported})")]
    SchemaTooNew { found: u32, supported: u32 },
    #[error("page {0} not found")]
    PageNotFound(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Lang {
    Zh,
    En,
    Ja,
    Ko,
    Other(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page {
    pub id: String,
    pub file_name: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chapter {
    pub id: String,
    pub title: String,
    /// Page ids in reading order.
    pub page_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectFile {
    pub schema_version: u32,
    pub name: String,
    pub source_lang: Lang,
    pub target_lang: Lang,
    pub created_at: String,
    pub updated_at: String,
    /// Node enable/retry config mirrored from the workflow canvas.
    pub pipeline: PipelineGraph,
    pub pages: Vec<Page>,
    pub chapters: Vec<Chapter>,
    /// term -> translation, injected into translate & polish nodes.
    pub glossary: BTreeMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct Project {
    root: PathBuf,
    file: ProjectFile,
}

impl Project {
    /// Creates a fresh `.mepro` project directory with the canonical layout.
    pub fn create(root: impl Into<PathBuf>, name: &str, source_lang: Lang, target_lang: Lang) -> Result<Self, ProjectError> {
        let root = root.into();
        std::fs::create_dir_all(root.join("pages"))?;
        std::fs::create_dir_all(root.join("artifacts"))?;
        std::fs::create_dir_all(root.join("history"))?;
        let now = Utc::now().to_rfc3339();
        let file = ProjectFile {
            schema_version: CURRENT_SCHEMA_VERSION,
            name: name.to_string(),
            source_lang,
            target_lang,
            created_at: now.clone(),
            updated_at: now,
            pipeline: PipelineGraph::default_pipeline(),
            pages: vec![],
            chapters: vec![],
            glossary: BTreeMap::new(),
        };
        let mut project = Self { root, file };
        project.save()?;
        Ok(project)
    }

    /// Opens an existing project, validating schema compatibility and running
    /// migrations when needed.
    pub fn open(root: impl Into<PathBuf>) -> Result<Self, ProjectError> {
