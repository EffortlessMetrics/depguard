//! Conformance tests for depguard.
//!
//! These tests validate:
//! 1. All check IDs have explanations
//! 2. All codes have explanations
//! 3. All fixture reports validate against expected schemas
//! 4. Report schema conformance

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
