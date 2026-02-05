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

pub use check::{run_check, verdict_exit_code, CheckInput, CheckOutput};
pub use explain::{format_explanation, format_not_found, run_explain, ExplainOutput};
pub use render::{render_annotations, render_markdown};
pub use report::{
    empty_report, parse_report_json, runtime_error_report, serialize_report, to_renderable,
    ReportVariant, ReportVersion,
};
