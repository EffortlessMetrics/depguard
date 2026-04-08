//! Fuzz target for workspace discovery logic.
//!
//! Goal: The workspace discovery should **never panic** on any input.
//! It may return errors for invalid configurations, but panics are unacceptable.
//!
//! Run with:
//! ```bash
//! cargo +nightly fuzz run fuzz_workspace_discovery
//! ```

#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;

/// Structured input for workspace discovery fuzzing.
#[derive(Arbitrary, Debug)]
struct WorkspaceDiscoveryInput {
    /// Root manifest content (Cargo.toml)
    root_manifest: String,
    /// Member manifest contents (path -> content)
    member_manifests: Vec<(String, String)>,
    /// Member glob patterns to test
    member_patterns: Vec<String>,
    /// Exclude glob patterns to test
    exclude_patterns: Vec<String>,
}

fuzz_target!(|input: WorkspaceDiscoveryInput| {
    // Limit input size to avoid OOM and keep fuzzing fast
    if input.root_manifest.len() > 65_536 {
        return;
    }
    if input.member_manifests.len() > 50 {
        return;
    }
    if input.member_patterns.len() > 20 || input.exclude_patterns.len() > 20 {
        return;
    }

    // Filter out excessively long strings
    let member_manifests: Vec<(String, String)> = input
        .member_manifests
        .into_iter()
        .filter(|(path, content)| {
            path.len() <= 512 && content.len() <= 65_536 && !path.contains('\0')
        })
        .take(50)
        .collect();

    let member_patterns: Vec<String> = input
        .member_patterns
        .into_iter()
        .filter(|p| p.len() <= 256 && !p.contains('\0'))
        .take(20)
        .collect();

    let exclude_patterns: Vec<String> = input
        .exclude_patterns
        .into_iter()
        .filter(|p| p.len() <= 256 && !p.contains('\0'))
        .take(20)
        .collect();

    // Test glob compilation and matching - should never panic
    let _ = depguard_repo::fuzz::expand_globs(&member_patterns, &[]);
    let _ = depguard_repo::fuzz::expand_globs(&exclude_patterns, &[]);

    // Test parsing root manifest with workspace configuration
    let _ = depguard_repo::fuzz::parse_root_manifest(&input.root_manifest);

    // Test parsing member manifests
    for (_path, content) in &member_manifests {
        let _ = depguard_repo::fuzz::parse_member_manifest(content);
    }

    // Test nested workspace detection by constructing manifest with nested members
    let nested_manifest = format!(
        r#"[workspace]
members = [{}]
exclude = [{}]
"#,
        member_patterns
            .iter()
            .map(|p| format!(r#""{}""#, p.escape_default()))
            .collect::<Vec<_>>()
            .join(", "),
        exclude_patterns
            .iter()
            .map(|p| format!(r#""{}""#, p.escape_default()))
            .collect::<Vec<_>>()
            .join(", ")
    );
    let _ = depguard_repo::fuzz::parse_root_manifest(&nested_manifest);
});
