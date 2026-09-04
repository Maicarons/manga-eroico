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
                None => {
                    tx.send(
                        PipelineEvent::new(page, kind, StepStatus::Failed)
                            .with_message(format!("no executor registered for {kind:?}")),
                    )
                    .await?;
                    return Ok(false);
                }
            };

            tx.send(PipelineEvent::new(page, kind, StepStatus::Running)).await?;

            let mut attempt = 0u32;
            let max_retries = cfg.max_retries;
            loop {
                // The progress reporter is a closure borrowing the channel; it
                // only lives for the duration of `step.run` so later awaits
                // below can use `tx.send` freely.
                let result = {
                    let reporter = |pct: u8| {
                        let _ = tx.try_send(PipelineEvent {
                            progress: Some(pct),
                            ..PipelineEvent::new(page, kind, StepStatus::Running)
                        });
                    };
                    step.run(page, &reporter)
                };
                match result {
                    Ok(()) => {
                        tx.send(PipelineEvent::new(page, kind, StepStatus::Completed)).await?;
                        break;
                    }
                    Err(e) if attempt < max_retries => {
                        attempt += 1;
                        tracing::warn!(page = %page, step = kind.as_str(), attempt, error = %e, "step failed, retrying");
                        tokio::time::sleep(std::time::Duration::from_millis(self.retries_wait_ms)).await;
                    }
                    Err(e) => {
                        tx.send(
                            PipelineEvent::new(page, kind, StepStatus::Failed)
                                .with_message(e.to_string()),
                        )
                        .await?;
                        return Ok(false);
                    }
                }
            }
        }
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    struct MockStep {
        kind: StepKind,
        calls: Arc<AtomicU32>,
        fail_times: u32,
    }

    impl Step for MockStep {
        fn kind(&self) -> StepKind {
            self.kind
        }
        fn run(
            &self,
            _page: &PageId,
            progress: &(dyn Fn(u8) + Send + Sync),
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            progress(50);
            if self.calls.load(Ordering::SeqCst) <= self.fail_times {
                return Err("boom".into());
            }
            Ok(())
        }
    }

    fn mock(kind: StepKind, fail_times: u32) -> (Box<dyn Step>, Arc<AtomicU32>) {
        let calls = Arc::new(AtomicU32::new(0));
        (Box::new(MockStep { kind, calls: calls.clone(), fail_times }), calls)
    }

    async fn run(graph: PipelineGraph, steps: Vec<Box<dyn Step>>) -> (bool, Vec<PipelineEvent>) {
        let (tx, mut rx) = mpsc::channel(256);
        let mut engine = Engine::new(graph, steps);
        engine.set_retry_wait(1);
        let page = PageId("p1".into());
        let handle = tokio::spawn(async move { engine.run_page(&page, tx).await });
