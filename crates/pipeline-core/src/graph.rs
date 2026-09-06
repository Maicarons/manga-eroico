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
    /// Free-form node parameters (thresholds, model choices, endpoints...).
    /// Each node type documents its own keys; unknown keys are preserved.
    #[serde(default)]
    pub params: std::collections::BTreeMap<String, serde_json::Value>,
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            kind: StepKind::Detect,
            enabled: true,
            max_retries: 2,
            params: Default::default(),
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
                    ..NodeConfig::default()
                })
                .into_iter()
            .collect(),
        }
    }

    pub fn is_enabled(&self, kind: StepKind) -> bool {
        self.configs
            .iter()
            .find(|c| c.kind == kind)
            .map(|c| c.enabled)
            .unwrap_or(true)
    }

    pub fn max_retries(&self, kind: StepKind) -> u32 {
        self.configs
            .iter()
            .find(|c| c.kind == kind)
            .map(|c| c.max_retries)
            .unwrap_or(0)
    }

    /// Steps in execution order.
    pub fn steps(&self) -> impl Iterator<Item = &NodeConfig> {
        self.configs.iter()
    }

    /// Toggles a node from the workflow canvas. Returns `false` when the kind
    /// is not part of the graph.
    /// Sets one node parameter, returning false when the node is absent.
    pub fn set_param(
        &mut self,
        kind: StepKind,
        key: &str,
        value: serde_json::Value,
    ) -> bool {
        match self.configs.iter_mut().find(|c| c.kind == kind) {
            Some(cfg) => {
                cfg.params.insert(key.to_string(), value);
                true
            }
            None => false,
        }
    }

    /// Reads one node parameter (None when node or key is absent).
    pub fn param(&self, kind: StepKind, key: &str) -> Option<&serde_json::Value> {
        self.configs
            .iter()
            .find(|c| c.kind == kind)
            .and_then(|c| c.params.get(key))
    }

    pub fn set_enabled(&mut self, kind: StepKind, enabled: bool) -> bool {
        match self.configs.iter_mut().find(|c| c.kind == kind) {
            Some(c) => {
                c.enabled = enabled;
                true
            }
            None => false,
        }
    }
}

/// Reading order hint for manga pages: Japanese manga reads right-to-left.
pub const PAGE_ORDER_HINT: &str = "pages are ordered by reading direction metadata on the project";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_pipeline_disables_polish_only() {
        let g = PipelineGraph::default_pipeline();
        assert_eq!(g.configs.len(), 6);
        assert!(!g.is_enabled(StepKind::Polish));
        for k in [StepKind::Detect, StepKind::Ocr, StepKind::Inpaint, StepKind::Translate, StepKind::Render] {
            assert!(g.is_enabled(k), "{k:?} should be enabled by default");
        }
    }

    #[test]
    fn toggle_polish_roundtrip() {
        let mut g = PipelineGraph::default_pipeline();
        assert!(g.set_enabled(StepKind::Polish, true));
        assert!(g.is_enabled(StepKind::Polish));
        assert!(g.set_enabled(StepKind::Polish, false));
        assert!(!g.is_enabled(StepKind::Polish));
        // unknown kind must report false
        let mut empty = PipelineGraph { configs: vec![] };
        assert!(!empty.set_enabled(StepKind::Detect, true));
    }

    #[test]
    fn chain_is_contiguous() {
        for k in ALL_STEPS {
            if let Some(prev) = k.previous() {
                assert_eq!(ALL_STEPS[ALL_STEPS.iter().position(|s| *s == prev).unwrap() + 1], k);
            }
        }
    }

    #[test]
    fn serde_roundtrip() {
        let g = PipelineGraph::default_pipeline();
        let json = serde_json::to_string(&g).unwrap();
        let back: PipelineGraph = serde_json::from_str(&json).unwrap();
        assert_eq!(back.configs.len(), 6);
    }
}
