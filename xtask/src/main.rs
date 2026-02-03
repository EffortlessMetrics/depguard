//! Developer tasks (schema generation, fixture updates, packaging).
//!
//! Keeping this separate avoids bloating the end-user CLI.

use anyhow::{bail, Context};
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

/// Schema definition with its target filename.
struct SchemaSpec {
    filename: &'static str,
    generate: fn() -> schemars::schema::RootSchema,
}

/// Generate the DepguardReport schema.
fn generate_report_schema() -> schemars::schema::RootSchema {
    schema_for!(depguard_types::DepguardReport)
}

/// Generate the DepguardConfigV1 schema.
fn generate_config_schema() -> schemars::schema::RootSchema {
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
            filename: "depguard.config.v1.json",
            generate: generate_config_schema,
        },
    ]
}

/// Serialize a schema to pretty-printed JSON with trailing newline.
fn serialize_schema(schema: &schemars::schema::RootSchema) -> anyhow::Result<String> {
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
        "print-schema-ids" => {
            // List all schema IDs for reference
            println!("receipt.envelope.v1 (vendored, not generated)");
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
