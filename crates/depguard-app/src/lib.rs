//! Use case orchestration for depguard.
//!
//! This crate provides the application layer: use cases that coordinate the domain, repo, and
//! render layers. It is intentionally thin and delegates heavy lifting to the appropriate layers.
//!
//! The CLI crate depends on this; it only handles argument parsing and I/O.

#![forbid(unsafe_code)]

mod check;
mod explain;
mod render;
mod report;

pub use check::{CheckInput, CheckOutput, run_check, verdict_exit_code};
pub use explain::{ExplainOutput, format_explanation, format_not_found, run_explain};
pub use render::{render_annotations, render_markdown};
pub use report::{
    ReportVariant, ReportVersion, add_artifact, empty_report, parse_report_json,
    runtime_error_report, serialize_report, to_renderable,
};
