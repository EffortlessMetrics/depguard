//! Rendering utilities for CI surfaces (Markdown, GitHub annotations, etc).

#![forbid(unsafe_code)]

mod gha;
mod markdown;
mod model;

pub use gha::render_github_annotations;
pub use markdown::render_markdown;
pub use model::{
    RenderableData, RenderableFinding, RenderableLocation, RenderableReport, RenderableSeverity,
    RenderableVerdictStatus,
};
