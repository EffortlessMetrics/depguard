use crate::RepoPath;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use time::OffsetDateTime;

/// Stable schema identifiers for depguard reports.
pub const SCHEMA_REPORT_V1: &str = "depguard.report.v1";
pub const SCHEMA_REPORT_V2: &str = "depguard.report.v2";
pub const SCHEMA_SENSOR_REPORT_V1: &str = "sensor.report.v1";

/// Severity is intentionally small: it maps cleanly to CI signals.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Info,
    Warning,
    Error,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Location {
    pub path: RepoPath,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub col: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Finding {
    pub severity: Severity,
    pub check_id: String,
    pub code: String,
    pub message: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<Location>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub help: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,

    /// Stable identifier intended for dedup and trending. Typically a hash of:
    /// `check_id + code + canonical_path + (line?) + salient fields`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fingerprint: Option<String>,

    /// Check-specific structured payload (kept open-ended for forward compatibility).
    #[serde(default, skip_serializing_if = "serde_json::Value::is_null")]
    pub data: JsonValue,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Verdict {
    Pass,
    Warn,
    Fail,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ToolMeta {
    pub name: String,
    pub version: String,
}

// ============================================================================
// Receipt v2 (cockpit envelope aligned)
// ============================================================================

/// Severity used in v2 receipts ("warn" instead of "warning").
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum SeverityV2 {
    Info,
    Warn,
    Error,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum VerdictStatus {
    Pass,
    Warn,
    Fail,
    Skip,
}

/// Helper function for skip_serializing_if on suppressed field.
fn is_zero(val: &u32) -> bool {
    *val == 0
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct VerdictCounts {
    pub info: u32,
    pub warn: u32,
    pub error: u32,
    /// Count of findings suppressed by baseline filtering.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub suppressed: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct VerdictV2 {
    pub status: VerdictStatus,
    pub counts: VerdictCounts,
    #[serde(default)]
    pub reasons: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ToolMetaV2 {
    pub name: String,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RunHost {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub os: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hostname: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RunCi {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RunGit {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repo: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub head_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_sha: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub head_sha: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub merge_base: Option<String>,
}

// ============================================================================
// Capability reporting for No Green By Omission
// ============================================================================

/// Status of a capability (available, missing, or degraded).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum CapabilityAvailability {
    Available,
    Missing,
    Degraded,
}

/// Status of a single capability with optional reason.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct CapabilityStatus {
    pub status: CapabilityAvailability,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// Capabilities block for No Green By Omission reporting.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Capabilities {
    /// Git integration status (for diff scope, blame, etc.).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git: Option<CapabilityStatus>,
    /// Configuration file status.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<CapabilityStatus>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct RunMeta {
    #[schemars(with = "String")]
    #[serde(with = "time::serde::rfc3339")]
    pub started_at: OffsetDateTime,
    #[schemars(with = "Option<String>")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(with = "time::serde::rfc3339::option")]
    pub ended_at: Option<OffsetDateTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host: Option<RunHost>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ci: Option<RunCi>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git: Option<RunGit>,
    /// Capability status for No Green By Omission reporting.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<Capabilities>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct FindingV2 {
    pub severity: SeverityV2,
    pub check_id: String,
    pub code: String,
    pub message: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<Location>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub help: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub fingerprint: Option<String>,

    #[serde(default, skip_serializing_if = "serde_json::Value::is_null")]
    pub data: JsonValue,
}

// ============================================================================
// Artifact pointers
// ============================================================================

/// Type classification for artifact pointers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ArtifactType {
    Comment,
    Annotation,
    Extra,
}

/// Pointer to an additional artifact produced by a sensor run.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ArtifactPointer {
    /// Type classification for this artifact.
    #[serde(rename = "type")]
    pub artifact_type: ArtifactType,
    /// Path to the artifact file, relative to artifacts directory.
    pub path: String,
    /// MIME type or format identifier (e.g., "text/markdown").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
}

// ============================================================================
// Depguard-specific data
// ============================================================================

/// Depguard-specific summary payload for the report.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
pub struct DepguardData {
    pub scope: String,
    pub profile: String,

    pub manifests_scanned: u32,
    pub dependencies_scanned: u32,

    pub findings_total: u32,
    pub findings_emitted: u32,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub truncated_reason: Option<String>,
}

/// A generic receipt/envelope.
///
/// Keeping this generic allows Depguard to embed tool-specific data while still enforcing a stable outer shape.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ReportEnvelope<TData = DepguardData> {
    /// Versioned schema identifier for the envelope shape.
    pub schema: String,
    pub tool: ToolMeta,
    #[schemars(with = "String")]
    #[serde(with = "time::serde::rfc3339")]
    pub started_at: OffsetDateTime,
    #[schemars(with = "String")]
    #[serde(with = "time::serde::rfc3339")]
    pub finished_at: OffsetDateTime,
    pub verdict: Verdict,
    pub findings: Vec<Finding>,
    pub data: TData,
}

/// V1 report (legacy envelope).
pub type DepguardReportV1 = ReportEnvelope<DepguardData>;

/// V2 report (cockpit-aligned envelope).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ReportEnvelopeV2<TData = DepguardData> {
    pub schema: String,
    pub tool: ToolMetaV2,
    pub run: RunMeta,
    pub verdict: VerdictV2,
    pub findings: Vec<FindingV2>,
    /// Optional list of additional artifacts produced by this run.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifacts: Option<Vec<ArtifactPointer>>,
    pub data: TData,
}

pub type DepguardReportV2 = ReportEnvelopeV2<DepguardData>;

// Back-compat alias (v1).
pub type DepguardReport = DepguardReportV1;
