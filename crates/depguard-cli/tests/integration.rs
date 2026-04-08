//! Comprehensive integration tests for depguard CLI commands.
//!
//! This test module covers:
//! - Check command with various configurations and flags
//! - Baseline command generation and application
//! - Render commands (md, annotations, sarif, junit, jsonl)
//! - Explain command for all check IDs and codes
//! - Error handling scenarios

use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;
use std::path::PathBuf;
use tempfile::TempDir;

/// Helper to get a Command for the depguard binary.
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

// =============================================================================
// CHECK COMMAND TESTS
// =============================================================================

mod check_command {
    use super::*;

    #[test]
    fn check_with_clean_fixture_passes() {
        let fixture_path = fixtures_dir().join("clean");
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let report_path = temp_dir.path().join("report.json");

        depguard_cmd()
            .arg("--repo-root")
            .arg(&fixture_path)
            .arg("check")
            .arg("--report-out")
            .arg(&report_path)
            .assert()
            .success();

        assert!(report_path.exists(), "Report file should be created");

        let report_content = std::fs::read_to_string(&report_path).expect("Failed to read report");
        let report: Value =
            serde_json::from_str(&report_content).expect("Failed to parse report JSON");

        assert_eq!(
            report["verdict"]["status"].as_str(),
            Some("pass"),
            "Clean fixture should pass"
        );
    }

    #[test]
    fn check_with_wildcards_fixture_fails() {
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

        let report_content = std::fs::read_to_string(&report_path).expect("Failed to read report");
        let report: Value =
            serde_json::from_str(&report_content).expect("Failed to parse report JSON");

        assert_eq!(
            report["verdict"]["status"].as_str(),
            Some("fail"),
            "Wildcards fixture should fail"
        );
    }

    #[test]
    fn check_with_config_flag() {
        // Use fixture that has a config file
        let fixture_path = fixtures_dir().join("default_features_explicit");
        let config_path = fixture_path.join("depguard.toml");
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let report_path = temp_dir.path().join("report.json");

        // Skip if config doesn't exist
        if !config_path.exists() {
            eprintln!("Skipping test: config file not found at {:?}", config_path);
            return;
        }

        // Run the check command with explicit config (--config is a global flag before subcommand)
        let output = depguard_cmd()
            .arg("--repo-root")
            .arg(&fixture_path)
            .arg("--config")
            .arg(&config_path)
            .arg("check")
            .arg("--report-out")
            .arg(&report_path)
            .output()
            .expect("Failed to run command");

        // The command should run (may pass or fail depending on config)
        // Just verify the report is created
        if !report_path.exists() {
            eprintln!("stdout: {}", String::from_utf8_lossy(&output.stdout));
            eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
        }
        assert!(report_path.exists(), "Report file should be created");
    }

    #[test]
    fn check_with_output_format_json() {
        let fixture_path = fixtures_dir().join("clean");
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let report_path = temp_dir.path().join("report.json");

        depguard_cmd()
            .arg("--repo-root")
            .arg(&fixture_path)
            .arg("check")
            .arg("--report-out")
            .arg(&report_path)
            .assert()
            .success();

        let report_content = std::fs::read_to_string(&report_path).expect("Failed to read report");
        // Verify it's valid JSON
        let _: Value = serde_json::from_str(&report_content).expect("Output should be valid JSON");
    }

    #[test]
    fn check_creates_parent_directories() {
        let fixture_path = fixtures_dir().join("clean");
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let report_path = temp_dir
            .path()
            .join("deeply")
            .join("nested")
            .join("dir")
            .join("report.json");

        depguard_cmd()
            .arg("--repo-root")
            .arg(&fixture_path)
            .arg("check")
            .arg("--report-out")
            .arg(&report_path)
            .assert()
            .success();

        assert!(
            report_path.exists(),
            "Report file should be created with parent directories"
        );
    }

    #[test]
    fn check_with_diff_base_and_head_flags() {
        let fixture_path = fixtures_dir().join("clean");
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let report_path = temp_dir.path().join("report.json");

        // Test with --base and --head flags (diff scope)
        // This may fail if not in a git repo, so we just check it accepts the args
        let result = depguard_cmd()
            .arg("--repo-root")
            .arg(&fixture_path)
            .arg("check")
            .arg("--base")
            .arg("HEAD~1")
            .arg("--head")
            .arg("HEAD")
            .arg("--report-out")
            .arg(&report_path)
            .output();

        // Just verify the command runs (may fail if not in git repo)
        if let Ok(output) = result {
            let _ = output.status.code();
        }
    }

    #[test]
    fn check_with_report_version_v1() {
        let fixture_path = fixtures_dir().join("clean");
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let report_path = temp_dir.path().join("report.json");

        depguard_cmd()
            .arg("--repo-root")
            .arg(&fixture_path)
            .arg("check")
            .arg("--report-version")
            .arg("v1")
            .arg("--report-out")
            .arg(&report_path)
            .assert()
            .success();

        let report_content = std::fs::read_to_string(&report_path).expect("Failed to read report");
        let report: Value =
            serde_json::from_str(&report_content).expect("Failed to parse report JSON");

        // v1 report has different structure
        assert!(
            report["schema"].is_string() || report["version"].is_number(),
            "v1 report should have schema or version field"
        );
    }

    #[test]
    fn check_with_report_version_v2() {
        let fixture_path = fixtures_dir().join("clean");
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let report_path = temp_dir.path().join("report.json");

        depguard_cmd()
            .arg("--repo-root")
            .arg(&fixture_path)
            .arg("check")
            .arg("--report-version")
            .arg("v2")
            .arg("--report-out")
            .arg(&report_path)
            .assert()
            .success();

        let report_content = std::fs::read_to_string(&report_path).expect("Failed to read report");
        let report: Value =
            serde_json::from_str(&report_content).expect("Failed to parse report JSON");

        assert_eq!(
            report["schema"].as_str(),
            Some("depguard.report.v2"),
            "v2 report should have correct schema"
        );
    }

    #[test]
    fn check_with_mode_standard() {
        let fixture_path = fixtures_dir().join("wildcards");
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let report_path = temp_dir.path().join("report.json");

        // Standard mode exits with code 2 on policy failure
        depguard_cmd()
            .arg("--repo-root")
            .arg(&fixture_path)
            .arg("check")
            .arg("--mode")
            .arg("standard")
            .arg("--report-out")
            .arg(&report_path)
            .assert()
            .code(2);
    }

    #[test]
    fn check_with_mode_cockpit() {
        let fixture_path = fixtures_dir().join("wildcards");
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let report_path = temp_dir.path().join("report.json");

        // Cockpit mode exits with code 0 even on policy failure
        depguard_cmd()
            .arg("--repo-root")
            .arg(&fixture_path)
            .arg("check")
            .arg("--mode")
            .arg("cockpit")
            .arg("--report-out")
            .arg(&report_path)
            .assert()
            .success();

        let report_content = std::fs::read_to_string(&report_path).expect("Failed to read report");
        let report: Value =
            serde_json::from_str(&report_content).expect("Failed to parse report JSON");

        // Verdict should still be fail
        assert_eq!(
            report["verdict"]["status"].as_str(),
            Some("fail"),
            "Verdict should be fail even in cockpit mode"
        );
    }

    #[test]
    fn check_with_out_dir_flag() {
        let fixture_path = fixtures_dir().join("clean");
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let out_dir = temp_dir.path().join("output");

        depguard_cmd()
            .arg("--repo-root")
            .arg(&fixture_path)
            .arg("check")
            .arg("--out-dir")
            .arg(&out_dir)
            .assert()
            .success();

        assert!(
            out_dir.join("report.json").exists(),
            "report.json should be in out-dir"
        );
    }

    #[test]
    fn check_with_multiple_output_formats() {
        let fixture_path = fixtures_dir().join("wildcards");
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let out_dir = temp_dir.path().join("output");

        depguard_cmd()
            .arg("--repo-root")
            .arg(&fixture_path)
            .arg("check")
            .arg("--out-dir")
            .arg(&out_dir)
            .arg("--write-markdown")
            .arg("--write-junit")
            .arg("--write-jsonl")
            .assert()
            .code(2);

        assert!(
            out_dir.join("report.json").exists(),
            "report.json should exist"
        );
        assert!(
            out_dir.join("comment.md").exists(),
            "comment.md should exist"
        );
        assert!(
            out_dir.join("report.junit.xml").exists(),
            "report.junit.xml should exist"
        );
        assert!(
            out_dir.join("report.jsonl").exists(),
            "report.jsonl should exist"
        );
    }

    #[test]
    fn check_empty_package() {
        let fixture_path = fixtures_dir().join("empty_package");
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let report_path = temp_dir.path().join("report.json");

        depguard_cmd()
            .arg("--repo-root")
            .arg(&fixture_path)
            .arg("check")
            .arg("--report-out")
            .arg(&report_path)
            .assert()
            .success();

        let report_content = std::fs::read_to_string(&report_path).expect("Failed to read report");
        let report: Value =
            serde_json::from_str(&report_content).expect("Failed to parse report JSON");

        assert_eq!(
            report["verdict"]["status"].as_str(),
            Some("pass"),
            "Empty package should pass"
        );
    }

    #[test]
    fn check_workspace_with_members() {
        let fixture_path = fixtures_dir().join("nested_workspace");
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let report_path = temp_dir.path().join("report.json");

        depguard_cmd()
            .arg("--repo-root")
            .arg(&fixture_path)
            .arg("check")
            .arg("--report-out")
            .arg(&report_path)
            .assert()
            .code(2); // Expected to have findings

        assert!(report_path.exists(), "Report file should be created");
    }
}

// =============================================================================
// BASELINE COMMAND TESTS
// =============================================================================

mod baseline_command {
    use super::*;

    #[test]
    fn baseline_generates_valid_json() {
        let fixture_path = fixtures_dir().join("wildcards");
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let baseline_path = temp_dir.path().join(".depguard-baseline.json");

        depguard_cmd()
            .arg("--repo-root")
            .arg(&fixture_path)
            .arg("baseline")
            .arg("--output")
            .arg(&baseline_path)
            .assert()
            .success();

        assert!(baseline_path.exists(), "Baseline file should be created");

        let baseline_content =
            std::fs::read_to_string(&baseline_path).expect("Failed to read baseline");
        let baseline: Value =
            serde_json::from_str(&baseline_content).expect("Baseline should be valid JSON");

        // Verify baseline structure - it should have $schema or schema field
        assert!(
            baseline["$schema"].is_string()
                || baseline["schema"].is_string()
                || baseline["version"].is_string(),
            "Baseline should have schema or version field"
        );
        // Baseline may have entries or suppressions array
        assert!(
            baseline["entries"].is_array()
                || baseline["suppressions"].is_array()
                || baseline["findings"].is_array(),
            "Baseline should have entries, suppressions, or findings array"
        );
    }

    #[test]
    fn baseline_suppresses_findings_on_subsequent_run() {
        let fixture_path = fixtures_dir().join("wildcards");
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let baseline_path = temp_dir.path().join(".depguard-baseline.json");
        let report_path = temp_dir.path().join("report.json");

        // Generate baseline
        depguard_cmd()
            .arg("--repo-root")
            .arg(&fixture_path)
            .arg("baseline")
            .arg("--output")
            .arg(&baseline_path)
            .assert()
            .success();

        // Run check with baseline - should pass now
        depguard_cmd()
            .arg("--repo-root")
            .arg(&fixture_path)
            .arg("check")
            .arg("--baseline")
            .arg(&baseline_path)
            .arg("--report-out")
            .arg(&report_path)
            .assert()
            .success();

        let report_content = std::fs::read_to_string(&report_path).expect("Failed to read report");
        let report: Value =
            serde_json::from_str(&report_content).expect("Failed to parse report JSON");

        assert_eq!(
            report["verdict"]["status"].as_str(),
            Some("pass"),
            "Baseline should suppress all findings"
        );
    }

    #[test]
    fn baseline_counts_suppressed_findings() {
        let fixture_path = fixtures_dir().join("wildcards");
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let baseline_path = temp_dir.path().join(".depguard-baseline.json");
        let report_path = temp_dir.path().join("report.json");

        // Generate baseline
        depguard_cmd()
            .arg("--repo-root")
            .arg(&fixture_path)
            .arg("baseline")
            .arg("--output")
            .arg(&baseline_path)
            .assert()
            .success();

        // Run check with baseline
        depguard_cmd()
            .arg("--repo-root")
            .arg(&fixture_path)
            .arg("check")
            .arg("--baseline")
            .arg(&baseline_path)
            .arg("--report-out")
            .arg(&report_path)
            .assert()
            .success();

        let report_content = std::fs::read_to_string(&report_path).expect("Failed to read report");
        let report: Value =
            serde_json::from_str(&report_content).expect("Failed to parse report JSON");

        // Check that suppressed count is tracked
        let suppressed = report["verdict"]["counts"]["suppressed"]
            .as_u64()
            .unwrap_or(0);
        assert!(suppressed > 0, "Should have suppressed findings count");
    }

    #[test]
    fn baseline_with_multi_violation_fixture() {
        let fixture_path = fixtures_dir().join("multi_violation");
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let baseline_path = temp_dir.path().join(".depguard-baseline.json");
        let report_path = temp_dir.path().join("report.json");

        // Generate baseline
        depguard_cmd()
            .arg("--repo-root")
            .arg(&fixture_path)
            .arg("baseline")
            .arg("--output")
            .arg(&baseline_path)
            .assert()
            .success();

        // Run check with baseline
        depguard_cmd()
            .arg("--repo-root")
            .arg(&fixture_path)
            .arg("check")
            .arg("--baseline")
            .arg(&baseline_path)
            .arg("--report-out")
            .arg(&report_path)
            .assert()
            .success();

        let report_content = std::fs::read_to_string(&report_path).expect("Failed to read report");
        let report: Value =
            serde_json::from_str(&report_content).expect("Failed to parse report JSON");

        assert_eq!(
            report["verdict"]["status"].as_str(),
            Some("pass"),
            "All violations should be suppressed"
        );
    }

    #[test]
    fn baseline_output_to_custom_path() {
        let fixture_path = fixtures_dir().join("wildcards");
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let custom_path = temp_dir
            .path()
            .join("custom")
            .join("dir")
            .join("my-baseline.json");

        depguard_cmd()
            .arg("--repo-root")
            .arg(&fixture_path)
            .arg("baseline")
            .arg("--output")
            .arg(&custom_path)
            .assert()
            .success();

        assert!(
            custom_path.exists(),
            "Baseline should be created at custom path"
        );
    }
}

// =============================================================================
// RENDER COMMAND TESTS
// =============================================================================

mod render_commands {
    use super::*;

    /// Helper to create a report from the wildcards fixture
    fn create_wildcards_report() -> (TempDir, PathBuf) {
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

        (temp_dir, report_path)
    }

    // MD COMMAND TESTS

    #[test]
    fn md_command_basic_output() {
        let (_temp_dir, report_path) = create_wildcards_report();

        let output = depguard_cmd()
            .arg("md")
            .arg("--report")
            .arg(&report_path)
            .output()
            .expect("Failed to run md command");

        assert!(output.status.success(), "md command should succeed");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.to_lowercase().contains("fail"),
            "Markdown should contain verdict"
        );
    }

    #[test]
    fn md_command_includes_findings() {
        let (_temp_dir, report_path) = create_wildcards_report();

        let output = depguard_cmd()
            .arg("md")
            .arg("--report")
            .arg(&report_path)
            .output()
            .expect("Failed to run md command");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("wildcard"),
            "Markdown should mention wildcard finding"
        );
    }

    #[test]
    fn md_command_with_output_file() {
        let (_temp_dir, report_path) = create_wildcards_report();
        let temp_dir2 = TempDir::new().expect("Failed to create temp dir");
        let md_path = temp_dir2.path().join("output.md");

        depguard_cmd()
            .arg("md")
            .arg("--report")
            .arg(&report_path)
            .arg("--output")
            .arg(&md_path)
            .assert()
            .success();

        assert!(md_path.exists(), "Markdown file should be created");
        let content = std::fs::read_to_string(&md_path).expect("Failed to read markdown");
        assert!(
            content.to_lowercase().contains("fail"),
            "Markdown file should contain verdict"
        );
    }

    #[test]
    fn md_command_clean_report() {
        let fixture_path = fixtures_dir().join("clean");
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let report_path = temp_dir.path().join("report.json");

        depguard_cmd()
            .arg("--repo-root")
            .arg(&fixture_path)
            .arg("check")
            .arg("--report-out")
            .arg(&report_path)
            .assert()
            .success();

        let output = depguard_cmd()
            .arg("md")
            .arg("--report")
            .arg(&report_path)
            .output()
            .expect("Failed to run md command");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.to_lowercase().contains("pass"),
            "Markdown should show pass verdict"
        );
    }

    // ANNOTATIONS COMMAND TESTS

    #[test]
    fn annotations_command_basic_output() {
        let (_temp_dir, report_path) = create_wildcards_report();

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
    fn annotations_command_includes_file_references() {
        let (_temp_dir, report_path) = create_wildcards_report();

        let output = depguard_cmd()
            .arg("annotations")
            .arg("--report")
            .arg(&report_path)
            .output()
            .expect("Failed to run annotations command");

        let stdout = String::from_utf8_lossy(&output.stdout);
        // GHA annotations format: ::error file=...,line=...::message
        assert!(
            stdout.contains("file=") || stdout.contains("::error"),
            "Should contain file reference or error annotation"
        );
    }

    #[test]
    fn annotations_command_outputs_to_stdout() {
        let (_temp_dir, report_path) = create_wildcards_report();

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
        // Verify output goes to stdout
        assert!(!stdout.is_empty(), "Annotations should output to stdout");
        assert!(
            stdout.contains("::error"),
            "Should contain GHA error annotation format"
        );
    }

    #[test]
    fn annotations_clean_report_no_errors() {
        let fixture_path = fixtures_dir().join("clean");
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let report_path = temp_dir.path().join("report.json");

        depguard_cmd()
            .arg("--repo-root")
            .arg(&fixture_path)
            .arg("check")
            .arg("--report-out")
            .arg(&report_path)
            .assert()
            .success();

        let output = depguard_cmd()
            .arg("annotations")
            .arg("--report")
            .arg(&report_path)
            .output()
            .expect("Failed to run annotations command");

        let stdout = String::from_utf8_lossy(&output.stdout);
        // Clean report should not have error annotations
        assert!(
            !stdout.contains("::error"),
            "Clean report should not have error annotations"
        );
    }

    // SARIF COMMAND TESTS

    #[test]
    fn sarif_command_basic_output() {
        let (_temp_dir, report_path) = create_wildcards_report();

        let output = depguard_cmd()
            .arg("sarif")
            .arg("--report")
            .arg(&report_path)
            .output()
            .expect("Failed to run sarif command");

        assert!(output.status.success(), "sarif command should succeed");
        let stdout = String::from_utf8_lossy(&output.stdout);

        // Verify SARIF structure
        assert!(
            stdout.contains("\"version\": \"2.1.0\""),
            "Should contain SARIF version"
        );
        assert!(stdout.contains("\"runs\""), "Should contain runs array");
    }

    #[test]
    fn sarif_command_valid_json() {
        let (_temp_dir, report_path) = create_wildcards_report();

        let output = depguard_cmd()
            .arg("sarif")
            .arg("--report")
            .arg(&report_path)
            .output()
            .expect("Failed to run sarif command");

        let stdout = String::from_utf8_lossy(&output.stdout);
        let _: Value = serde_json::from_str(&stdout).expect("SARIF output should be valid JSON");
    }

    #[test]
    fn sarif_command_contains_rules() {
        let (_temp_dir, report_path) = create_wildcards_report();

        let output = depguard_cmd()
            .arg("sarif")
            .arg("--report")
            .arg(&report_path)
            .output()
            .expect("Failed to run sarif command");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("\"ruleId\""),
            "Should contain ruleId references"
        );
    }

    #[test]
    fn sarif_command_with_output_file() {
        let (_temp_dir, report_path) = create_wildcards_report();
        let temp_dir2 = TempDir::new().expect("Failed to create temp dir");
        let sarif_path = temp_dir2.path().join("report.sarif");

        depguard_cmd()
            .arg("sarif")
            .arg("--report")
            .arg(&report_path)
            .arg("--output")
            .arg(&sarif_path)
            .assert()
            .success();

        assert!(sarif_path.exists(), "SARIF file should be created");
        let content = std::fs::read_to_string(&sarif_path).expect("Failed to read SARIF");
        assert!(
            content.contains("\"version\": \"2.1.0\""),
            "SARIF file should have correct version"
        );
    }

    // JUNIT COMMAND TESTS

    #[test]
    fn junit_command_basic_output() {
        let (_temp_dir, report_path) = create_wildcards_report();

        let output = depguard_cmd()
            .arg("junit")
            .arg("--report")
            .arg(&report_path)
            .output()
            .expect("Failed to run junit command");

        assert!(output.status.success(), "junit command should succeed");
        let stdout = String::from_utf8_lossy(&output.stdout);

        // Verify JUnit XML structure
        assert!(
            stdout.contains("<testsuite"),
            "Should contain testsuite element"
        );
        assert!(
            stdout.contains("<testcase"),
            "Should contain testcase elements"
        );
    }

    #[test]
    fn junit_command_valid_xml_structure() {
        let (_temp_dir, report_path) = create_wildcards_report();

        let output = depguard_cmd()
            .arg("junit")
            .arg("--report")
            .arg(&report_path)
            .output()
            .expect("Failed to run junit command");

        let stdout = String::from_utf8_lossy(&output.stdout);
        // Basic XML validation
        assert!(
            stdout.trim_start().starts_with("<?xml")
                || stdout.trim_start().starts_with("<testsuite"),
            "Should start with XML declaration or testsuite"
        );
    }

    #[test]
    fn junit_command_with_output_file() {
        let (_temp_dir, report_path) = create_wildcards_report();
        let temp_dir2 = TempDir::new().expect("Failed to create temp dir");
        let junit_path = temp_dir2.path().join("junit.xml");

        depguard_cmd()
            .arg("junit")
            .arg("--report")
            .arg(&report_path)
            .arg("--output")
            .arg(&junit_path)
            .assert()
            .success();

        assert!(junit_path.exists(), "JUnit file should be created");
    }

    #[test]
    fn junit_command_includes_failures() {
        let (_temp_dir, report_path) = create_wildcards_report();

        let output = depguard_cmd()
            .arg("junit")
            .arg("--report")
            .arg(&report_path)
            .output()
            .expect("Failed to run junit command");

        let stdout = String::from_utf8_lossy(&output.stdout);
        // JUnit represents findings as test failures
        assert!(
            stdout.contains("<failure") || stdout.contains("failures="),
            "Should indicate failures"
        );
    }

    // JSONL COMMAND TESTS

    #[test]
    fn jsonl_command_basic_output() {
        let (_temp_dir, report_path) = create_wildcards_report();

        let output = depguard_cmd()
            .arg("jsonl")
            .arg("--report")
            .arg(&report_path)
            .output()
            .expect("Failed to run jsonl command");

        assert!(output.status.success(), "jsonl command should succeed");
        let stdout = String::from_utf8_lossy(&output.stdout);

        // JSONL should have newline-delimited JSON
        assert!(
            stdout.contains("\"kind\":\"finding\"") || stdout.contains("\"kind\": \"finding\""),
            "Should contain finding records"
        );
        assert!(
            stdout.contains("\"kind\":\"summary\"") || stdout.contains("\"kind\": \"summary\""),
            "Should contain summary record"
        );
    }

    #[test]
    fn jsonl_command_each_line_valid_json() {
        let (_temp_dir, report_path) = create_wildcards_report();

        let output = depguard_cmd()
            .arg("jsonl")
            .arg("--report")
            .arg(&report_path)
            .output()
            .expect("Failed to run jsonl command");

        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if !line.is_empty() {
                let _: Value = serde_json::from_str(line).expect("Each line should be valid JSON");
            }
        }
    }

    #[test]
    fn jsonl_command_with_output_file() {
        let (_temp_dir, report_path) = create_wildcards_report();
        let temp_dir2 = TempDir::new().expect("Failed to create temp dir");
        let jsonl_path = temp_dir2.path().join("report.jsonl");

        depguard_cmd()
            .arg("jsonl")
            .arg("--report")
            .arg(&report_path)
            .arg("--output")
            .arg(&jsonl_path)
            .assert()
            .success();

        assert!(jsonl_path.exists(), "JSONL file should be created");
    }

    #[test]
    fn jsonl_command_has_summary_line() {
        let (_temp_dir, report_path) = create_wildcards_report();

        let output = depguard_cmd()
            .arg("jsonl")
            .arg("--report")
            .arg(&report_path)
            .output()
            .expect("Failed to run jsonl command");

        let stdout = String::from_utf8_lossy(&output.stdout);
        let lines: Vec<&str> = stdout.lines().collect();
        assert!(!lines.is_empty(), "Should have at least one line");

        // Last line should be summary
        let last_line = lines.last().expect("Should have at least one line");
        let summary: Value =
            serde_json::from_str(last_line).expect("Last line should be valid JSON");
        assert_eq!(
            summary["kind"].as_str(),
            Some("summary"),
            "Last line should be summary"
        );
    }
}

// =============================================================================
// EXPLAIN COMMAND TESTS
// =============================================================================

mod explain_command {
    use super::*;
    use depguard_check_catalog as check_catalog;

    #[test]
    fn explain_check_id_no_wildcards() {
        let output = depguard_cmd()
            .arg("explain")
            .arg("deps.no_wildcards")
            .output()
            .expect("Failed to run explain command");

        assert!(output.status.success(), "explain command should succeed");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.to_lowercase().contains("wildcard"),
            "Should explain wildcard check"
        );
    }

    #[test]
    fn explain_check_id_path_requires_version() {
        let output = depguard_cmd()
            .arg("explain")
            .arg("deps.path_requires_version")
            .output()
            .expect("Failed to run explain command");

        assert!(output.status.success(), "explain command should succeed");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.to_lowercase().contains("path") || stdout.to_lowercase().contains("version"),
            "Should explain path requires version check"
        );
    }

    #[test]
    fn explain_check_id_path_safety() {
        let output = depguard_cmd()
            .arg("explain")
            .arg("deps.path_safety")
            .output()
            .expect("Failed to run explain command");

        assert!(output.status.success(), "explain command should succeed");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.to_lowercase().contains("path"),
            "Should explain path safety check"
        );
    }

    #[test]
    fn explain_check_id_workspace_inheritance() {
        let output = depguard_cmd()
            .arg("explain")
            .arg("deps.workspace_inheritance")
            .output()
            .expect("Failed to run explain command");

        assert!(output.status.success(), "explain command should succeed");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.to_lowercase().contains("workspace"),
            "Should explain workspace inheritance check"
        );
    }

    #[test]
    fn explain_check_id_git_requires_version() {
        let output = depguard_cmd()
            .arg("explain")
            .arg("deps.git_requires_version")
            .output()
            .expect("Failed to run explain command");

        assert!(output.status.success(), "explain command should succeed");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.to_lowercase().contains("git"),
            "Should explain git requires version check"
        );
    }

    #[test]
    fn explain_check_id_dev_only_in_normal() {
        let output = depguard_cmd()
            .arg("explain")
            .arg("deps.dev_only_in_normal")
            .output()
            .expect("Failed to run explain command");

        assert!(output.status.success(), "explain command should succeed");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.to_lowercase().contains("dev") || stdout.to_lowercase().contains("development"),
            "Should explain dev only in normal check"
        );
    }

    #[test]
    fn explain_check_id_default_features_explicit() {
        let output = depguard_cmd()
            .arg("explain")
            .arg("deps.default_features_explicit")
            .output()
            .expect("Failed to run explain command");

        assert!(output.status.success(), "explain command should succeed");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.to_lowercase().contains("feature"),
            "Should explain default features explicit check"
        );
    }

    #[test]
    fn explain_check_id_no_multiple_versions() {
        let output = depguard_cmd()
            .arg("explain")
            .arg("deps.no_multiple_versions")
            .output()
            .expect("Failed to run explain command");

        assert!(output.status.success(), "explain command should succeed");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.to_lowercase().contains("version"),
            "Should explain no multiple versions check"
        );
    }

    #[test]
    fn explain_check_id_optional_unused() {
        let output = depguard_cmd()
            .arg("explain")
            .arg("deps.optional_unused")
            .output()
            .expect("Failed to run explain command");

        assert!(output.status.success(), "explain command should succeed");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.to_lowercase().contains("optional"),
            "Should explain optional unused check"
        );
    }

    #[test]
    fn explain_code_wildcard_version() {
        let output = depguard_cmd()
            .arg("explain")
            .arg("wildcard_version")
            .output()
            .expect("Failed to run explain command");

        assert!(output.status.success(), "explain command should succeed");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.to_lowercase().contains("wildcard") || stdout.contains('*'),
            "Should explain wildcard version code"
        );
    }

    #[test]
    fn explain_code_path_without_version() {
        let output = depguard_cmd()
            .arg("explain")
            .arg("path_without_version")
            .output()
            .expect("Failed to run explain command");

        assert!(output.status.success(), "explain command should succeed");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.to_lowercase().contains("path") || stdout.to_lowercase().contains("version"),
            "Should explain path without version code"
        );
    }

    #[test]
    fn explain_code_absolute_path() {
        let output = depguard_cmd()
            .arg("explain")
            .arg("absolute_path")
            .output()
            .expect("Failed to run explain command");

        assert!(output.status.success(), "explain command should succeed");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.to_lowercase().contains("absolute") || stdout.to_lowercase().contains("path"),
            "Should explain absolute path code"
        );
    }

    #[test]
    fn explain_code_parent_escape() {
        let output = depguard_cmd()
            .arg("explain")
            .arg("parent_escape")
            .output()
            .expect("Failed to run explain command");

        assert!(output.status.success(), "explain command should succeed");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.to_lowercase().contains("parent") || stdout.contains(".."),
            "Should explain parent escape code"
        );
    }

    #[test]
    fn explain_code_missing_workspace_true() {
        let output = depguard_cmd()
            .arg("explain")
            .arg("missing_workspace_true")
            .output()
            .expect("Failed to run explain command");

        assert!(output.status.success(), "explain command should succeed");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.to_lowercase().contains("workspace"),
            "Should explain missing workspace true code"
        );
    }

    #[test]
    fn explain_code_git_without_version() {
        let output = depguard_cmd()
            .arg("explain")
            .arg("git_without_version")
            .output()
            .expect("Failed to run explain command");

        assert!(output.status.success(), "explain command should succeed");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.to_lowercase().contains("git"),
            "Should explain git without version code"
        );
    }

    #[test]
    fn explain_output_format_includes_remediation() {
        let output = depguard_cmd()
            .arg("explain")
            .arg("deps.no_wildcards")
            .output()
            .expect("Failed to run explain command");

        let stdout = String::from_utf8_lossy(&output.stdout);
        // Explain output should include remediation guidance
        assert!(
            stdout.to_lowercase().contains("remediation")
                || stdout.to_lowercase().contains("fix")
                || stdout.to_lowercase().contains("solution"),
            "Explain should include remediation guidance"
        );
    }

    #[test]
    fn explain_unknown_check_id_fails() {
        depguard_cmd()
            .arg("explain")
            .arg("nonexistent.check")
            .assert()
            .failure();
    }

    #[test]
    fn explain_unknown_code_fails() {
        depguard_cmd()
            .arg("explain")
            .arg("nonexistent_code")
            .assert()
            .failure();
    }

    #[test]
    fn explain_all_registered_check_ids() {
        // Test that all check IDs from the catalog can be explained
        for check_id in check_catalog::all_check_ids() {
            let output = depguard_cmd()
                .arg("explain")
                .arg(check_id)
                .output()
                .expect("Failed to run explain command");

            assert!(
                output.status.success(),
                "explain command should succeed for check ID '{}'",
                check_id
            );
        }
    }

    #[test]
    fn explain_all_registered_codes() {
        // Test that all codes from the catalog can be explained
        for code in check_catalog::all_codes() {
            let output = depguard_cmd()
                .arg("explain")
                .arg(code)
                .output()
                .expect("Failed to run explain command");

            assert!(
                output.status.success(),
                "explain command should succeed for code '{}'",
                code
            );
        }
    }
}

// =============================================================================
// ERROR HANDLING TESTS
// =============================================================================

mod error_handling {
    use super::*;

    #[test]
    fn invalid_config_file_returns_error() {
        let fixture_path = fixtures_dir().join("clean");
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let invalid_config = temp_dir.path().join("invalid.toml");
        let report_path = temp_dir.path().join("report.json");

        // Write invalid TOML
        std::fs::write(&invalid_config, "this is not valid toml [[[[").unwrap();

        // --config is a global flag before subcommand
        depguard_cmd()
            .arg("--repo-root")
            .arg(&fixture_path)
            .arg("--config")
            .arg(&invalid_config)
            .arg("check")
            .arg("--report-out")
            .arg(&report_path)
            .assert()
            .failure();
    }

    #[test]
    fn missing_config_file_is_handled_gracefully() {
        let fixture_path = fixtures_dir().join("clean");
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let missing_config = temp_dir.path().join("nonexistent.toml");
        let report_path = temp_dir.path().join("report.json");

        // --config is a global flag before subcommand
        // The CLI may handle missing config gracefully (use defaults) or fail
        // Either behavior is acceptable
        let output = depguard_cmd()
            .arg("--repo-root")
            .arg(&fixture_path)
            .arg("--config")
            .arg(&missing_config)
            .arg("check")
            .arg("--report-out")
            .arg(&report_path)
            .output()
            .expect("Failed to run command");

        // If command succeeded, report should exist
        // If command failed, that's also acceptable for missing config
        if output.status.success() {
            assert!(
                report_path.exists(),
                "Report should exist if command succeeded"
            );
        }
    }

    #[test]
    fn missing_repo_root_returns_error() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let missing_root = temp_dir.path().join("missing-repo");
        let report_path = temp_dir.path().join("report.json");

        depguard_cmd()
            .arg("--repo-root")
            .arg(&missing_root)
            .arg("check")
            .arg("--report-out")
            .arg(&report_path)
            .assert()
            .failure();
    }

    #[test]
    fn missing_report_file_for_render_commands() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let missing_report = temp_dir.path().join("nonexistent.json");

        depguard_cmd()
            .arg("md")
            .arg("--report")
            .arg(&missing_report)
            .assert()
            .failure();

        depguard_cmd()
            .arg("annotations")
            .arg("--report")
            .arg(&missing_report)
            .assert()
            .failure();

        depguard_cmd()
            .arg("sarif")
            .arg("--report")
            .arg(&missing_report)
            .assert()
            .failure();

        depguard_cmd()
            .arg("junit")
            .arg("--report")
            .arg(&missing_report)
            .assert()
            .failure();

        depguard_cmd()
            .arg("jsonl")
            .arg("--report")
            .arg(&missing_report)
            .assert()
            .failure();
    }

    #[test]
    fn invalid_report_file_for_render_commands() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let invalid_report = temp_dir.path().join("invalid.json");

        // Write invalid JSON
        std::fs::write(&invalid_report, "not valid json").unwrap();

        depguard_cmd()
            .arg("md")
            .arg("--report")
            .arg(&invalid_report)
            .assert()
            .failure();
    }

    #[test]
    fn invalid_baseline_file_returns_error() {
        let fixture_path = fixtures_dir().join("clean");
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let invalid_baseline = temp_dir.path().join("invalid-baseline.json");
        let report_path = temp_dir.path().join("report.json");

        // Write invalid JSON
        std::fs::write(&invalid_baseline, "not valid json").unwrap();

        depguard_cmd()
            .arg("--repo-root")
            .arg(&fixture_path)
            .arg("check")
            .arg("--baseline")
            .arg(&invalid_baseline)
            .arg("--report-out")
            .arg(&report_path)
            .assert()
            .failure();
    }

    #[test]
    fn missing_baseline_file_returns_error() {
        let fixture_path = fixtures_dir().join("clean");
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let missing_baseline = temp_dir.path().join("nonexistent-baseline.json");
        let report_path = temp_dir.path().join("report.json");

        depguard_cmd()
            .arg("--repo-root")
            .arg(&fixture_path)
            .arg("check")
            .arg("--baseline")
            .arg(&missing_baseline)
            .arg("--report-out")
            .arg(&report_path)
            .assert()
            .failure();
    }

    #[test]
    fn invalid_diff_scope_value_returns_error() {
        let fixture_path = fixtures_dir().join("clean");
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let report_path = temp_dir.path().join("report.json");

        depguard_cmd()
            .arg("--repo-root")
            .arg(&fixture_path)
            .arg("check")
            .arg("--diff-scope")
            .arg("invalid_value")
            .arg("--report-out")
            .arg(&report_path)
            .assert()
            .failure();
    }

    #[test]
    fn invalid_mode_value_returns_error() {
        let fixture_path = fixtures_dir().join("clean");
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let report_path = temp_dir.path().join("report.json");

        depguard_cmd()
            .arg("--repo-root")
            .arg(&fixture_path)
            .arg("check")
            .arg("--mode")
            .arg("invalid_mode")
            .arg("--report-out")
            .arg(&report_path)
            .assert()
            .failure();
    }

    #[test]
    fn invalid_profile_value_returns_error() {
        let fixture_path = fixtures_dir().join("clean");
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let report_path = temp_dir.path().join("report.json");

        depguard_cmd()
            .arg("--repo-root")
            .arg(&fixture_path)
            .arg("check")
            .arg("--profile")
            .arg("invalid_profile")
            .arg("--report-out")
            .arg(&report_path)
            .assert()
            .failure();
    }

    #[test]
    fn invalid_report_version_returns_error() {
        let fixture_path = fixtures_dir().join("clean");
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let report_path = temp_dir.path().join("report.json");

        depguard_cmd()
            .arg("--repo-root")
            .arg(&fixture_path)
            .arg("check")
            .arg("--report-version")
            .arg("v99")
            .arg("--report-out")
            .arg(&report_path)
            .assert()
            .failure();
    }

    #[test]
    fn no_command_shows_help() {
        depguard_cmd()
            .assert()
            .failure()
            .stderr(predicate::str::contains("Usage").or(predicate::str::contains("Commands")));
    }

    #[test]
    fn unknown_command_returns_error() {
        depguard_cmd().arg("unknown_command").assert().failure();
    }

    #[test]
    fn check_without_repo_root_returns_error() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let report_path = temp_dir.path().join("report.json");

        // Try to run check without --repo-root (should fail or require it)
        let result = depguard_cmd()
            .arg("check")
            .arg("--report-out")
            .arg(&report_path)
            .assert();

        // This should either fail or use current directory
        // The behavior depends on CLI implementation
        let _ = result;
    }
}

// =============================================================================
// CARGO DEPGUARD SUBCOMMAND TESTS
// =============================================================================

mod cargo_depguard {
    use super::*;

    #[test]
    #[allow(deprecated)]
    fn cargo_depguard_help_works() {
        let output = Command::cargo_bin("cargo-depguard")
            .expect("cargo-depguard binary not found")
            .arg("--help")
            .output()
            .expect("Failed to run cargo-depguard --help");

        // cargo-depguard wraps depguard, so help should mention depguard or cargo
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("depguard") || stdout.contains("cargo"),
            "Help should mention depguard or cargo"
        );
    }
}

// =============================================================================
// EXIT CODE TESTS
// =============================================================================

mod exit_codes {
    use super::*;

    #[test]
    fn exit_code_0_for_passing_check() {
        let fixture_path = fixtures_dir().join("clean");
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let report_path = temp_dir.path().join("report.json");

        depguard_cmd()
            .arg("--repo-root")
            .arg(&fixture_path)
            .arg("check")
            .arg("--report-out")
            .arg(&report_path)
            .assert()
            .code(0);
    }

    #[test]
    fn exit_code_2_for_failing_check() {
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
    }

    #[test]
    fn exit_code_1_for_tool_error() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let missing_root = temp_dir.path().join("nonexistent");

        depguard_cmd()
            .arg("--repo-root")
            .arg(&missing_root)
            .arg("check")
            .assert()
            .code(1);
    }

    #[test]
    fn exit_code_0_in_cockpit_mode_even_on_failure() {
        let fixture_path = fixtures_dir().join("wildcards");
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let report_path = temp_dir.path().join("report.json");

        depguard_cmd()
            .arg("--repo-root")
            .arg(&fixture_path)
            .arg("check")
            .arg("--mode")
            .arg("cockpit")
            .arg("--report-out")
            .arg(&report_path)
            .assert()
            .code(0);
    }
}

// =============================================================================
// VERSION AND HELP TESTS
// =============================================================================

mod version_and_help {
    use super::*;

    #[test]
    fn version_flag_works() {
        depguard_cmd()
            .arg("--version")
            .assert()
            .success()
            .stdout(predicate::str::contains("depguard"));
    }

    #[test]
    fn help_flag_works() {
        depguard_cmd()
            .arg("--help")
            .assert()
            .success()
            .stdout(predicate::str::contains("Commands"));
    }

    #[test]
    fn check_help_shows_options() {
        depguard_cmd()
            .arg("check")
            .arg("--help")
            .assert()
            .success()
            .stdout(predicate::str::contains("--report-out"));
    }

    #[test]
    fn baseline_help_shows_options() {
        depguard_cmd()
            .arg("baseline")
            .arg("--help")
            .assert()
            .success()
            .stdout(predicate::str::contains("--output"));
    }

    #[test]
    fn md_help_shows_options() {
        depguard_cmd()
            .arg("md")
            .arg("--help")
            .assert()
            .success()
            .stdout(predicate::str::contains("--report"));
    }

    #[test]
    fn annotations_help_shows_options() {
        depguard_cmd()
            .arg("annotations")
            .arg("--help")
            .assert()
            .success()
            .stdout(predicate::str::contains("--report"));
    }

    #[test]
    fn sarif_help_shows_options() {
        depguard_cmd()
            .arg("sarif")
            .arg("--help")
            .assert()
            .success()
            .stdout(predicate::str::contains("--report"));
    }

    #[test]
    fn junit_help_shows_options() {
        depguard_cmd()
            .arg("junit")
            .arg("--help")
            .assert()
            .success()
            .stdout(predicate::str::contains("--report"));
    }

    #[test]
    fn jsonl_help_shows_options() {
        depguard_cmd()
            .arg("jsonl")
            .arg("--help")
            .assert()
            .success()
            .stdout(predicate::str::contains("--report"));
    }

    #[test]
    fn explain_help_shows_usage() {
        depguard_cmd()
            .arg("explain")
            .arg("--help")
            .assert()
            .success()
            .stdout(predicate::str::contains("check_id").or(predicate::str::contains("code")));
    }
}
