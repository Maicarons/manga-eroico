//! Static description of the six-step manga translation pipeline.

use serde::{Deserialize, Serialize};

/// The six steps of the manga-eroico pipeline, in execution order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepKind {
    Detect,
    Ocr,
    Inpaint,
    Translate,
    Polish,
    Render,
}

pub const ALL_STEPS: [StepKind; 6] = [
    StepKind::Detect,
    StepKind::Ocr,
    StepKind::Inpaint,
    StepKind::Translate,
    StepKind::Polish,
    StepKind::Render,
];

impl StepKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            StepKind::Detect => "detect",
            StepKind::Ocr => "ocr",
            StepKind::Inpaint => "inpaint",
            StepKind::Translate => "translate",
            StepKind::Polish => "polish",
            StepKind::Render => "render",
        }
    }

    /// The step that precedes this one in the default chain, if any.
    pub fn previous(&self) -> Option<StepKind> {
        match self {
            StepKind::Detect => None,
            StepKind::Ocr => Some(StepKind::Detect),
            StepKind::Inpaint => Some(StepKind::Ocr),
            StepKind::Translate => Some(StepKind::Inpaint),
            StepKind::Polish => Some(StepKind::Translate),
            StepKind::Render => Some(StepKind::Polish),
        }
    }
}

/// User-controlled configuration for a single node on the workflow canvas.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    pub kind: StepKind,
    /// Disabled nodes are skipped: their input passes through unchanged.
    pub enabled: bool,
    /// Max automatic retries before the step is marked failed.
    pub max_retries: u32,
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            kind: StepKind::Detect,
            enabled: true,
            max_retries: 2,
        }
    }
}

/// The pipeline graph. Currently a linear chain (the default visual layout);
/// the kind order is authoritative, `configs` may enable/disable any node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineGraph {
    pub configs: Vec<NodeConfig>,
}

impl PipelineGraph {
    /// The default six-node pipeline with every node enabled except polish
    /// (polish requires an external OpenAI-compatible endpoint, so it is
    /// opt-in, matching the product spec).
    pub fn default_pipeline() -> Self {
        Self {
            configs: ALL_STEPS
                .map(|kind| NodeConfig {
                    kind,
                    enabled: !matches!(kind, StepKind::Polish),
