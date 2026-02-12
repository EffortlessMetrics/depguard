//! Integration tests for cockpit mode and sensor.report.v1 output.

use std::process::Command;

/// Helper to get the depguard binary path.
fn depguard_bin() -> std::path::PathBuf {
    // Find the target directory
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::env::current_dir().unwrap());

    // Go up to workspace root and find target/debug/depguard
    let workspace_root = manifest_dir.parent().unwrap().parent().unwrap();
    workspace_root.join("target").join("debug").join("depguard")
}

/// Helper to get the fixtures directory.
fn fixtures_dir() -> std::path::PathBuf {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::env::current_dir().unwrap());

    manifest_dir
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tests")
        .join("fixtures")
}

#[test]
fn cockpit_mode_exits_zero_on_policy_failure() {
    // Run against wildcards fixture (which fails policy) with --mode cockpit
    let fixture = fixtures_dir().join("wildcards");
    let temp_dir = tempfile::tempdir().unwrap();
    let report_out = temp_dir.path().join("report.json");

    let output = Command::new(depguard_bin())
        .args([
            "--repo-root",
            fixture.to_str().unwrap(),
            "check",
            "--mode",
            "cockpit",
            "--report-out",
            report_out.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to run depguard");

    // In cockpit mode, should exit 0 even when policy fails
    assert!(
        output.status.success(),
        "Expected exit 0 in cockpit mode, got {:?}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    // Verify report was written
    assert!(report_out.exists(), "Report should be written");

    // Verify verdict is fail (policy violation)
    let report_content = std::fs::read_to_string(&report_out).unwrap();
    let report: serde_json::Value = serde_json::from_str(&report_content).unwrap();
    assert_eq!(
        report["verdict"]["status"].as_str(),
        Some("fail"),
        "Verdict should be 'fail' for wildcards fixture"
    );
}

#[test]
fn standard_mode_exits_nonzero_on_policy_failure() {
    // Run against wildcards fixture (which fails policy) with --mode standard
    let fixture = fixtures_dir().join("wildcards");
    let temp_dir = tempfile::tempdir().unwrap();
    let report_out = temp_dir.path().join("report.json");

    let output = Command::new(depguard_bin())
        .args([
            "--repo-root",
            fixture.to_str().unwrap(),
            "check",
            "--mode",
            "standard",
            "--report-out",
            report_out.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to run depguard");

    // In standard mode, should exit 2 on policy failure
    assert_eq!(
        output.status.code(),
        Some(2),
        "Expected exit 2 in standard mode on policy failure\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn sensor_v1_schema_output() {
    // Run with --report-version sensor-v1
    let fixture = fixtures_dir().join("clean");
    let temp_dir = tempfile::tempdir().unwrap();
    let report_out = temp_dir.path().join("report.json");

    let output = Command::new(depguard_bin())
        .args([
            "--repo-root",
            fixture.to_str().unwrap(),
            "check",
            "--report-version",
            "sensor-v1",
            "--report-out",
            report_out.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to run depguard");

    assert!(
        output.status.success(),
        "Expected exit 0 for clean fixture\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Verify report was written
    assert!(report_out.exists(), "Report should be written");

    // Verify schema is sensor.report.v1
    let report_content = std::fs::read_to_string(&report_out).unwrap();
    let report: serde_json::Value = serde_json::from_str(&report_content).unwrap();

    assert_eq!(
        report["schema"].as_str(),
        Some("sensor.report.v1"),
        "Schema should be 'sensor.report.v1'"
    );

    // Verify capabilities block is present
    assert!(
        report["run"]["capabilities"].is_object(),
        "capabilities block should be present in sensor.report.v1 output"
    );

    // Verify capability structure
    let capabilities = &report["run"]["capabilities"];
    assert!(
        capabilities["git"].is_object(),
        "git capability should be present"
    );
    assert!(
        capabilities["config"].is_object(),
        "config capability should be present"
    );

    // Verify capability status values are valid
    let git_status = capabilities["git"]["status"].as_str().unwrap();
    assert!(
        ["available", "missing", "degraded"].contains(&git_status),
        "git status should be a valid CapabilityAvailability value"
    );
}

#[test]
fn sensor_v1_with_config_has_available_config_capability() {
    // Run with a config file present
    let fixture = fixtures_dir().join("wildcards"); // Has depguard.toml
    let temp_dir = tempfile::tempdir().unwrap();
    let report_out = temp_dir.path().join("report.json");

    // Check if there's a depguard.toml in the fixture
    let config_path = fixture.join("depguard.toml");

    let mut args = vec![
        "--repo-root".to_string(),
        fixture.to_str().unwrap().to_string(),
        "check".to_string(),
        "--report-version".to_string(),
        "sensor-v1".to_string(),
        "--mode".to_string(),
        "cockpit".to_string(),
        "--report-out".to_string(),
        report_out.to_str().unwrap().to_string(),
    ];

    // If config exists, explicitly specify it
    if config_path.exists() {
        args.push("--config".to_string());
        args.push(config_path.to_str().unwrap().to_string());
    }

    let output = Command::new(depguard_bin())
        .args(&args)
        .output()
        .expect("Failed to run depguard");

    assert!(
        output.status.success(),
        "Expected exit 0 in cockpit mode\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let report_content = std::fs::read_to_string(&report_out).unwrap();
    let report: serde_json::Value = serde_json::from_str(&report_content).unwrap();

    // If config file exists, config capability should be available
    if config_path.exists() {
        assert_eq!(
            report["run"]["capabilities"]["config"]["status"].as_str(),
            Some("available"),
            "Config capability should be 'available' when config file is present"
        );
    }
}

#[test]
fn default_mode_is_standard() {
    // Run without --mode flag against failing fixture
    let fixture = fixtures_dir().join("wildcards");
    let temp_dir = tempfile::tempdir().unwrap();
    let report_out = temp_dir.path().join("report.json");

    let output = Command::new(depguard_bin())
        .args([
            "--repo-root",
            fixture.to_str().unwrap(),
            "check",
            "--report-out",
            report_out.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to run depguard");

    // Default mode is standard, so should exit 2 on policy failure
    assert_eq!(
        output.status.code(),
        Some(2),
        "Default mode should be standard (exit 2 on policy failure)\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
