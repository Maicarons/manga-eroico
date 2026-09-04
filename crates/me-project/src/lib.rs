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
        let root = root.into();
        let marker = root.join("project.json");
        if !marker.exists() {
            return Err(ProjectError::NotAProject(root));
        }
        let raw = std::fs::read(&marker)?;
        let file: ProjectFile = serde_json::from_slice(&raw)?;
        if file.schema_version > CURRENT_SCHEMA_VERSION {
            return Err(ProjectError::SchemaTooNew {
                found: file.schema_version,
                supported: CURRENT_SCHEMA_VERSION,
            });
        }
        let mut project = Self { root, file };
        migrate(&mut project);
        Ok(project)
    }

    pub fn save(&mut self) -> Result<(), ProjectError> {
        let mut file = self.file.clone();
        file.updated_at = Utc::now().to_rfc3339();
        let bytes = serde_json::to_vec_pretty(&file)?;
        std::fs::write(self.root.join("project.json"), bytes)?;
        self.file = file;
        Ok(())
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn file(&self) -> &ProjectFile {
        &self.file
    }

    // ---- pages ----

    /// Registers an imported image; the physical file must already exist under
    /// `pages/` (the import step copies it there content-addressed).
    pub fn add_page(&mut self, file_name: &str, width: u32, height: u32) -> String {
        let id = format!("pg_{}", uuid::Uuid::new_v4().simple());
        self.file.pages.push(Page { id: id.clone(), file_name: file_name.into(), width, height });
        id
    }

    pub fn page(&self, id: &str) -> Option<&Page> {
        self.file.pages.iter().find(|p| p.id == id)
    }

    // ---- chapters ----

    pub fn add_chapter(&mut self, title: &str, page_ids: Vec<String>) -> String {
        let id = format!("ch_{}", uuid::Uuid::new_v4().simple());
        self.file.chapters.push(Chapter { id: id.clone(), title: title.into(), page_ids });
        id
    }

    // ---- glossary ----

    pub fn set_glossary_term(&mut self, term: &str, translation: &str) {
        self.file.glossary.insert(term.into(), translation.into());
    }

    // ---- pipeline config (workflow canvas mirror) ----

    pub fn set_pipeline(&mut self, graph: PipelineGraph) {
        self.file.pipeline = graph;
    }

    // ---- artifacts (node x page x version) ----

    /// Persists an artifact (JSON or image) for `node`/`page` as a new version
    /// and returns its version number (1-based).
    pub fn put_artifact(&self, node: &str, page_id: &str, bytes: &[u8], ext: &str) -> Result<u32, ProjectError> {
        let dir = self.root.join("artifacts").join(node).join(page_id);
        std::fs::create_dir_all(&dir)?;
        let version = next_version(&dir)?;
        let path = dir.join(format!("v{version:04}.{ext}"));
        std::fs::write(path, bytes)?;
        Ok(version)
    }

    /// Latest artifact bytes for `node`/`page`, if any.
    pub fn latest_artifact(&self, node: &str, page_id: &str) -> Result<Option<(u32, Vec<u8>)>, ProjectError> {
        let dir = self.root.join("artifacts").join(node).join(page_id);
        if !dir.exists() {
            return Ok(None);
        }
        let mut versions: Vec<u32> = list_versions(&dir)?;
        versions.sort_unstable();
        match versions.last() {
            Some(v) => {
                // find the file with that version (any extension)
                for entry in std::fs::read_dir(&dir)? {
                    let entry = entry?;
                    let name = entry.file_name().to_string_lossy().to_string();
                    if let Some(vn) = name.strip_prefix('v').and_then(|s| s.split('.').next()) {
                        if vn.parse::<u32>().ok() == Some(*v) {
                            return Ok(Some((*v, std::fs::read(entry.path())?)));
                        }
                    }
                }
                Ok(None)
            }
            None => Ok(None),
        }
    }

    // ---- history ----

    /// Appends an operation record to `history/log.jsonl` (append-only).
    pub fn log_operation(&self, op: &str, detail: &serde_json::Value) -> Result<(), ProjectError> {
        use std::io::Write;
