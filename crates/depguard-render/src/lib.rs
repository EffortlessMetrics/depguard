//! Rendering utilities for CI surfaces (Markdown, GitHub annotations, etc).

#![forbid(unsafe_code)]

mod gha;
mod jsonl;
mod junit;
mod markdown;
mod model;
mod sarif;

pub use gha::render_github_annotations;
pub use jsonl::render_jsonl;
pub use junit::render_junit;
pub use markdown::render_markdown;
pub use model::{
    RenderableData, RenderableFinding, RenderableLocation, RenderableReport, RenderableSeverity,
    RenderableVerdictStatus,
};
pub use sarif::render_sarif;
