use crate::RepoPath;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use time::OffsetDateTime;

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
    pub started_at: OffsetDateTime,
    pub finished_at: OffsetDateTime,
    pub verdict: Verdict,
    pub findings: Vec<Finding>,
    pub data: TData,
}

pub type DepguardReport = ReportEnvelope<DepguardData>;
