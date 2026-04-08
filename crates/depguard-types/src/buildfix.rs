use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Stable schema identifier for buildfix plan envelopes.
pub const SCHEMA_BUILDFIX_PLAN_V1: &str = "buildfix.plan.v1";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct BuildfixPlanV1 {
    pub schema: String,
    pub source: BuildfixSourceReport,
    #[serde(default)]
    pub fixes: Vec<BuildfixFixAction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<BuildfixMetadata>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct BuildfixSourceReport {
    pub tool: String,
    pub report_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub report_schema: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct BuildfixFixAction {
    pub finding_ref: BuildfixFindingRef,
    pub action: BuildfixAction,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<BuildfixConfidence>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requires_review: Option<bool>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct BuildfixFindingRef {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool: Option<String>,
    pub check_id: String,
    pub code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fingerprint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<BuildfixLocation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safety: Option<BuildfixSafety>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preconditions: Option<BuildfixPreconditions>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum BuildfixSafety {
    Safe,
    ReviewRequired,
    Unsafe,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct BuildfixPreconditions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit_sha: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receipt_hash: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct BuildfixLocation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub col: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct BuildfixAction {
    #[serde(rename = "type")]
    pub action_type: BuildfixActionType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<BuildfixActionTarget>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum BuildfixActionType {
    Replace,
    Insert,
    Delete,
    Command,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct BuildfixActionTarget {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_start: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_end: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub col_start: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub col_end: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum BuildfixConfidence {
    High,
    Medium,
    Low,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct BuildfixMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generated_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generator: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dry_run: Option<bool>,
}
