//! Stable DTOs and IDs used across the depguard workspace.
//!
//! This crate is intentionally boring:
//! - data types for the emitted receipt/report
//! - stable string IDs and codes
//! - canonical repo-relative path handling

#![forbid(unsafe_code)]

pub mod ids;
pub mod path;
pub mod receipt;

pub use path::RepoPath;
pub use receipt::{DepguardData, DepguardReport, Finding, Location, ReportEnvelope, Severity, ToolMeta, Verdict};
