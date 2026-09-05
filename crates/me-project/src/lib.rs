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
pub enum Lang {
    Zh,
    #[serde(rename = "zh-Hant")]
    ZhHant,
    Yue,
    En,
    Ja,
    Ko,
    Fr,
    Pt,
    Es,
    It,
    De,
    Nl,
    Pl,
    Cs,
    Ru,
    Uk,
    Ar,
    He,
    Fa,
    Tr,
    Th,
    Vi,
    Id,
    Ms,
    Tl,
    Hi,
    Bn,
    Ta,
    Te,
    Mr,
    Gu,
    Ur,
    Km,
    My,
    Bo,
    Kk,
    Mn,
    Ug,
    /// Anything outside the Hy-MT2 language table (free ISO code).
    Other(String),
}

impl Lang {
    /// ISO-ish code used in project files and the UI.
    pub fn code(&self) -> &str {
        match self {
            Lang::Zh => "zh",
            Lang::ZhHant => "zh-Hant",
            Lang::Yue => "yue",
            Lang::En => "en",
            Lang::Ja => "ja",
            Lang::Ko => "ko",
            Lang::Fr => "fr",
            Lang::Pt => "pt",
            Lang::Es => "es",
            Lang::It => "it",
            Lang::De => "de",
            Lang::Nl => "nl",
            Lang::Pl => "pl",
            Lang::Cs => "cs",
            Lang::Ru => "ru",
            Lang::Uk => "uk",
            Lang::Ar => "ar",
            Lang::He => "he",
            Lang::Fa => "fa",
            Lang::Tr => "tr",
            Lang::Th => "th",
            Lang::Vi => "vi",
            Lang::Id => "id",
            Lang::Ms => "ms",
            Lang::Tl => "tl",
            Lang::Hi => "hi",
            Lang::Bn => "bn",
            Lang::Ta => "ta",
            Lang::Te => "te",
            Lang::Mr => "mr",
            Lang::Gu => "gu",
            Lang::Ur => "ur",
            Lang::Km => "km",
            Lang::My => "my",
            Lang::Bo => "bo",
            Lang::Kk => "kk",
            Lang::Mn => "mn",
            Lang::Ug => "ug",
            Lang::Other(c) => c,
        }
    }

    /// Parses an ISO-ish code; unknown codes become `Lang::Other`.
    pub fn from_code(code: &str) -> Self {
        let known = [
            ("zh", Lang::Zh), ("zh-hant", Lang::ZhHant), ("zh-tw", Lang::ZhHant),
            ("zh-hk", Lang::ZhHant), ("yue", Lang::Yue), ("en", Lang::En),
            ("ja", Lang::Ja), ("ko", Lang::Ko), ("fr", Lang::Fr), ("pt", Lang::Pt),
            ("es", Lang::Es), ("it", Lang::It), ("de", Lang::De), ("nl", Lang::Nl),
            ("pl", Lang::Pl), ("cs", Lang::Cs), ("ru", Lang::Ru), ("uk", Lang::Uk),
            ("ar", Lang::Ar), ("he", Lang::He), ("fa", Lang::Fa), ("tr", Lang::Tr),
            ("th", Lang::Th), ("vi", Lang::Vi), ("id", Lang::Id), ("ms", Lang::Ms),
            ("tl", Lang::Tl), ("fil", Lang::Tl), ("hi", Lang::Hi), ("bn", Lang::Bn),
            ("ta", Lang::Ta), ("te", Lang::Te), ("mr", Lang::Mr), ("gu", Lang::Gu),
            ("ur", Lang::Ur), ("km", Lang::Km), ("my", Lang::My), ("bo", Lang::Bo),
            ("kk", Lang::Kk), ("mn", Lang::Mn), ("ug", Lang::Ug),
        ];
        let lower = code.trim().to_ascii_lowercase();
        known
            .iter()
            .find(|(c, _)| *c == lower)
            .map(|(_, l)| l.clone())
            .unwrap_or_else(|| Lang::Other(code.trim().to_string()))
    }

    /// Every Hy-MT2-supported language (translation capability).
    pub fn all_supported() -> &'static [Lang] {
        &[
            Lang::Zh, Lang::ZhHant, Lang::Yue, Lang::En, Lang::Ja, Lang::Ko,
            Lang::Fr, Lang::Pt, Lang::Es, Lang::It, Lang::De, Lang::Nl,
            Lang::Pl, Lang::Cs, Lang::Ru, Lang::Uk, Lang::Ar, Lang::He,
            Lang::Fa, Lang::Tr, Lang::Th, Lang::Vi, Lang::Id, Lang::Ms,
            Lang::Tl, Lang::Hi, Lang::Bn, Lang::Ta, Lang::Te, Lang::Mr,
            Lang::Gu, Lang::Ur, Lang::Km, Lang::My, Lang::Bo, Lang::Kk,
            Lang::Mn, Lang::Ug,
        ]
    }
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
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.root.join("history").join("log.jsonl"))?;
        let record = serde_json::json!({ "ts": Utc::now().to_rfc3339(), "op": op, "detail": detail });
        writeln!(f, "{record}")?;
        Ok(())
    }
}

fn next_version(dir: &Path) -> Result<u32, ProjectError> {
    Ok(list_versions(dir)?.into_iter().max().unwrap_or(0) + 1)
}

fn list_versions(dir: &Path) -> Result<Vec<u32>, ProjectError> {
    let mut out = vec![];
    if dir.exists() {
        for entry in std::fs::read_dir(dir)? {
            let name = entry?.file_name().to_string_lossy().to_string();
            if let Some(v) = name.strip_prefix('v').and_then(|s| s.split('.').next()) {
                if let Ok(n) = v.parse::<u32>() {
                    out.push(n);
                }
            }
        }
    }
    Ok(out)
}

/// Placeholder migration chain: v1 is current; future versions add steps here.
fn migrate(_project: &mut Project) {}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp() -> tempfile::TempDir {
        tempfile::tempdir().unwrap()
    }

    #[test]
    fn create_open_save_roundtrip() {
        let dir = tmp();
        let root = dir.path().join("Test.mepro");
        {
            let mut p = Project::create(&root, "Test", Lang::Ja, Lang::Zh).unwrap();
            assert!(root.join("pages").exists());
            assert!(root.join("artifacts").exists());
            let pid = p.add_page("001.jpg", 1100, 1600);
            let cid = p.add_chapter("Ch1", vec![pid.clone()]);
            p.set_glossary_term("protagonist", "主角");
            p.log_operation("import", &serde_json::json!({ "pages": 1 })).unwrap();
            p.save().unwrap();
            assert_eq!(p.page(&pid).unwrap().width, 1100);
            assert_eq!(p.file.chapters[0].id, cid);
        }
        let p = Project::open(&root).unwrap();
        assert_eq!(p.file().name, "Test");
        assert_eq!(p.file().schema_version, CURRENT_SCHEMA_VERSION);
        assert_eq!(p.file().pages.len(), 1);
        assert_eq!(p.file().glossary.get("protagonist").unwrap(), "主角");
        assert!(!p.file().pipeline.is_enabled(pipeline_core::graph::StepKind::Polish));
        assert!(p.file().updated_at >= p.file().created_at);
    }

    #[test]
    fn open_rejects_non_project() {
        let dir = tmp();
        let err = Project::open(dir.path()).unwrap_err();
        assert!(matches!(err, ProjectError::NotAProject(_)));
    }

    #[test]
    fn open_rejects_newer_schema() {
        let dir = tmp();
        let root = dir.path().join("Fut.mepro");
        Project::create(&root, "F", Lang::Ja, Lang::En).unwrap();
        let mut file: ProjectFile =
            serde_json::from_slice(&std::fs::read(root.join("project.json")).unwrap()).unwrap();
        file.schema_version = 999;
        std::fs::write(root.join("project.json"), serde_json::to_vec(&file).unwrap()).unwrap();
        let err = Project::open(&root).unwrap_err();
        assert!(matches!(err, ProjectError::SchemaTooNew { .. }));
    }

    #[test]
    fn artifact_versions_increment_and_latest_wins() {
        let dir = tmp();
        let root = dir.path().join("A.mepro");
        let mut p = Project::create(&root, "A", Lang::Ja, Lang::Ko).unwrap();
        let pid = p.add_page("p.jpg", 100, 100);
        let v1 = p.put_artifact("ocr", &pid, b"{\"text\":\"v1\"}", "json").unwrap();
        let v2 = p.put_artifact("ocr", &pid, b"{\"text\":\"v2\"}", "json").unwrap();
        assert_eq!((v1, v2), (1, 2));
        let (v, bytes) = p.latest_artifact("ocr", &pid).unwrap().unwrap();
        assert_eq!(v, 2);
        assert_eq!(bytes, b"{\"text\":\"v2\"}");
        assert!(p.latest_artifact("render", &pid).unwrap().is_none());
    }

    #[test]
    fn history_is_append_only() {
        let dir = tmp();
        let root = dir.path().join("H.mepro");
        let p = Project::create(&root, "H", Lang::Ja, Lang::Zh).unwrap();
        p.log_operation("a", &serde_json::json!(1)).unwrap();
        p.log_operation("b", &serde_json::json!(2)).unwrap();
        let log = std::fs::read_to_string(root.join("history/log.jsonl")).unwrap();
        let lines: Vec<&str> = log.trim().lines().collect();
        assert_eq!(lines.len(), 2);
        let rec: serde_json::Value = serde_json::from_str(lines[1]).unwrap();
        assert_eq!(rec["op"], "b");
    }
}
