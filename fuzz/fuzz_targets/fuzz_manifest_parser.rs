//! Fuzz target for manifest parsing with complex TOML content.
//!
//! Goal: The manifest parser should **never panic** on any input.
//! It may return errors for invalid TOML, but panics are unacceptable.
//!
//! Run with:
//! ```bash
//! cargo +nightly fuzz run fuzz_manifest_parser
//! ```

#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;

/// Structured input for manifest parser fuzzing.
#[derive(Arbitrary, Debug)]
struct ManifestParserInput {
    /// Raw TOML content for the manifest
    toml_content: String,
    /// Whether to test as root manifest (with workspace dependencies)
    is_root: bool,
}

/// Structured input for complex feature configurations.
#[derive(Arbitrary, Debug)]
#[allow(dead_code)]
struct FeatureConfig {
    feature_name: String,
    /// Feature dependencies (other features or crate features)
    dependencies: Vec<String>,
}

/// Structured input for target-specific dependencies.
#[derive(Arbitrary, Debug)]
#[allow(dead_code)]
struct TargetDepInput {
    target_spec: String,
    dependency_name: String,
    version: String,
}

fuzz_target!(|input: ManifestParserInput| {
    // Limit input size to avoid OOM and keep fuzzing fast
    if input.toml_content.len() > 1_048_576 {
        return;
    }

    // Only test valid UTF-8 (already guaranteed by String type)
    let text = &input.toml_content;

    // Test root manifest parsing - should never panic
    if input.is_root {
        let _ = depguard_repo::fuzz::parse_root_manifest(text);
    } else {
        // Test member manifest parsing - should never panic
        let _ = depguard_repo::fuzz::parse_member_manifest(text);
    }
});

/// Additional fuzz target for feature parsing edge cases.
/// This tests complex feature configurations that might cause parsing issues.
#[cfg(test)]
mod tests {
    use super::*;

    fn build_manifest_with_features(features: &[FeatureConfig]) -> String {
        let feature_lines: Vec<String> = features
            .iter()
            .map(|f| {
                let deps: String = f
                    .dependencies
                    .iter()
                    .map(|d| format!("\"{}\"", d.escape_default()))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{} = [{}]", f.feature_name, deps)
            })
            .collect();

        format!(
            r#"[package]
name = "test"
version = "0.1.0"

[features]
{}
"#,
            feature_lines.join("\n")
        )
    }

    fn build_manifest_with_target_deps(target_deps: &[TargetDepInput]) -> String {
        let target_lines: Vec<String> = target_deps
            .iter()
            .map(|t| {
                format!(
                    r#"[target.'{}'.dependencies]
{} = "{}"
"#,
                    t.target_spec, t.dependency_name, t.version
                )
            })
            .collect();

        format!(
            r#"[package]
name = "test"
version = "0.1.0"

{}
"#,
            target_lines.join("\n")
        )
    }
}
