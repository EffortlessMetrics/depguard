//! End-to-end CLI integration tests using test fixtures.
//!
//! Each fixture in `tests/fixtures/` contains:
//! - A Cargo.toml (and optionally workspace members)
//! - An expected.report.json with expected output (timestamps use "__TIMESTAMP__" placeholder)
//!
//! These tests run the CLI against each fixture and verify:
//! 1. Exit code matches expected (0=pass, 2=fail)
//! 2. JSON output matches expected (ignoring timestamps)

use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;
use std::path::PathBuf;
use tempfile::TempDir;

/// Helper to get a Command for the depguard binary.
/// Wraps the deprecated cargo_bin to centralize the deprecation warning.
#[allow(deprecated)]
fn depguard_cmd() -> Command {
    Command::cargo_bin("depguard").expect("depguard binary not found - run `cargo build` first")
}

/// Get the path to the test fixtures directory
fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("depguard-cli crate should have a parent directory")
        .parent()
        .expect("crates directory should have a parent (repo root)")
        .join("tests")
        .join("fixtures")
}

/// Normalize a JSON value by replacing timestamp fields with a placeholder.
/// This allows comparison of outputs that contain non-deterministic timestamps.
fn normalize_timestamps(mut value: Value) -> Value {
    if let Some(obj) = value.as_object_mut() {
        if obj.contains_key("started_at") {
            obj.insert(
                "started_at".to_string(),
                Value::String("__TIMESTAMP__".to_string()),
            );
        }
        if obj.contains_key("finished_at") {
            obj.insert(
                "finished_at".to_string(),
                Value::String("__TIMESTAMP__".to_string()),
            );
        }
        for (_, v) in obj.iter_mut() {
            *v = normalize_timestamps(v.take());
        }
    } else if let Some(arr) = value.as_array_mut() {
        for v in arr.iter_mut() {
            *v = normalize_timestamps(v.take());
        }
    }
    value
}

/// Run the CLI check command against a fixture and return the JSON report.
fn run_check_on_fixture(fixture_name: &str) -> (i32, Value) {
    let fixture_path = fixtures_dir().join(fixture_name);
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let report_path = temp_dir.path().join("report.json");

    let output = depguard_cmd()
        .arg("--repo-root")
        .arg(&fixture_path)
        .arg("check")
        .arg("--report-out")
        .arg(&report_path)
        .output()
        .expect("Failed to run command");

    let exit_code = output.status.code().unwrap_or(-1);

    let report_content = std::fs::read_to_string(&report_path).expect("Failed to read report");
    let report: Value = serde_json::from_str(&report_content).expect("Failed to parse report JSON");

    (exit_code, report)
}

/// Load and parse the expected report for a fixture.
fn load_expected_report(fixture_name: &str) -> Value {
    let expected_path = fixtures_dir()
        .join(fixture_name)
        .join("expected.report.json");
    let content = std::fs::read_to_string(&expected_path).expect("Failed to read expected report");
    serde_json::from_str(&content).expect("Failed to parse expected report")
}

/// Compare two JSON values, ignoring timestamp differences.
fn assert_reports_match(actual: Value, expected: Value, fixture_name: &str) {
    let actual_normalized = normalize_timestamps(actual);
    let expected_normalized = normalize_timestamps(expected);

    assert_eq!(
        actual_normalized,
        expected_normalized,
        "Report mismatch for fixture '{}'.\n\nActual:\n{}\n\nExpected:\n{}",
        fixture_name,
        serde_json::to_string_pretty(&actual_normalized).unwrap(),
        serde_json::to_string_pretty(&expected_normalized).unwrap()
    );
}

// ============================================================================
// Fixture tests
// ============================================================================

#[test]
fn fixture_clean_passes() {
    let (exit_code, report) = run_check_on_fixture("clean");
    let expected = load_expected_report("clean");

    assert_eq!(exit_code, 0, "clean fixture should exit with 0 (pass)");
    assert_reports_match(report, expected, "clean");
}

#[test]
fn fixture_wildcards_fails() {
    let (exit_code, report) = run_check_on_fixture("wildcards");
    let expected = load_expected_report("wildcards");

    assert_eq!(exit_code, 2, "wildcards fixture should exit with 2 (fail)");
    assert_reports_match(report, expected, "wildcards");
}

#[test]
fn fixture_path_missing_version_fails() {
    let (exit_code, report) = run_check_on_fixture("path_missing_version");
    let expected = load_expected_report("path_missing_version");

    assert_eq!(
        exit_code, 2,
        "path_missing_version fixture should exit with 2 (fail)"
    );
    assert_reports_match(report, expected, "path_missing_version");
}

#[test]
fn fixture_path_safety_fails() {
    let (exit_code, report) = run_check_on_fixture("path_safety");
    let expected = load_expected_report("path_safety");

    assert_eq!(
        exit_code, 2,
        "path_safety fixture should exit with 2 (fail)"
    );
    assert_reports_match(report, expected, "path_safety");
}

#[test]
fn fixture_workspace_inheritance_fails() {
    let (exit_code, report) = run_check_on_fixture("workspace_inheritance");
    let expected = load_expected_report("workspace_inheritance");

    assert_eq!(
        exit_code, 2,
        "workspace_inheritance fixture should exit with 2 (fail)"
    );
    assert_reports_match(report, expected, "workspace_inheritance");
}

#[test]
fn fixture_multi_violation_fails() {
    let (exit_code, report) = run_check_on_fixture("multi_violation");
    let expected = load_expected_report("multi_violation");

    assert_eq!(
        exit_code, 2,
        "multi_violation fixture should exit with 2 (fail)"
    );

    // Verify deterministic ordering: findings should be sorted by line number
    // (all have same severity and path, so line is the tiebreaker)
    let findings = report["findings"].as_array().expect("findings should be array");
    let lines: Vec<i64> = findings
        .iter()
        .map(|f| f["location"]["line"].as_i64().unwrap())
        .collect();
    let mut sorted_lines = lines.clone();
    sorted_lines.sort();
    assert_eq!(lines, sorted_lines, "findings should be sorted by line number");

    assert_reports_match(report, expected, "multi_violation");
}

// ============================================================================
// CLI behavior tests
// ============================================================================

#[test]
fn check_command_creates_output_file() {
    let fixture_path = fixtures_dir().join("clean");
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let report_path = temp_dir.path().join("subdir").join("report.json");

    depguard_cmd()
        .arg("--repo-root")
        .arg(&fixture_path)
        .arg("check")
        .arg("--report-out")
        .arg(&report_path)
        .assert()
        .success();

    assert!(report_path.exists(), "Report file should be created");
}

#[test]
fn check_with_markdown_output() {
    let fixture_path = fixtures_dir().join("wildcards");
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let report_path = temp_dir.path().join("report.json");
    let md_path = temp_dir.path().join("report.md");

    depguard_cmd()
        .arg("--repo-root")
        .arg(&fixture_path)
        .arg("check")
        .arg("--report-out")
        .arg(&report_path)
        .arg("--write-markdown")
        .arg("--markdown-out")
        .arg(&md_path)
        .assert()
        .code(2);

    assert!(report_path.exists(), "JSON report should be created");
    assert!(md_path.exists(), "Markdown report should be created");

    let md_content =
        std::fs::read_to_string(&md_path).expect("failed to read generated markdown file");
    assert!(
        md_content.to_lowercase().contains("fail"),
        "Markdown should contain verdict"
    );
    assert!(
        md_content.contains("wildcard"),
        "Markdown should contain finding"
    );
}

#[test]
fn md_command_renders_from_report() {
    // First, create a report
    let fixture_path = fixtures_dir().join("wildcards");
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let report_path = temp_dir.path().join("report.json");

    depguard_cmd()
        .arg("--repo-root")
        .arg(&fixture_path)
        .arg("check")
        .arg("--report-out")
        .arg(&report_path)
        .assert()
        .code(2);

    // Then, render markdown from it
    let output = depguard_cmd()
        .arg("md")
        .arg("--report")
        .arg(&report_path)
        .output()
        .expect("Failed to run md command");

    assert!(output.status.success(), "md command should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("fail") || stdout.contains("Fail"),
        "Should contain verdict"
    );
}

#[test]
fn annotations_command_renders_gha_format() {
    // First, create a report
    let fixture_path = fixtures_dir().join("wildcards");
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let report_path = temp_dir.path().join("report.json");

    depguard_cmd()
        .arg("--repo-root")
        .arg(&fixture_path)
        .arg("check")
        .arg("--report-out")
        .arg(&report_path)
        .assert()
        .code(2);

    // Then, render annotations from it
    let output = depguard_cmd()
        .arg("annotations")
        .arg("--report")
        .arg(&report_path)
        .output()
        .expect("Failed to run annotations command");

    assert!(
        output.status.success(),
        "annotations command should succeed"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("::error"),
        "Should contain GHA error annotation format"
    );
}

#[test]
fn explain_command_shows_check_info() {
    let output = depguard_cmd()
        .arg("explain")
        .arg("deps.no_wildcards")
        .output()
        .expect("Failed to run explain command");

    assert!(output.status.success(), "explain command should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("wildcard"), "Should explain wildcard check");
}

#[test]
fn explain_command_shows_code_info() {
    let output = depguard_cmd()
        .arg("explain")
        .arg("wildcard_version")
        .output()
        .expect("Failed to run explain command");

    assert!(output.status.success(), "explain command should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("wildcard") || stdout.contains("*"),
        "Should explain wildcard code"
    );
}

#[test]
fn explain_unknown_returns_error() {
    depguard_cmd()
        .arg("explain")
        .arg("nonexistent_check")
        .assert()
        .failure();
}

#[test]
fn version_flag_works() {
    depguard_cmd()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("0.1.0"));
}

#[test]
fn missing_repo_root_returns_error() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let report_path = temp_dir.path().join("report.json");

    depguard_cmd()
        .arg("--repo-root")
        .arg("/nonexistent/path/to/repo")
        .arg("check")
        .arg("--report-out")
        .arg(&report_path)
        .assert()
        .failure();
}
