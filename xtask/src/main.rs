//! Developer tasks (schema generation, fixture updates, packaging).
//!
//! Keeping this separate avoids bloating the end-user CLI.

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
    let mut schema_value: serde_json::Value = serde_json::from_str(&schema_content)
        .with_context(|| "Failed to parse sensor.report.v1.json as JSON")?;
    // Remove $id since it's a logical identifier, not a resolvable URL.
    // The jsonschema crate tries to resolve $id as a URI.
    if let Some(obj) = schema_value.as_object_mut() {
        obj.remove("$id");
    }

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
                if let Some(loc) = finding.get("location") {
                    if let Some(path_str) = loc.get("path").and_then(|v| v.as_str()) {
                        if !is_clean_path(path_str) {
                            errors.push(format!(
                                "{}: finding[{}].location.path '{}' is not clean (no absolute, no ../, forward slashes only)",
                                filename, i, path_str
                            ));
                        }
                    }
                }
            }
        }

        if let Some(artifacts) = value.get("artifacts").and_then(|v| v.as_array()) {
            for (i, artifact) in artifacts.iter().enumerate() {
                if let Some(path_str) = artifact.get("path").and_then(|v| v.as_str()) {
                    if !is_clean_path(path_str) {
                        errors.push(format!(
                            "{}: artifacts[{}].path '{}' is not clean",
                            filename, i, path_str
                        ));
                    }
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
                if let Some(s) = reason.as_str() {
                    if !is_valid_token(s) {
                        errors.push(format!(
                            "{}: verdict.reasons[{}] '{}' is not a valid token",
                            filename, i, s
                        ));
                    }
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
                if let Some(reason) = cap_value.get("reason").and_then(|v| v.as_str()) {
                    if !is_valid_token(reason) {
                        errors.push(format!(
                            "{}: capabilities.{}.reason '{}' is not a valid token",
                            filename, cap_name, reason
                        ));
                    }
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
    let mut schema_value: serde_json::Value = serde_json::from_str(&schema_content)?;
    if let Some(obj) = schema_value.as_object_mut() {
        obj.remove("$id");
    }
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

        let output = std::process::Command::new(&depguard_bin)
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
            .with_context(|| format!("Failed to run depguard on fixture '{}'", fixture_name))?;

        // Cockpit mode must exit 0 when a receipt is written.
        // Exit 2 here would indicate a regression to standard-mode semantics.
        if !output.status.success() {
            errors.push(format!(
                "fixture '{}': depguard exited with {:?}: {}",
                fixture_name,
                output.status.code(),
                String::from_utf8_lossy(&output.stderr)
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
            let normalized_report = normalize_timestamps(&report_value);
            let normalized_golden = normalize_timestamps(&golden_value);

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

/// Normalize timestamps in a JSON value for comparison.
fn normalize_timestamps(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => {
            let mut new_map = serde_json::Map::new();
            for (k, v) in map {
                if k == "started_at" || k == "ended_at" {
                    new_map.insert(k.clone(), serde_json::Value::String("__TIMESTAMP__".into()));
                } else if k == "duration_ms" {
                    new_map.insert(k.clone(), serde_json::Value::Number(0.into()));
                } else {
                    new_map.insert(k.clone(), normalize_timestamps(v));
                }
            }
            serde_json::Value::Object(new_map)
        }
        serde_json::Value::Array(arr) => {
            serde_json::Value::Array(arr.iter().map(normalize_timestamps).collect())
        }
        other => other.clone(),
    }
}

/// Validate that all check IDs and codes have explanations.
fn explain_coverage() -> anyhow::Result<()> {
    let check_ids = depguard_types::explain::all_check_ids();
    let codes = depguard_types::explain::all_codes();

    let mut errors = Vec::new();

    // Validate check IDs
    for check_id in check_ids {
        match depguard_types::explain::lookup_explanation(check_id) {
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
        match depguard_types::explain::lookup_explanation(code) {
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

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
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
