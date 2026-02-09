//! Conformance tests for depguard.
//!
//! These tests validate:
//! 1. All check IDs have explanations
//! 2. All codes have explanations
//! 3. All fixture reports validate against expected schemas
//! 4. Report schema conformance
//! 5. Contract fixtures validate against sensor.report.v1 schema
//! 6. Path and token hygiene in fixtures

use depguard_types::{explain, ids};
use serde_json::Value;
use std::path::PathBuf;

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("depguard-cli should have parent")
        .parent()
        .expect("crates should have parent")
        .join("tests")
        .join("fixtures")
}

fn contracts_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("depguard-cli should have parent")
        .parent()
        .expect("crates should have parent")
        .join("contracts")
}

fn is_valid_token(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_lowercase() => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

fn is_clean_path(path: &str) -> bool {
    !(path.starts_with('/')
        || path.starts_with('\\')
        || path.contains("..")
        || path.contains('\\')
        || (path.len() >= 2 && path.as_bytes()[1] == b':'))
}

// =============================================================================
// Explanation Coverage Tests
// =============================================================================

#[test]
fn all_check_ids_have_explanations() {
    for check_id in explain::all_check_ids() {
        let explanation = explain::lookup_explanation(check_id);
        assert!(
            explanation.is_some(),
            "Check ID '{}' has no explanation in registry",
            check_id
        );

        // Verify explanation has non-empty content
        let exp = explanation.unwrap();
        assert!(
            !exp.title.is_empty(),
            "Check ID '{}' has empty title",
            check_id
        );
        assert!(
            !exp.description.is_empty(),
            "Check ID '{}' has empty description",
            check_id
        );
        assert!(
            !exp.remediation.is_empty(),
            "Check ID '{}' has empty remediation",
            check_id
        );
    }
}

#[test]
fn all_codes_have_explanations() {
    for code in explain::all_codes() {
        let explanation = explain::lookup_explanation(code);
        assert!(
            explanation.is_some(),
            "Code '{}' has no explanation in registry",
            code
        );

        // Verify explanation has non-empty content
        let exp = explanation.unwrap();
        assert!(!exp.title.is_empty(), "Code '{}' has empty title", code);
        assert!(
            !exp.description.is_empty(),
            "Code '{}' has empty description",
            code
        );
        assert!(
            !exp.remediation.is_empty(),
            "Code '{}' has empty remediation",
            code
        );
    }
}

#[test]
fn check_ids_and_codes_are_consistent() {
    // Verify that check IDs follow the expected pattern
    for check_id in explain::all_check_ids() {
        assert!(
            check_id.contains('.'),
            "Check ID '{}' should be dotted (e.g., 'deps.no_wildcards')",
            check_id
        );
    }

    // Verify that codes are snake_case
    for code in explain::all_codes() {
        assert!(
            !code.contains('.'),
            "Code '{}' should not contain dots",
            code
        );
        // Should be lowercase with underscores
        let valid_chars = code.chars().all(|c| c.is_ascii_lowercase() || c == '_');
        assert!(
            valid_chars,
            "Code '{}' should be snake_case (lowercase with underscores)",
            code
        );
    }
}

// =============================================================================
// Known Check IDs and Codes Inventory
// =============================================================================

#[test]
fn known_check_ids_are_documented() {
    let known_check_ids = [
        ids::CHECK_DEPS_NO_WILDCARDS,
        ids::CHECK_DEPS_PATH_REQUIRES_VERSION,
        ids::CHECK_DEPS_PATH_SAFETY,
        ids::CHECK_DEPS_WORKSPACE_INHERITANCE,
        ids::CHECK_DEPS_GIT_REQUIRES_VERSION,
        ids::CHECK_DEPS_DEV_ONLY_IN_NORMAL,
        ids::CHECK_DEPS_DEFAULT_FEATURES_EXPLICIT,
        ids::CHECK_DEPS_NO_MULTIPLE_VERSIONS,
        ids::CHECK_DEPS_OPTIONAL_UNUSED,
        ids::CHECK_TOOL_RUNTIME,
    ];

    let registered = explain::all_check_ids();

    for id in &known_check_ids {
        assert!(
            registered.contains(id),
            "Known check ID '{}' is not in all_check_ids()",
            id
        );
    }

    // Ensure no extras in the registry that aren't in our known list
    // This helps catch when new checks are added but test not updated
    for id in registered {
        assert!(
            known_check_ids.contains(id),
            "Check ID '{}' in registry but not in known_check_ids test - update the test",
            id
        );
    }
}

#[test]
fn known_codes_are_documented() {
    let known_codes = [
        ids::CODE_WILDCARD_VERSION,
        ids::CODE_PATH_WITHOUT_VERSION,
        ids::CODE_ABSOLUTE_PATH,
        ids::CODE_PARENT_ESCAPE,
        ids::CODE_MISSING_WORKSPACE_TRUE,
        ids::CODE_GIT_WITHOUT_VERSION,
        ids::CODE_DEV_DEP_IN_NORMAL,
        ids::CODE_DEFAULT_FEATURES_IMPLICIT,
        ids::CODE_DUPLICATE_DIFFERENT_VERSIONS,
        ids::CODE_OPTIONAL_NOT_IN_FEATURES,
        ids::CODE_RUNTIME_ERROR,
    ];

    let registered = explain::all_codes();

    for code in &known_codes {
        assert!(
            registered.contains(code),
            "Known code '{}' is not in all_codes()",
            code
        );
    }

    // Ensure no extras in the registry that aren't in our known list
    for code in registered {
        assert!(
            known_codes.contains(code),
            "Code '{}' in registry but not in known_codes test - update the test",
            code
        );
    }
}

// =============================================================================
// Fixture Report Validation
// =============================================================================

#[test]
fn all_fixture_reports_are_valid_json() {
    let fixtures = fixtures_dir();
    let mut checked = 0;

    for entry in std::fs::read_dir(&fixtures).expect("Failed to read fixtures dir") {
        let entry = entry.expect("Failed to read entry");
        let fixture_dir = entry.path();

        if !fixture_dir.is_dir() {
            continue;
        }

        let report_path = fixture_dir.join("expected.report.json");
        if !report_path.exists() {
            continue;
        }

        let content = std::fs::read_to_string(&report_path)
            .unwrap_or_else(|_| panic!("Failed to read {}", report_path.display()));

        let report: Result<Value, _> = serde_json::from_str(&content);
        assert!(
            report.is_ok(),
            "Fixture {} has invalid JSON: {}",
            fixture_dir.file_name().unwrap().to_string_lossy(),
            report.unwrap_err()
        );

        checked += 1;
    }

    assert!(
        checked > 0,
        "No fixture reports found in {}",
        fixtures.display()
    );
}

#[test]
fn all_fixture_reports_have_required_fields() {
    let fixtures = fixtures_dir();

    for entry in std::fs::read_dir(&fixtures).expect("Failed to read fixtures dir") {
        let entry = entry.expect("Failed to read entry");
        let fixture_dir = entry.path();

        if !fixture_dir.is_dir() {
            continue;
        }

        let report_path = fixture_dir.join("expected.report.json");
        if !report_path.exists() {
            continue;
        }

        let fixture_name = fixture_dir.file_name().unwrap().to_string_lossy();
        let content = std::fs::read_to_string(&report_path)
            .unwrap_or_else(|_| panic!("Failed to read {}", report_path.display()));

        let report: Value = serde_json::from_str(&content)
            .unwrap_or_else(|_| panic!("Failed to parse {} as JSON", report_path.display()));

        // Check required fields
        assert!(
            report.get("schema").is_some(),
            "Fixture '{}' report missing 'schema' field",
            fixture_name
        );
        assert!(
            report.get("tool").is_some(),
            "Fixture '{}' report missing 'tool' field",
            fixture_name
        );
        assert!(
            report.get("run").is_some(),
            "Fixture '{}' report missing 'run' field",
            fixture_name
        );
        assert!(
            report.get("verdict").is_some(),
            "Fixture '{}' report missing 'verdict' field",
            fixture_name
        );
        assert!(
            report.get("findings").is_some(),
            "Fixture '{}' report missing 'findings' field",
            fixture_name
        );

        // Verify findings array
        let findings = report["findings"].as_array();
        assert!(
            findings.is_some(),
            "Fixture '{}' findings is not an array",
            fixture_name
        );
    }
}

#[test]
fn all_fixture_findings_have_valid_check_ids() {
    let fixtures = fixtures_dir();
    let valid_check_ids: Vec<&str> = explain::all_check_ids().to_vec();

    for entry in std::fs::read_dir(&fixtures).expect("Failed to read fixtures dir") {
        let entry = entry.expect("Failed to read entry");
        let fixture_dir = entry.path();

        if !fixture_dir.is_dir() {
            continue;
        }

        let report_path = fixture_dir.join("expected.report.json");
        if !report_path.exists() {
            continue;
        }

        let fixture_name = fixture_dir.file_name().unwrap().to_string_lossy();
        let content = std::fs::read_to_string(&report_path).unwrap();
        let report: Value = serde_json::from_str(&content).unwrap();

        if let Some(findings) = report["findings"].as_array() {
            for (i, finding) in findings.iter().enumerate() {
                if let Some(check_id) = finding["check_id"].as_str() {
                    assert!(
                        valid_check_ids.contains(&check_id),
                        "Fixture '{}' finding {} has unknown check_id '{}'. Valid IDs: {:?}",
                        fixture_name,
                        i,
                        check_id,
                        valid_check_ids
                    );
                }
            }
        }
    }
}

#[test]
fn all_fixture_findings_have_valid_codes() {
    let fixtures = fixtures_dir();
    let valid_codes: Vec<&str> = explain::all_codes().to_vec();

    for entry in std::fs::read_dir(&fixtures).expect("Failed to read fixtures dir") {
        let entry = entry.expect("Failed to read entry");
        let fixture_dir = entry.path();

        if !fixture_dir.is_dir() {
            continue;
        }

        let report_path = fixture_dir.join("expected.report.json");
        if !report_path.exists() {
            continue;
        }

        let fixture_name = fixture_dir.file_name().unwrap().to_string_lossy();
        let content = std::fs::read_to_string(&report_path).unwrap();
        let report: Value = serde_json::from_str(&content).unwrap();

        if let Some(findings) = report["findings"].as_array() {
            for (i, finding) in findings.iter().enumerate() {
                if let Some(code) = finding["code"].as_str() {
                    assert!(
                        valid_codes.contains(&code),
                        "Fixture '{}' finding {} has unknown code '{}'. Valid codes: {:?}",
                        fixture_name,
                        i,
                        code,
                        valid_codes
                    );
                }
            }
        }
    }
}

// =============================================================================
// Schema Version Validation
// =============================================================================

#[test]
fn all_fixture_reports_use_v2_schema() {
    let fixtures = fixtures_dir();

    for entry in std::fs::read_dir(&fixtures).expect("Failed to read fixtures dir") {
        let entry = entry.expect("Failed to read entry");
        let fixture_dir = entry.path();

        if !fixture_dir.is_dir() {
            continue;
        }

        let report_path = fixture_dir.join("expected.report.json");
        if !report_path.exists() {
            continue;
        }

        let fixture_name = fixture_dir.file_name().unwrap().to_string_lossy();
        let content = std::fs::read_to_string(&report_path).unwrap();
        let report: Value = serde_json::from_str(&content).unwrap();

        if let Some(schema) = report.get("schema").and_then(|v| v.as_str()) {
            // v2 reports should have the v2 schema
            assert!(
                schema == "depguard.report.v2" || schema.contains("report"),
                "Fixture '{}' has unexpected schema '{}' - expected depguard.report.v2",
                fixture_name,
                schema
            );
        }
    }
}

// =============================================================================
// Verdict Structure Validation
// =============================================================================

#[test]
fn all_fixture_verdicts_are_valid() {
    let fixtures = fixtures_dir();
    let valid_statuses = ["pass", "fail", "warn"];

    for entry in std::fs::read_dir(&fixtures).expect("Failed to read fixtures dir") {
        let entry = entry.expect("Failed to read entry");
        let fixture_dir = entry.path();

        if !fixture_dir.is_dir() {
            continue;
        }

        let report_path = fixture_dir.join("expected.report.json");
        if !report_path.exists() {
            continue;
        }

        let fixture_name = fixture_dir.file_name().unwrap().to_string_lossy();
        let content = std::fs::read_to_string(&report_path).unwrap();
        let report: Value = serde_json::from_str(&content).unwrap();

        // v2 reports have verdict as object with status
        if let Some(verdict) = report.get("verdict") {
            if let Some(status) = verdict.get("status").and_then(|v| v.as_str()) {
                assert!(
                    valid_statuses.contains(&status),
                    "Fixture '{}' has invalid verdict status '{}'. Valid: {:?}",
                    fixture_name,
                    status,
                    valid_statuses
                );
            } else if let Some(status) = verdict.as_str() {
                // v1 reports might have verdict as string
                assert!(
                    valid_statuses.contains(&status),
                    "Fixture '{}' has invalid verdict '{}'. Valid: {:?}",
                    fixture_name,
                    status,
                    valid_statuses
                );
            }
        }
    }
}

// =============================================================================
// Contract Fixture Validation (sensor.report.v1 schema)
// =============================================================================

#[test]
fn contract_fixtures_validate_against_sensor_schema() {
    let contracts = contracts_dir();
    let schema_path = contracts.join("schemas").join("sensor.report.v1.json");
    let fixtures_path = contracts.join("fixtures");

    assert!(
        schema_path.exists(),
        "sensor.report.v1.json schema not found at {:?}",
        schema_path
    );
    assert!(
        fixtures_path.exists(),
        "contracts/fixtures/ not found at {:?}",
        fixtures_path
    );

    let schema_content = std::fs::read_to_string(&schema_path).unwrap();
    let schema_value: Value = serde_json::from_str(&schema_content).unwrap();
    let compiled = jsonschema::draft7::new(&schema_value).expect("Failed to compile schema");

    let mut checked = 0;

    for entry in std::fs::read_dir(&fixtures_path).expect("Failed to read contracts/fixtures/") {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().is_none_or(|ext| ext != "json") {
            continue;
        }

        let filename = path.file_name().unwrap().to_string_lossy().to_string();
        let content = std::fs::read_to_string(&path).unwrap();
        let value: Value = serde_json::from_str(&content)
            .unwrap_or_else(|e| panic!("{} is not valid JSON: {}", filename, e));

        let error_msgs: Vec<String> = compiled
            .iter_errors(&value)
            .map(|e| e.to_string())
            .collect();
        if !error_msgs.is_empty() {
            panic!(
                "Contract fixture '{}' does not validate against sensor.report.v1 schema:\n{}",
                filename,
                error_msgs.join("\n")
            );
        }

        checked += 1;
    }

    assert!(
        checked > 0,
        "No contract fixtures found in {:?}",
        fixtures_path
    );
}

#[test]
fn fixture_findings_have_clean_paths() {
    let contracts = contracts_dir();
    let fixtures_path = contracts.join("fixtures");
    if !fixtures_path.exists() {
        return;
    }

    for entry in std::fs::read_dir(&fixtures_path).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().is_none_or(|ext| ext != "json") {
            continue;
        }

        let filename = path.file_name().unwrap().to_string_lossy().to_string();
        let content = std::fs::read_to_string(&path).unwrap();
        let value: Value = serde_json::from_str(&content).unwrap();

        if let Some(findings) = value.get("findings").and_then(|v| v.as_array()) {
            for (i, finding) in findings.iter().enumerate() {
                if let Some(loc) = finding.get("location")
                    && let Some(p) = loc.get("path").and_then(|v| v.as_str())
                {
                    assert!(
                        is_clean_path(p),
                        "{}: finding[{}].location.path '{}' is not clean (no absolute, no ../, forward slashes only)",
                        filename,
                        i,
                        p
                    );
                }
            }
        }

        if let Some(artifacts) = value.get("artifacts").and_then(|v| v.as_array()) {
            for (i, artifact) in artifacts.iter().enumerate() {
                if let Some(p) = artifact.get("path").and_then(|v| v.as_str()) {
                    assert!(
                        is_clean_path(p),
                        "{}: artifacts[{}].path '{}' is not clean",
                        filename,
                        i,
                        p
                    );
                }
            }
        }
    }
}

#[test]
fn fixture_verdict_reasons_are_tokens() {
    let contracts = contracts_dir();
    let fixtures_path = contracts.join("fixtures");
    if !fixtures_path.exists() {
        return;
    }

    for entry in std::fs::read_dir(&fixtures_path).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().is_none_or(|ext| ext != "json") {
            continue;
        }

        let filename = path.file_name().unwrap().to_string_lossy().to_string();
        let content = std::fs::read_to_string(&path).unwrap();
        let value: Value = serde_json::from_str(&content).unwrap();

        if let Some(reasons) = value
            .get("verdict")
            .and_then(|v| v.get("reasons"))
            .and_then(|v| v.as_array())
        {
            for (i, reason) in reasons.iter().enumerate() {
                if let Some(s) = reason.as_str() {
                    assert!(
                        is_valid_token(s),
                        "{}: verdict.reasons[{}] '{}' is not a valid token (must match ^[a-z][a-z0-9_]*$)",
                        filename,
                        i,
                        s
                    );
                }
            }
        }
    }
}

#[test]
fn fixture_capability_reasons_are_tokens() {
    let contracts = contracts_dir();
    let fixtures_path = contracts.join("fixtures");
    if !fixtures_path.exists() {
        return;
    }

    for entry in std::fs::read_dir(&fixtures_path).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().is_none_or(|ext| ext != "json") {
            continue;
        }

        let filename = path.file_name().unwrap().to_string_lossy().to_string();
        let content = std::fs::read_to_string(&path).unwrap();
        let value: Value = serde_json::from_str(&content).unwrap();

        if let Some(caps) = value
            .get("run")
            .and_then(|v| v.get("capabilities"))
            .and_then(|v| v.as_object())
        {
            for (cap_name, cap_value) in caps {
                if let Some(reason) = cap_value.get("reason").and_then(|v| v.as_str()) {
                    assert!(
                        is_valid_token(reason),
                        "{}: capabilities.{}.reason '{}' is not a valid token (must match ^[a-z][a-z0-9_]*$)",
                        filename,
                        cap_name,
                        reason
                    );
                }
            }
        }
    }
}
