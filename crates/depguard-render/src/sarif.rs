use crate::{RenderableReport, RenderableSeverity, RenderableVerdictStatus};
use serde_json::{Map, Value, json};
use std::collections::BTreeMap;

pub fn render_sarif(report: &RenderableReport) -> String {
    let mut rules: BTreeMap<String, Value> = BTreeMap::new();
    let mut results: Vec<Value> = Vec::new();

    for finding in &report.findings {
        let rule_id = finding
            .check_id
            .clone()
            .unwrap_or_else(|| format!("depguard.{}", finding.code));

        rules.entry(rule_id.clone()).or_insert_with(|| {
            let mut props = Map::new();
            props.insert("code".to_string(), json!(finding.code));

            let mut rule = Map::new();
            rule.insert("id".to_string(), json!(rule_id));
            if let Some(check_id) = &finding.check_id {
                rule.insert("name".to_string(), json!(check_id));
            }
            rule.insert(
                "shortDescription".to_string(),
                json!({ "text": finding.code }),
            );
            if let Some(url) = &finding.url {
                rule.insert("helpUri".to_string(), json!(url));
            }
            rule.insert("properties".to_string(), Value::Object(props));
            Value::Object(rule)
        });

        let mut result_props = Map::new();
        result_props.insert("code".to_string(), json!(finding.code));
        if let Some(help) = &finding.help {
            result_props.insert("help".to_string(), json!(help));
        }
        if let Some(check_id) = &finding.check_id {
            result_props.insert("check_id".to_string(), json!(check_id));
        }

        let mut result = Map::new();
        result.insert("ruleId".to_string(), json!(rule_id));
        result.insert(
            "level".to_string(),
            json!(match finding.severity {
                RenderableSeverity::Error => "error",
                RenderableSeverity::Warning => "warning",
                RenderableSeverity::Info => "note",
            }),
        );
        result.insert("message".to_string(), json!({ "text": finding.message }));
        result.insert("properties".to_string(), Value::Object(result_props));

        if let Some(loc) = &finding.location {
            let mut physical_location = Map::new();
            physical_location.insert("artifactLocation".to_string(), json!({ "uri": loc.path }));

            if loc.line.is_some() || loc.col.is_some() {
                let mut region = Map::new();
                if let Some(line) = loc.line {
                    region.insert("startLine".to_string(), json!(line));
                }
                if let Some(col) = loc.col {
                    region.insert("startColumn".to_string(), json!(col));
                }
                physical_location.insert("region".to_string(), Value::Object(region));
            }

            result.insert(
                "locations".to_string(),
                json!([
                    {
                        "physicalLocation": physical_location,
                    }
                ]),
            );
        }

        results.push(Value::Object(result));
    }

    let verdict = match report.verdict {
        RenderableVerdictStatus::Pass => "pass",
        RenderableVerdictStatus::Warn => "warn",
        RenderableVerdictStatus::Fail => "fail",
        RenderableVerdictStatus::Skip => "skip",
    };

    let doc = json!({
        "$schema": "https://json.schemastore.org/sarif-2.1.0.json",
        "version": "2.1.0",
        "runs": [
            {
                "tool": {
                    "driver": {
                        "name": "depguard",
                        "informationUri": "https://github.com/EffortlessMetrics/depguard",
                        "rules": rules.values().cloned().collect::<Vec<_>>(),
                    }
                },
                "results": results,
                "properties": {
                    "depguard_verdict": verdict,
                    "findings_emitted": report.data.findings_emitted,
                    "findings_total": report.data.findings_total,
                }
            }
        ]
    });

    serde_json::to_string_pretty(&doc).unwrap_or_else(|_| "{}".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        RenderableData, RenderableFinding, RenderableLocation, RenderableSeverity,
        RenderableVerdictStatus,
    };

    #[test]
    fn render_sarif_includes_rules_and_results() {
        let report = RenderableReport {
            verdict: RenderableVerdictStatus::Fail,
            findings: vec![RenderableFinding {
                severity: RenderableSeverity::Error,
                check_id: Some("deps.no_wildcards".to_string()),
                code: "wildcard_version".to_string(),
                message: "dependency uses wildcard".to_string(),
                location: Some(RenderableLocation {
                    path: "Cargo.toml".to_string(),
                    line: Some(8),
                    col: Some(1),
                }),
                help: Some("pin the version".to_string()),
                url: Some("https://example.invalid/help".to_string()),
            }],
            data: RenderableData {
                findings_emitted: 1,
                findings_total: 1,
                truncated_reason: None,
            },
        };

        let sarif = render_sarif(&report);
        let doc: Value = serde_json::from_str(&sarif).expect("sarif json");

        assert_eq!(doc["version"], "2.1.0");
        assert_eq!(doc["runs"][0]["tool"]["driver"]["name"], "depguard");
        assert_eq!(doc["runs"][0]["results"][0]["ruleId"], "deps.no_wildcards");
        assert_eq!(doc["runs"][0]["results"][0]["level"], "error");
        assert_eq!(
            doc["runs"][0]["results"][0]["locations"][0]["physicalLocation"]["artifactLocation"]["uri"],
            "Cargo.toml"
        );
        assert_eq!(
            doc["runs"][0]["results"][0]["locations"][0]["physicalLocation"]["region"]["startLine"],
            8
        );
    }

    #[test]
    fn render_sarif_handles_findings_without_check_id_or_location() {
        let report = RenderableReport {
            verdict: RenderableVerdictStatus::Warn,
            findings: vec![RenderableFinding {
                severity: RenderableSeverity::Warning,
                check_id: None,
                code: "tool_warning".to_string(),
                message: "something happened".to_string(),
                location: None,
                help: None,
                url: None,
            }],
            data: RenderableData {
                findings_emitted: 1,
                findings_total: 1,
                truncated_reason: None,
            },
        };

        let sarif = render_sarif(&report);
        let doc: Value = serde_json::from_str(&sarif).expect("sarif json");

        assert_eq!(
            doc["runs"][0]["results"][0]["ruleId"],
            "depguard.tool_warning"
        );
        assert!(doc["runs"][0]["results"][0].get("locations").is_none());
        assert_eq!(doc["runs"][0]["properties"]["depguard_verdict"], "warn");
    }

    #[test]
    fn render_sarif_empty_report_has_no_results() {
        let report = RenderableReport {
            verdict: RenderableVerdictStatus::Pass,
            findings: Vec::new(),
            data: RenderableData {
                findings_emitted: 0,
                findings_total: 0,
                truncated_reason: None,
            },
        };

        let sarif = render_sarif(&report);
        let doc: Value = serde_json::from_str(&sarif).expect("sarif json");

        assert_eq!(
            doc["runs"][0]["results"].as_array().map(|a| a.len()),
            Some(0)
        );
    }
}
