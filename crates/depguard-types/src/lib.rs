//! Stable DTOs and IDs used across the depguard workspace.
//!
//! This crate is intentionally boring:
//! - data types for the emitted receipt/report
//! - stable string IDs and codes
//! - canonical repo-relative path handling
//! - explain registry for remediation guidance

#![forbid(unsafe_code)]

pub mod explain;
pub mod ids;
pub mod path;
pub mod receipt;

pub use explain::{lookup_explanation, ExamplePair, Explanation};
pub use path::RepoPath;
pub use receipt::{
    ArtifactPointer, ArtifactType, Capabilities, CapabilityAvailability, CapabilityStatus,
    DepguardData, DepguardReport, DepguardReportV1, DepguardReportV2, Finding, FindingV2, Location,
    ReportEnvelope, ReportEnvelopeV2, RunCi, RunGit, RunHost, RunMeta, Severity, SeverityV2,
    ToolMeta, ToolMetaV2, Verdict, VerdictCounts, VerdictStatus, VerdictV2, SCHEMA_REPORT_V1,
    SCHEMA_REPORT_V2, SCHEMA_SENSOR_REPORT_V1,
};
