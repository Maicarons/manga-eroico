//! pipeline-core: the workflow engine behind manga-eroico's visual node graph.
//!
//! The manga translation pipeline is modeled as an ordered list of [`StepKind`]s
//! (detect -> ocr -> inpaint -> translate -> polish -> render). Each step can be
//! enabled/disabled by the user from the workflow canvas; disabled steps are
//! skipped and their output passes through. Every status change is emitted as a
//! [`PipelineEvent`] so the UI can render live node states and artifacts.

pub mod engine;
pub mod graph;

pub use engine::{Engine, PipelineEvent, StepStatus};
pub use graph::{NodeConfig, PipelineGraph, StepKind, PAGE_ORDER_HINT};
