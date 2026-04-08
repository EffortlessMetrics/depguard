use crate::{Location, ToolMeta};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

/// Stable schema identifier for depguard baseline files.
pub const SCHEMA_BASELINE_V1: &str = "depguard.baseline.v1";

/// A baseline entry for a single known finding.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct BaselineFinding {
    /// Stable finding fingerprint.
    pub fingerprint: String,
    pub check_id: String,
    pub code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<Location>,
}

/// Baseline file used to suppress known findings during gradual rollout.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DepguardBaselineV1 {
    /// Versioned schema identifier for the baseline shape.
    pub schema: String,
    pub tool: ToolMeta,
    #[schemars(with = "String")]
    #[serde(with = "time::serde::rfc3339")]
    pub generated_at: OffsetDateTime,
    /// Fingerprints to suppress. Must be deterministic and unique.
    #[serde(default)]
    pub fingerprints: Vec<String>,
    /// Optional human-readable entries for review and auditing.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub findings: Vec<BaselineFinding>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn baseline_roundtrip() {
        let baseline = DepguardBaselineV1 {
            schema: SCHEMA_BASELINE_V1.to_string(),
            tool: ToolMeta {
                name: "depguard".to_string(),
                version: "0.1.0".to_string(),
            },
            generated_at: OffsetDateTime::UNIX_EPOCH,
            fingerprints: vec!["abc".to_string()],
            findings: vec![BaselineFinding {
                fingerprint: "abc".to_string(),
                check_id: "deps.no_wildcards".to_string(),
                code: "wildcard_version".to_string(),
                location: None,
            }],
        };

        let encoded = serde_json::to_string(&baseline).expect("serialize baseline");
        let decoded: DepguardBaselineV1 =
            serde_json::from_str(&encoded).expect("deserialize baseline");
        assert_eq!(decoded, baseline);
    }
}
