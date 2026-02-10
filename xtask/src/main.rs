//! Developer tasks (schema generation, fixture updates, packaging).
//!
//! Keeping this separate avoids bloating the end-user CLI.

#![allow(unexpected_cfgs)]

use anyhow::{Context, bail};
use schemars::schema_for;
use std::fs;
use std::path::PathBuf;

/// Get the project root (parent of xtask directory).
fn project_root() -> PathBuf {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            // Fallback: assume we're in xtask dir or use current dir
            std::env::current_dir().expect("Cannot determine current directory")
        });

    // If we're in the xtask directory, go up one level
    if manifest_dir.ends_with("xtask") {
        manifest_dir
            .parent()
            .expect("xtask has no parent")
            .to_path_buf()
    } else {
        manifest_dir
    }
}

/// Get the schemas directory path.
fn schemas_dir() -> PathBuf {
    project_root().join("schemas")
}

/// Get the contracts/schemas directory path.
fn contracts_schemas_dir() -> PathBuf {
    project_root().join("contracts").join("schemas")
}

/// Get the contracts/fixtures directory path.
fn contracts_fixtures_dir() -> PathBuf {
    project_root().join("contracts").join("fixtures")
}

/// Schema definition with its target filename.
struct SchemaSpec {
    filename: &'static str,
    generate: fn() -> schemars::Schema,
}

/// Generate the DepguardReport schema.
fn generate_report_schema() -> schemars::Schema {
    schema_for!(depguard_types::DepguardReportV1)
}

/// Generate the DepguardReport v2 schema.
fn generate_report_schema_v2() -> schemars::Schema {
    schema_for!(depguard_types::DepguardReportV2)
}

/// Generate the DepguardConfigV1 schema.
fn generate_config_schema() -> schemars::Schema {
    schema_for!(depguard_settings::DepguardConfigV1)
}

/// List of schemas to generate.
/// Note: receipt.envelope.v1.json is vendored/external and not regenerated.
fn schema_specs() -> Vec<SchemaSpec> {
    vec![
        SchemaSpec {
            filename: "depguard.report.v1.json",
            generate: generate_report_schema,
        },
        SchemaSpec {
            filename: "depguard.report.v2.json",
            generate: generate_report_schema_v2,
        },
        SchemaSpec {
            filename: "depguard.config.v1.json",
            generate: generate_config_schema,
        },
    ]
}

/// Serialize a schema to pretty-printed JSON with trailing newline.
fn serialize_schema(schema: &schemars::Schema) -> anyhow::Result<String> {
    let mut json = serde_json::to_string_pretty(schema).context("Failed to serialize schema")?;
    json.push('\n');
    Ok(json)
}

/// Emit schemas to the schemas/ directory.
fn emit_schemas() -> anyhow::Result<()> {
    let dir = schemas_dir();

    // Ensure schemas directory exists
    if !dir.exists() {
        fs::create_dir_all(&dir).context("Failed to create schemas directory")?;
    }

    for spec in schema_specs() {
        let schema = (spec.generate)();
        let json = serialize_schema(&schema)?;
        let path = dir.join(spec.filename);

        fs::write(&path, &json)
            .with_context(|| format!("Failed to write schema to {}", path.display()))?;

        println!("Wrote {}", path.display());
    }

    println!("\nSchemas emitted successfully.");
    Ok(())
}

/// Validate that schemas in the repo match what would be generated.
/// Returns Ok(()) if all schemas match, Err otherwise.
fn validate_schemas() -> anyhow::Result<()> {
    let dir = schemas_dir();
    let mut all_match = true;
    let mut missing = Vec::new();
    let mut mismatched = Vec::new();

    for spec in schema_specs() {
        let path = dir.join(spec.filename);

        if !path.exists() {
            missing.push(spec.filename);
            all_match = false;
            continue;
        }

        let schema = (spec.generate)();
        let expected = serialize_schema(&schema)?;
        let actual = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read {}", path.display()))?;

        if expected != actual {
            mismatched.push(spec.filename);
            all_match = false;
        }
    }

    if all_match {
        println!("All schemas are up to date.");
        Ok(())
    } else {
        if !missing.is_empty() {
            eprintln!("Missing schemas:");
            for name in &missing {
                eprintln!("  - {}", name);
            }
        }
        if !mismatched.is_empty() {
            eprintln!("Schemas out of date:");
            for name in &mismatched {
                eprintln!("  - {}", name);
            }
        }
        eprintln!("\nRun `cargo xtask emit-schemas` to regenerate.");
        bail!("Schema validation failed")
    }
}

fn print_help() {
    eprintln!("xtask commands:");
    eprintln!("  help              Show this message");
    eprintln!("  emit-schemas      Generate JSON schemas from Rust types to schemas/");
    eprintln!("  validate-schemas  Check if schemas/ matches generated output (for CI)");
    eprintln!("  print-schema-ids  Print known schema IDs");
    eprintln!("  conform           Validate contract fixtures against sensor.report.v1 schema");
    eprintln!(
        "  conform-full      Full conformance: contract fixtures + depguard output validation"
    );
    eprintln!("  explain-coverage  Validate all check IDs and codes have explanations");
}

/// Token pattern for reason codes and verdict reasons.
fn is_valid_token(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_lowercase() => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

/// Check that a path is clean: no absolute paths, no `../`, forward slashes only.
fn is_clean_path(path: &str) -> bool {
    !(path.starts_with('/')
        || path.starts_with('\\')
        || path.contains("..")
        || path.contains('\\')
        // Reject Windows-style drive letters like C:
        || (path.len() >= 2 && path.as_bytes()[1] == b':'))
}

/// Validate sensor.report.v1 conformance.
///
/// This checks:
/// 1. Schema validation: contract fixtures validate against sensor.report.v1.json
/// 2. Path hygiene: no absolute paths, no `../`, forward slashes only
/// 3. Token hygiene: verdict.reasons[] and capabilities.*.reason match token pattern
fn conform() -> anyhow::Result<()> {
    let contracts_dir = contracts_schemas_dir();
    let sensor_schema_path = contracts_dir.join("sensor.report.v1.json");

    // Check schema file exists
    if !sensor_schema_path.exists() {
        bail!(
            "sensor.report.v1.json not found at {}\n\n\
            Run `cargo build` and ensure contracts/schemas/ is populated.",
            sensor_schema_path.display()
        );
    }

    println!("✓ sensor.report.v1.json schema exists");

    // Load and compile the schema
    let schema_content = fs::read_to_string(&sensor_schema_path)
        .with_context(|| format!("Failed to read {}", sensor_schema_path.display()))?;
    let schema_value: serde_json::Value = serde_json::from_str(&schema_content)
        .with_context(|| "Failed to parse sensor.report.v1.json as JSON")?;

    let compiled = jsonschema::draft7::new(&schema_value)
        .map_err(|e| anyhow::anyhow!("Failed to compile schema: {}", e))?;

    println!("✓ sensor.report.v1.json schema compiles");

    // Validate contract fixtures
    let fixtures_dir = contracts_fixtures_dir();
    if !fixtures_dir.exists() {
        bail!(
            "contracts/fixtures/ not found at {}\n\n\
            Create contract fixtures first.",
            fixtures_dir.display()
        );
    }

    let mut fixture_count = 0;
    let mut errors = Vec::new();

    for entry in fs::read_dir(&fixtures_dir).context("Failed to read contracts/fixtures/")? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().is_none_or(|ext| ext != "json") {
            continue;
        }

        let filename = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let content =
            fs::read_to_string(&path).with_context(|| format!("Failed to read {}", filename))?;
        let value: serde_json::Value = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse {} as JSON", filename))?;

        // 1. Schema validation
        for err in compiled.iter_errors(&value) {
            errors.push(format!("{}: schema validation: {}", filename, err));
        }

        // 2. Path hygiene
        if let Some(findings) = value.get("findings").and_then(|v| v.as_array()) {
            for (i, finding) in findings.iter().enumerate() {
                if let Some(loc) = finding.get("location")
                    && let Some(path_str) = loc.get("path").and_then(|v| v.as_str())
                    && !is_clean_path(path_str)
                {
                    errors.push(format!(
                                "{}: finding[{}].location.path '{}' is not clean (no absolute, no ../, forward slashes only)",
                                filename, i, path_str
                            ));
                }
            }
        }

        if let Some(artifacts) = value.get("artifacts").and_then(|v| v.as_array()) {
            for (i, artifact) in artifacts.iter().enumerate() {
                if let Some(path_str) = artifact.get("path").and_then(|v| v.as_str())
                    && !is_clean_path(path_str)
                {
                    errors.push(format!(
                        "{}: artifacts[{}].path '{}' is not clean",
                        filename, i, path_str
                    ));
                }
            }
        }

        // 3. Token hygiene — verdict.reasons[]
        if let Some(reasons) = value
            .get("verdict")
            .and_then(|v| v.get("reasons"))
            .and_then(|v| v.as_array())
        {
            for (i, reason) in reasons.iter().enumerate() {
                if let Some(s) = reason.as_str()
                    && !is_valid_token(s)
                {
                    errors.push(format!(
                        "{}: verdict.reasons[{}] '{}' is not a valid token",
                        filename, i, s
                    ));
                }
            }
        }

        // 3b. Token hygiene — capabilities.*.reason
        if let Some(caps) = value
            .get("run")
            .and_then(|v| v.get("capabilities"))
            .and_then(|v| v.as_object())
        {
            for (cap_name, cap_value) in caps {
                if let Some(reason) = cap_value.get("reason").and_then(|v| v.as_str())
                    && !is_valid_token(reason)
                {
                    errors.push(format!(
                        "{}: capabilities.{}.reason '{}' is not a valid token",
                        filename, cap_name, reason
                    ));
                }
            }
        }

        fixture_count += 1;
        println!("  ✓ {} validates", filename);
    }

    if fixture_count == 0 {
        bail!("No JSON fixtures found in {}", fixtures_dir.display());
    }

    if !errors.is_empty() {
        eprintln!("\nConformance errors:");
        for err in &errors {
            eprintln!("  - {}", err);
        }
        bail!("Conformance validation failed with {} errors", errors.len());
    }

    println!(
        "\n✓ All {} contract fixtures pass conformance checks!",
        fixture_count
    );
    Ok(())
}

struct RunOutput {
    success: bool,
    code: Option<i32>,
    stderr: String,
}

#[cfg(not(any(test, coverage)))]
fn run_depguard(
    depguard_bin: &PathBuf,
    fixture_dir: &PathBuf,
    report_out: &PathBuf,
) -> anyhow::Result<RunOutput> {
    let output = std::process::Command::new(depguard_bin)
        .args([
            "--repo-root",
            fixture_dir.to_str().unwrap_or_default(),
            "check",
            "--report-version",
            "sensor-v1",
            "--mode",
            "cockpit",
            "--report-out",
            report_out.to_str().unwrap_or_default(),
        ])
        .output()
        .with_context(|| {
            format!(
                "Failed to run depguard on fixture '{}'",
                fixture_dir.display()
            )
        })?;

    Ok(RunOutput {
        success: output.status.success(),
        code: output.status.code(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    })
}

#[allow(unexpected_cfgs)]
#[cfg(any(test, coverage))]
fn run_depguard(
    _depguard_bin: &PathBuf,
    fixture_dir: &PathBuf,
    report_out: &PathBuf,
) -> anyhow::Result<RunOutput> {
    let fixture_name = fixture_dir
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or_default();

    if fixture_name.contains("fail-status") {
        return Ok(RunOutput {
            success: false,
            code: Some(1),
            stderr: "simulated failure".to_string(),
        });
    }

    if fixture_name.contains("bad-report") {
        let report = serde_json::json!(["invalid"]);
        let json = serde_json::to_string(&report)?;
        fs::write(report_out, json)?;
    } else if !fixture_name.contains("missing-report") {
        let report = serde_json::json!({
            "schema": "sensor.report.v1",
            "tool": { "name": "depguard", "version": "0.0.0" },
            "run": { "started_at": "2025-01-01T00:00:00Z", "ended_at": "2025-01-01T00:00:00Z" },
            "verdict": { "status": "pass" },
            "findings": []
        });
        let json = serde_json::to_string(&report)?;
        fs::write(report_out, json)?;
    }

    Ok(RunOutput {
        success: true,
        code: Some(0),
        stderr: String::new(),
    })
}

/// Full conformance: contract fixtures + depguard binary output validation.
///
/// This extends `conform()` by also:
/// 1. Running the built depguard binary on test fixtures with `--report-version sensor-v1`
/// 2. Validating produced receipts against sensor.report.v1 schema
/// 3. Comparing against golden files (timestamp-normalized)
fn conform_full() -> anyhow::Result<()> {
    // First run the basic conformance checks
    conform()?;

    println!("\n--- Full conformance: depguard binary output ---\n");

    let contracts_dir = contracts_schemas_dir();
    let sensor_schema_path = contracts_dir.join("sensor.report.v1.json");
    let schema_content = fs::read_to_string(&sensor_schema_path)?;
    let schema_value: serde_json::Value = serde_json::from_str(&schema_content)?;
    let compiled = jsonschema::draft7::new(&schema_value)
        .map_err(|e| anyhow::anyhow!("Failed to compile schema: {}", e))?;

    // Find the depguard binary
    let depguard_bin = project_root().join("target").join("debug").join("depguard");

    #[cfg(target_os = "windows")]
    let depguard_bin = depguard_bin.with_extension("exe");

    if !depguard_bin.exists() {
        bail!(
            "depguard binary not found at {}.\n\
            Run `cargo build -p depguard-cli` first.",
            depguard_bin.display()
        );
    }

    let test_fixtures_dir = project_root().join("tests").join("fixtures");
    let mut errors = Vec::new();

    for entry in fs::read_dir(&test_fixtures_dir).context("Failed to read tests/fixtures/")? {
        let entry = entry?;
        let fixture_dir = entry.path();
        if !fixture_dir.is_dir() {
            continue;
        }

        // Only run on fixtures that have a Cargo.toml
        if !fixture_dir.join("Cargo.toml").exists() {
            continue;
        }

        let fixture_name = fixture_dir
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let temp_dir = tempfile::tempdir().context("Failed to create temp dir")?;
        let report_out = temp_dir.path().join("report.json");

        let output = run_depguard(&depguard_bin, &fixture_dir, &report_out)
            .with_context(|| format!("Failed to run depguard on fixture '{}'", fixture_name))?;

        // Cockpit mode must exit 0 when a receipt is written.
        // Exit 2 here would indicate a regression to standard-mode semantics.
        if !output.success {
            errors.push(format!(
                "fixture '{}': depguard exited with {:?}: {}",
                fixture_name,
                output.code,
                output.stderr
            ));
            continue;
        }

        if !report_out.exists() {
            errors.push(format!(
                "fixture '{}': no report output generated",
                fixture_name
            ));
            continue;
        }

        let report_content = fs::read_to_string(&report_out)?;
        let report_value: serde_json::Value = serde_json::from_str(&report_content)
            .with_context(|| format!("Failed to parse report for fixture '{}'", fixture_name))?;

        // Validate against schema
        for err in compiled.iter_errors(&report_value) {
            errors.push(format!(
                "fixture '{}': schema validation: {}",
                fixture_name, err
            ));
        }

        // Check golden file if it exists
        let golden_path = fixture_dir.join("expected.sensor-report.json");
        if golden_path.exists() {
            let golden_content = fs::read_to_string(&golden_path)?;
            let golden_value: serde_json::Value = serde_json::from_str(&golden_content)?;

            // Compare with timestamp normalization
            let normalized_report =
                depguard_test_util::normalize_nondeterministic(report_value.clone());
            let normalized_golden =
                depguard_test_util::normalize_nondeterministic(golden_value.clone());

            if normalized_report != normalized_golden {
                errors.push(format!(
                    "fixture '{}': output differs from golden file expected.sensor-report.json",
                    fixture_name
                ));
            } else {
                println!(
                    "  ✓ fixture '{}' matches golden sensor-v1 report",
                    fixture_name
                );
            }
        } else {
            println!(
                "  ✓ fixture '{}' produces valid sensor-v1 output (no golden file)",
                fixture_name
            );
        }
    }

    if !errors.is_empty() {
        eprintln!("\nFull conformance errors:");
        for err in &errors {
            eprintln!("  - {}", err);
        }
        bail!(
            "Full conformance validation failed with {} errors",
            errors.len()
        );
    }

    println!("\n✓ Full conformance checks passed!");
    Ok(())
}

/// Validate that all check IDs and codes have explanations.
fn explain_coverage() -> anyhow::Result<()> {
    let check_ids = depguard_types::explain::all_check_ids();
    let codes = depguard_types::explain::all_codes();

    explain_coverage_with(check_ids, codes, depguard_types::explain::lookup_explanation)
}

fn explain_coverage_with<F>(
    check_ids: &[&str],
    codes: &[&str],
    mut lookup: F,
) -> anyhow::Result<()>
where
    F: FnMut(&str) -> Option<depguard_types::explain::Explanation>,
{
    let mut errors = Vec::new();

    // Validate check IDs
    for check_id in check_ids {
        match lookup(check_id) {
            Some(exp) => {
                if exp.title.is_empty() {
                    errors.push(format!("Check ID '{}' has empty title", check_id));
                }
                if exp.description.is_empty() {
                    errors.push(format!("Check ID '{}' has empty description", check_id));
                }
                if exp.remediation.is_empty() {
                    errors.push(format!("Check ID '{}' has empty remediation", check_id));
                }
            }
            None => {
                errors.push(format!("Check ID '{}' has no explanation", check_id));
            }
        }
    }

    // Validate codes
    for code in codes {
        match lookup(code) {
            Some(exp) => {
                if exp.title.is_empty() {
                    errors.push(format!("Code '{}' has empty title", code));
                }
                if exp.description.is_empty() {
                    errors.push(format!("Code '{}' has empty description", code));
                }
                if exp.remediation.is_empty() {
                    errors.push(format!("Code '{}' has empty remediation", code));
                }
            }
            None => {
                errors.push(format!("Code '{}' has no explanation", code));
            }
        }
    }

    if errors.is_empty() {
        println!("✓ {} check IDs have explanations", check_ids.len());
        println!("✓ {} codes have explanations", codes.len());
        println!("\n✓ All explain coverage checks passed!");
        Ok(())
    } else {
        for error in &errors {
            eprintln!("  - {}", error);
        }
        bail!(
            "Explain coverage validation failed with {} errors",
            errors.len()
        )
    }
}

fn run_with_args(args: &[String]) -> anyhow::Result<()> {
    let cmd = args.get(1).map(|s| s.as_str()).unwrap_or("help");

    match cmd {
        "help" | "--help" | "-h" => {
            print_help();
            Ok(())
        }
        "emit-schemas" => emit_schemas(),
        "validate-schemas" => validate_schemas(),
        "conform" => conform(),
        "conform-full" => conform_full(),
        "explain-coverage" => explain_coverage(),
        "print-schema-ids" => {
            // List all schema IDs for reference
            println!("receipt.envelope.v1 (vendored, not generated)");
            println!("sensor.report.v1 (universal cockpit protocol)");
            for spec in schema_specs() {
                let name = spec.filename.trim_end_matches(".json");
                println!("{}", name);
            }
            Ok(())
        }
        other => bail!("unknown xtask command: {other}\n\nRun `cargo xtask help` for usage."),
    }
    .context("xtask failed")
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    run_with_args(&args)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::path::PathBuf;
    use std::sync::Mutex;
    use tempfile::TempDir;

    static ENV_MUTEX: Mutex<()> = Mutex::new(());

    fn with_temp_root<F: FnOnce(&PathBuf)>(f: F) {
        let _lock = ENV_MUTEX.lock().expect("env mutex");
        with_temp_root_unlocked(f);
    }

    fn with_temp_root_unlocked<F: FnOnce(&PathBuf)>(f: F) {
        let tmp = TempDir::new().expect("temp dir");
        let root = tmp.path().to_path_buf();
        fs::create_dir_all(root.join("xtask")).expect("create xtask dir");

        let old = std::env::var("CARGO_MANIFEST_DIR").ok();
        unsafe {
            std::env::set_var("CARGO_MANIFEST_DIR", root.join("xtask"));
        }

        f(&root);

        if let Some(val) = old {
            unsafe {
                std::env::set_var("CARGO_MANIFEST_DIR", val);
            }
        } else {
            unsafe {
                std::env::remove_var("CARGO_MANIFEST_DIR");
            }
        }
    }

    fn restore_manifest_dir(old: Option<String>) {
        if let Some(val) = old {
            unsafe {
                std::env::set_var("CARGO_MANIFEST_DIR", val);
            }
        } else {
            unsafe {
                std::env::remove_var("CARGO_MANIFEST_DIR");
            }
        }
    }

    fn write_schema_file(path: &PathBuf) {
        let schema = json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "type": "object"
        });
        let json = serde_json::to_string(&schema).expect("schema json");
        fs::write(path, json).expect("write schema");
    }

    fn setup_contracts(root: &PathBuf) -> PathBuf {
        let schemas_dir = root.join("contracts").join("schemas");
        let fixtures_dir = root.join("contracts").join("fixtures");
        fs::create_dir_all(&schemas_dir).expect("create schemas");
        fs::create_dir_all(&fixtures_dir).expect("create fixtures");
        write_schema_file(&schemas_dir.join("sensor.report.v1.json"));
        fixtures_dir
    }

    fn write_contract_fixture(fixtures_dir: &PathBuf, name: &str, value: serde_json::Value) {
        let content = serde_json::to_string(&value).expect("fixture json");
        fs::write(fixtures_dir.join(name), content).expect("write fixture");
    }

    fn write_test_fixture(root: &PathBuf, name: &str, golden: Option<serde_json::Value>) {
        let dir = root.join("tests").join("fixtures").join(name);
        fs::create_dir_all(&dir).expect("create fixture dir");
        fs::write(
            dir.join("Cargo.toml"),
            r#"[package]
name = "fixture"
version = "0.1.0"
"#,
        )
        .expect("write Cargo.toml");

        if let Some(golden_value) = golden {
            let content = serde_json::to_string(&golden_value).expect("golden json");
            fs::write(dir.join("expected.sensor-report.json"), content).expect("write golden");
        }
    }

    fn write_dummy_depguard_bin(root: &PathBuf) {
        let bin_dir = root.join("target").join("debug");
        fs::create_dir_all(&bin_dir).expect("create bin dir");
        let mut bin = bin_dir.join("depguard");
        #[cfg(target_os = "windows")]
        {
            bin = bin.with_extension("exe");
        }
        fs::write(bin, "stub").expect("write stub bin");
    }

    #[test]
    fn token_validation_rules() {
        assert!(is_valid_token("abc"));
        assert!(is_valid_token("abc_123"));
        assert!(!is_valid_token("Abc"));
        assert!(!is_valid_token("abc-def"));
        assert!(!is_valid_token(""));
    }

    #[test]
    fn clean_path_rules() {
        assert!(is_clean_path("crates/depguard/Cargo.toml"));
        assert!(!is_clean_path("/abs/path"));
        assert!(!is_clean_path("..\\escape"));
        assert!(!is_clean_path("../escape"));
        assert!(!is_clean_path("C:\\\\Code\\\\proj"));
    }

    #[test]
    fn schema_specs_include_expected_and_generate() {
        let names: Vec<&str> = schema_specs().iter().map(|s| s.filename).collect();
        assert!(names.contains(&"depguard.report.v1.json"));
        assert!(names.contains(&"depguard.report.v2.json"));
        assert!(names.contains(&"depguard.config.v1.json"));

        for spec in schema_specs() {
            let _schema = (spec.generate)();
        }
    }

    #[test]
    fn project_root_and_dirs_are_consistent() {
        let _lock = ENV_MUTEX.lock().expect("env mutex");
        let old = std::env::var("CARGO_MANIFEST_DIR").ok();
        let tmp = TempDir::new().expect("temp dir");
        let cases = [
            PathBuf::from(env!("CARGO_MANIFEST_DIR")),
            tmp.path().to_path_buf(),
        ];

        for manifest_dir in cases {
            unsafe {
                std::env::set_var("CARGO_MANIFEST_DIR", &manifest_dir);
            }

            let root = project_root();
            if manifest_dir.ends_with("xtask") {
                assert_eq!(
                    root,
                    manifest_dir.parent().expect("xtask parent").to_path_buf()
                );
            } else {
                assert_eq!(root, manifest_dir);
            }
            assert_eq!(schemas_dir(), root.join("schemas"));
            assert_eq!(
                contracts_schemas_dir(),
                root.join("contracts").join("schemas")
            );
            assert_eq!(
                contracts_fixtures_dir(),
                root.join("contracts").join("fixtures")
            );
        }

        restore_manifest_dir(old);
    }

    #[test]
    fn project_root_uses_manifest_dir_when_not_xtask() {
        let _lock = ENV_MUTEX.lock().expect("env mutex");
        let tmp = TempDir::new().expect("temp dir");
        let old = std::env::var("CARGO_MANIFEST_DIR").ok();
        unsafe {
            std::env::set_var("CARGO_MANIFEST_DIR", tmp.path());
        }

        let root = project_root();
        assert_eq!(root, tmp.path().to_path_buf());

        restore_manifest_dir(old);
    }

    #[test]
    fn serialize_schema_appends_newline() {
        let schema = generate_config_schema();
        let json = serialize_schema(&schema).expect("serialize schema");
        assert!(json.ends_with('\n'));
    }

    #[test]
    fn project_root_falls_back_to_current_dir() {
        let _lock = ENV_MUTEX.lock().expect("env mutex");
        let tmp = TempDir::new().expect("temp dir");
        let old = std::env::var("CARGO_MANIFEST_DIR").ok();
        unsafe {
            std::env::remove_var("CARGO_MANIFEST_DIR");
        }
        let old_cwd = std::env::current_dir().expect("cwd");
        std::env::set_current_dir(tmp.path()).expect("set cwd");

        let root = project_root();
        assert_eq!(root, tmp.path().to_path_buf());

        std::env::set_current_dir(old_cwd).expect("restore cwd");
        restore_manifest_dir(old);
    }

    #[test]
    fn with_temp_root_restores_missing_env() {
        let _lock = ENV_MUTEX.lock().expect("env mutex");
        let old = std::env::var("CARGO_MANIFEST_DIR").ok();
        unsafe {
            std::env::remove_var("CARGO_MANIFEST_DIR");
        }

        with_temp_root_unlocked(|_root| {
            assert!(std::env::var("CARGO_MANIFEST_DIR").is_ok());
        });

        assert!(std::env::var("CARGO_MANIFEST_DIR").is_err());
        restore_manifest_dir(old);
    }

    #[test]
    fn restore_manifest_dir_handles_some() {
        let _lock = ENV_MUTEX.lock().expect("env mutex");
        let original = std::env::var("CARGO_MANIFEST_DIR").ok();

        restore_manifest_dir(Some("temp-manifest".to_string()));
        assert_eq!(
            std::env::var("CARGO_MANIFEST_DIR").expect("env var"),
            "temp-manifest"
        );

        restore_manifest_dir(original);
    }

    #[test]
    fn restore_manifest_dir_handles_none() {
        let _lock = ENV_MUTEX.lock().expect("env mutex");
        let original = std::env::var("CARGO_MANIFEST_DIR").ok();

        restore_manifest_dir(None);
        assert!(std::env::var("CARGO_MANIFEST_DIR").is_err());

        restore_manifest_dir(original);
    }

    #[test]
    fn emit_schemas_writes_files() {
        with_temp_root(|root| {
            let schemas_dir = root.join("schemas");
            assert!(!schemas_dir.exists());

            emit_schemas().expect("emit schemas");
            for spec in schema_specs() {
                assert!(schemas_dir.join(spec.filename).exists());
            }
        });
    }

    #[test]
    fn emit_schemas_existing_dir() {
        with_temp_root(|root| {
            let schemas_dir = root.join("schemas");
            fs::create_dir_all(&schemas_dir).expect("schemas dir");

            emit_schemas().expect("emit schemas");
            for spec in schema_specs() {
                assert!(schemas_dir.join(spec.filename).exists());
            }
        });
    }

    #[test]
    fn validate_schemas_ok() {
        with_temp_root(|root| {
            let schemas_dir = root.join("schemas");
            fs::create_dir_all(&schemas_dir).expect("schemas dir");

            for spec in schema_specs() {
                let schema = (spec.generate)();
                let json = serialize_schema(&schema).expect("schema json");
                fs::write(schemas_dir.join(spec.filename), json).expect("write schema");
            }

            validate_schemas().expect("validate schemas");
        });
    }

    #[test]
    fn validate_schemas_reports_missing_only() {
        with_temp_root(|root| {
            let schemas_dir = root.join("schemas");
            fs::create_dir_all(&schemas_dir).expect("schemas dir");

            let err = validate_schemas().unwrap_err();
            assert!(err.to_string().contains("Schema validation failed"));
        });
    }

    #[test]
    fn validate_schemas_reports_mismatched_only() {
        with_temp_root(|root| {
            let schemas_dir = root.join("schemas");
            fs::create_dir_all(&schemas_dir).expect("schemas dir");

            for spec in schema_specs() {
                let schema = (spec.generate)();
                let json = serialize_schema(&schema).expect("schema json");
                fs::write(schemas_dir.join(spec.filename), json).expect("write schema");
            }

            let specs = schema_specs();
            let spec = specs.first().expect("spec");
            fs::write(schemas_dir.join(spec.filename), "bad").expect("write bad schema");

            let err = validate_schemas().unwrap_err();
            assert!(err.to_string().contains("Schema validation failed"));
        });
    }

    #[test]
    fn validate_schemas_reports_missing_and_mismatch() {
        with_temp_root(|root| {
            let schemas_dir = root.join("schemas");
            fs::create_dir_all(&schemas_dir).expect("schemas dir");

            let specs = schema_specs();
            let spec = specs.first().expect("spec");
            fs::write(schemas_dir.join(spec.filename), "bad").expect("write bad schema");

            let err = validate_schemas().unwrap_err();
            assert!(err.to_string().contains("Schema validation failed"));
        });
    }

    #[test]
    fn print_help_outputs() {
        print_help();
    }

    #[test]
    fn conform_success_with_minimal_fixture() {
        with_temp_root(|root| {
            let fixtures_dir = setup_contracts(root);
            let fixture = json!({
                "findings": [
                    { "location": { "path": "Cargo.toml" } }
                ],
                "artifacts": [
                    { "path": "artifacts/report.json" }
                ],
                "verdict": { "reasons": ["ok_reason"] },
                "run": {
                    "capabilities": {
                        "git": { "reason": "ok_reason" }
                    }
                }
            });
            write_contract_fixture(&fixtures_dir, "good.json", fixture);

            conform().expect("conform");
        });
    }

    #[test]
    fn conform_missing_schema_errors() {
        with_temp_root(|root| {
            let fixtures_dir = root.join("contracts").join("fixtures");
            fs::create_dir_all(&fixtures_dir).expect("fixtures dir");

            let err = conform().unwrap_err();
            assert!(err.to_string().contains("sensor.report.v1.json not found"));
        });
    }

    #[test]
    fn conform_missing_fixtures_dir_errors() {
        with_temp_root(|root| {
            let schemas_dir = root.join("contracts").join("schemas");
            fs::create_dir_all(&schemas_dir).expect("schemas dir");
            write_schema_file(&schemas_dir.join("sensor.report.v1.json"));

            let err = conform().unwrap_err();
            assert!(err.to_string().contains("contracts/fixtures/ not found"));
        });
    }

    #[test]
    fn conform_no_json_fixtures() {
        with_temp_root(|root| {
            let fixtures_dir = setup_contracts(root);
            fs::write(fixtures_dir.join("README.txt"), "ignore").expect("write non-json");
            let err = conform().unwrap_err();
            assert!(err.to_string().contains("No JSON fixtures"));
        });
    }

    #[test]
    fn conform_reports_schema_errors() {
        with_temp_root(|root| {
            let fixtures_dir = setup_contracts(root);
            fs::write(fixtures_dir.join("bad.json"), "[1,2]").expect("write bad fixture");

            let err = conform().unwrap_err();
            assert!(err.to_string().contains("Conformance validation failed"));
        });
    }

    #[test]
    fn conform_reports_errors_on_invalid_fixture() {
        with_temp_root(|root| {
            let fixtures_dir = setup_contracts(root);
            let fixture = json!({
                "findings": [
                    { "location": { "path": "/abs/path" } }
                ],
                "artifacts": [
                    { "path": "/abs/path" }
                ],
                "verdict": { "reasons": ["BadToken"] },
                "run": {
                    "capabilities": {
                        "git": { "reason": "BadToken" }
                    }
                }
            });
            write_contract_fixture(&fixtures_dir, "bad.json", fixture);

            let err = conform().unwrap_err();
            assert!(err.to_string().contains("Conformance validation failed"));
        });
    }

    #[test]
    fn conform_full_success_with_and_without_golden() {
        with_temp_root(|root| {
            let fixtures_dir = setup_contracts(root);
            let fixture = json!({
                "findings": [],
                "verdict": { "reasons": ["ok_reason"] },
                "run": {
                    "capabilities": {
                        "git": { "reason": "ok_reason" }
                    }
                }
            });
            write_contract_fixture(&fixtures_dir, "good.json", fixture);

            write_dummy_depguard_bin(root);

            let report = json!({
                "schema": "sensor.report.v1",
                "tool": { "name": "depguard", "version": "0.0.0" },
                "run": { "started_at": "2025-01-01T00:00:00Z", "ended_at": "2025-01-01T00:00:00Z" },
                "verdict": { "status": "pass" },
                "findings": []
            });

            write_test_fixture(root, "ok-golden", Some(report.clone()));
            write_test_fixture(root, "ok-no-golden", None);

            conform_full().expect("conform full");
        });
    }

    #[test]
    fn conform_full_errors_when_binary_missing() {
        with_temp_root(|root| {
            let fixtures_dir = setup_contracts(root);
            let fixture = json!({
                "findings": [],
                "verdict": { "reasons": ["ok_reason"] }
            });
            write_contract_fixture(&fixtures_dir, "good.json", fixture);
            write_test_fixture(root, "ok", None);

            let err = conform_full().unwrap_err();
            assert!(err.to_string().contains("depguard binary not found"));
        });
    }

    #[test]
    fn conform_full_reports_errors_for_failures() {
        with_temp_root(|root| {
            let fixtures_dir = setup_contracts(root);
            let fixture = json!({
                "findings": [],
                "verdict": { "reasons": ["ok_reason"] }
            });
            write_contract_fixture(&fixtures_dir, "good.json", fixture);

            write_dummy_depguard_bin(root);

            write_test_fixture(root, "fail-status", None);
            write_test_fixture(root, "missing-report", None);
            write_test_fixture(root, "bad-report", None);
            write_test_fixture(root, "mismatch-golden", Some(json!({ "schema": "different" })));

            let test_fixtures_dir = root.join("tests").join("fixtures");
            fs::write(test_fixtures_dir.join("README.txt"), "ignore").expect("write file");
            fs::create_dir_all(test_fixtures_dir.join("empty")).expect("create empty dir");

            let err = conform_full().unwrap_err();
            assert!(err.to_string().contains("Full conformance validation failed"));
        });
    }

    #[test]
    fn explain_coverage_ok() {
        explain_coverage().expect("explain coverage");
    }

    #[test]
    fn explain_coverage_error_path() {
        let check_ids = ["check.one", "check.none"];
        let codes = ["code.one", "code.empty"];
        let result = explain_coverage_with(&check_ids, &codes, |id| {
            match id {
                "check.one" | "code.empty" => Some(depguard_types::explain::Explanation {
                    title: "",
                    description: "",
                    remediation: "",
                    examples: depguard_types::explain::ExamplePair { before: "", after: "" },
                }),
                _ => None,
            }
        });
        assert!(result.is_err());
    }

    #[test]
    fn run_with_args_help_and_unknown() {
        run_with_args(&vec!["xtask".to_string(), "help".to_string()]).expect("help");
        let err = run_with_args(&vec!["xtask".to_string(), "nope".to_string()]).unwrap_err();
        assert!(err.to_string().contains("unknown xtask command"));
    }

    #[test]
    fn run_with_args_emit_and_validate_schemas() {
        with_temp_root(|_root| {
            run_with_args(&vec!["xtask".to_string(), "emit-schemas".to_string()])
                .expect("emit");
            run_with_args(&vec!["xtask".to_string(), "validate-schemas".to_string()])
                .expect("validate");
        });
    }

    #[test]
    fn run_with_args_conform_and_conform_full() {
        with_temp_root(|root| {
            let fixtures_dir = setup_contracts(root);
            let fixture = json!({
                "findings": [],
                "verdict": { "reasons": ["ok_reason"] }
            });
            write_contract_fixture(&fixtures_dir, "good.json", fixture);

            write_dummy_depguard_bin(root);
            write_test_fixture(root, "ok", None);

            run_with_args(&vec!["xtask".to_string(), "conform".to_string()])
                .expect("conform");
            run_with_args(&vec!["xtask".to_string(), "conform-full".to_string()])
                .expect("conform-full");
        });
    }

    #[test]
    fn run_with_args_print_schema_ids_and_explain() {
        run_with_args(&vec!["xtask".to_string(), "print-schema-ids".to_string()])
            .expect("print-schema-ids");
        run_with_args(&vec!["xtask".to_string(), "explain-coverage".to_string()])
            .expect("explain-coverage");
    }
}
