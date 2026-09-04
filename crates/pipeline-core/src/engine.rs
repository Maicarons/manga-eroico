//! Async pipeline executor with live events, retries and skip semantics.

use crate::graph::{PipelineGraph, StepKind};
use serde::{Deserialize, Serialize};
use std::fmt;
use tokio::sync::mpsc;

/// Identifies a page inside a project.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PageId(pub String);

impl fmt::Display for PageId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepStatus {
    Pending,
    Running,
    Completed,
    Skipped,
    Failed,
}

/// Emitted for every state transition; the UI subscribes via Tauri events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineEvent {
    pub page: PageId,
    pub step: StepKind,
    pub status: StepStatus,
    /// 0..=100 within the current step, if reported.
    pub progress: Option<u8>,
    pub message: Option<String>,
}

impl PipelineEvent {
    fn new(page: &PageId, step: StepKind, status: StepStatus) -> Self {
        Self { page: page.clone(), step, status, progress: None, message: None }
    }

    fn with_message(mut self, msg: impl Into<String>) -> Self {
        self.message = Some(msg.into());
        self
    }
}

/// A single pipeline step implementation. Mock implementations power the
/// CI test suite and the web preview; real model-backed implementations are
/// provided by the `me-*` crates behind their respective features.
pub trait Step: Send + Sync {
    fn kind(&self) -> StepKind;
    fn run(
        &self,
        page: &PageId,
        progress: &(dyn Fn(u8) + Send + Sync),
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
}

/// Executes the configured pipeline for one page, in order.
pub struct Engine {
    graph: PipelineGraph,
    steps: Vec<Box<dyn Step>>,
    retries_wait_ms: u64,
}

impl Engine {
    pub fn new(graph: PipelineGraph, steps: Vec<Box<dyn Step>>) -> Self {
        Self { graph, steps, retries_wait_ms: 10 }
    }

    pub fn set_retry_wait(&mut self, ms: u64) {
        self.retries_wait_ms = ms;
    }

    /// Runs all steps for `page`, streaming [`PipelineEvent`]s into `tx`.
    /// A failed step aborts the page run (after retries) with `Ok(false)`;
    /// only channel errors bubble up as `Err`.
    pub async fn run_page(
        &self,
        page: &PageId,
        tx: mpsc::Sender<PipelineEvent>,
    ) -> Result<bool, mpsc::error::SendError<PipelineEvent>> {
        for cfg in self.graph.steps() {
            let kind = cfg.kind;
            if !cfg.enabled {
                tx.send(PipelineEvent::new(page, kind, StepStatus::Skipped)).await?;
                continue;
            }
            let step = match self.steps.iter().find(|s| s.kind() == kind) {
                Some(s) => s,
