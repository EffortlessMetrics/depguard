//! Rendering utilities for CI surfaces (Markdown, GitHub annotations, etc).

#![forbid(unsafe_code)]

mod gha;
mod markdown;

pub use gha::render_github_annotations;
pub use markdown::render_markdown;
