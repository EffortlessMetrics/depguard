use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// `depguard.toml` schema v1.
///
/// This is a *user-facing* config model: it is intentionally permissive so forward-compat is easy.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct DepguardConfigV1 {
    /// Optional schema string for tooling (`depguard.config.v1`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,

    /// When to fail the check: `error` (default) or `warn`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fail_on: Option<String>,

    /// How many findings to emit before truncating the list.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_findings: Option<u32>,

    /// Map of check_id -> config.
    #[serde(default)]
    pub checks: BTreeMap<String, CheckConfig>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct CheckConfig {
    /// Override preset enable/disable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,

    /// Override preset severity: `info`, `warning`, `error`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub severity: Option<String>,

    /// Generic allowlist patterns (semantics are check-specific).
    #[serde(default)]
    pub allow: Vec<String>,
}
