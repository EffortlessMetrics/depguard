//! Integration tests for manifest discovery.
//!
//! These tests verify that workspace discovery produces a stable, deterministic
//! ordering of manifests regardless of filesystem traversal order.

use camino::Utf8PathBuf;
use depguard_repo::discover_manifests;
use std::path::PathBuf;

/// Get the path to the test fixtures directory (repo root / tests / fixtures).
fn fixtures_dir() -> Utf8PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // crates/depguard-repo -> crates -> repo root
    let repo_root = manifest_dir
        .parent()
        .expect("depguard-repo should have parent (crates)")
        .parent()
        .expect("crates should have parent (repo root)");
    Utf8PathBuf::from_path_buf(repo_root.join("tests").join("fixtures"))
        .expect("fixture path should be valid UTF-8")
}

/// Golden snapshot test for discovered manifest ordering.
///
/// This test verifies that:
/// 1. Manifests are discovered in a deterministic order
/// 2. The order is lexicographic (sorted by path string)
/// 3. The root Cargo.toml comes first
/// 4. Member manifests follow in sorted order
#[test]
fn manifest_ordering_is_deterministic() {
    let fixture_path = fixtures_dir().join("manifest_ordering");

    // Discover manifests
    let manifests = discover_manifests(&fixture_path).expect("discovery should succeed");

    // Convert to strings for comparison
    let actual: Vec<&str> = manifests.iter().map(|p| p.as_str()).collect();

    // Load expected ordering from golden file
    let expected_path = fixture_path.join("expected.manifest_order.json");
    let expected_content =
        std::fs::read_to_string(&expected_path).expect("should read expected manifest order");
    let expected: Vec<String> =
        serde_json::from_str(&expected_content).expect("should parse expected manifest order");

    assert_eq!(
        actual, expected,
        "Manifest ordering should match golden snapshot.\n\
         Actual: {actual:?}\n\
         Expected: {expected:?}"
    );
}

/// Test that the same manifests are discovered on multiple runs.
///
/// This validates idempotency - running discovery multiple times should
/// produce identical results.
#[test]
fn manifest_discovery_is_idempotent() {
    let fixture_path = fixtures_dir().join("manifest_ordering");

    // Run discovery multiple times
    let run1 = discover_manifests(&fixture_path).expect("first discovery should succeed");
    let run2 = discover_manifests(&fixture_path).expect("second discovery should succeed");
    let run3 = discover_manifests(&fixture_path).expect("third discovery should succeed");

    // Convert to comparable format
    let run1_paths: Vec<&str> = run1.iter().map(|p| p.as_str()).collect();
    let run2_paths: Vec<&str> = run2.iter().map(|p| p.as_str()).collect();
    let run3_paths: Vec<&str> = run3.iter().map(|p| p.as_str()).collect();

    assert_eq!(
        run1_paths, run2_paths,
        "Discovery should produce identical results on runs 1 and 2"
    );
    assert_eq!(
        run2_paths, run3_paths,
        "Discovery should produce identical results on runs 2 and 3"
    );
}

/// Test that the root Cargo.toml is always first in the discovered manifests.
#[test]
fn root_manifest_is_first() {
    let fixture_path = fixtures_dir().join("manifest_ordering");

    let manifests = discover_manifests(&fixture_path).expect("discovery should succeed");

    assert!(
        !manifests.is_empty(),
        "Should discover at least one manifest"
    );
    assert_eq!(
        manifests[0].as_str(),
        "Cargo.toml",
        "Root Cargo.toml should be first"
    );
}

/// Test that manifests are sorted lexicographically by path.
#[test]
fn manifests_are_sorted_lexicographically() {
    let fixture_path = fixtures_dir().join("manifest_ordering");

    let manifests = discover_manifests(&fixture_path).expect("discovery should succeed");
    let paths: Vec<&str> = manifests.iter().map(|p| p.as_str()).collect();

    // Verify the list is sorted
    let mut sorted_paths = paths.clone();
    sorted_paths.sort();

    assert_eq!(
        paths, sorted_paths,
        "Manifests should be in lexicographic order"
    );
}

/// Test that discovery with globs produces sorted results.
///
/// Uses the workspace_members_exclude fixture which uses glob patterns.
#[test]
fn glob_expansion_produces_sorted_results() {
    let fixture_path = fixtures_dir().join("workspace_members_exclude");

    let manifests = discover_manifests(&fixture_path).expect("discovery should succeed");
    let paths: Vec<&str> = manifests.iter().map(|p| p.as_str()).collect();

    // Verify the list is sorted
    let mut sorted_paths = paths.clone();
    sorted_paths.sort();

    assert_eq!(
        paths, sorted_paths,
        "Glob-expanded manifests should be in lexicographic order"
    );
}

/// Test that nested workspaces don't interfere with ordering.
#[test]
fn nested_workspace_maintains_ordering() {
    let fixture_path = fixtures_dir().join("nested_workspace");

    let manifests = discover_manifests(&fixture_path).expect("discovery should succeed");
    let paths: Vec<&str> = manifests.iter().map(|p| p.as_str()).collect();

    // Verify the list is sorted
    let mut sorted_paths = paths.clone();
    sorted_paths.sort();

    assert_eq!(
        paths, sorted_paths,
        "Nested workspace manifests should be in lexicographic order"
    );
}

// ============================================================================
// Edge Case Tests
// ============================================================================

/// Test discovery on a package with only [package] section (no dependencies).
/// This verifies that a minimal Cargo.toml is handled correctly.
#[test]
fn discovers_single_package_without_dependencies() {
    let fixture_path = fixtures_dir().join("empty_package");

    let manifests = discover_manifests(&fixture_path).expect("discovery should succeed");

    assert_eq!(manifests.len(), 1);
    assert_eq!(manifests[0].as_str(), "Cargo.toml");
}

/// Test that deeply nested paths (10 levels) are discovered correctly.
/// Note: Windows has a MAX_PATH limitation of 260 characters, so we use
/// shorter directory names and fewer levels to stay within limits.
#[test]
fn discovers_deeply_nested_manifests() {
    // Create a temporary directory with deeply nested structure
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let root_path = camino::Utf8PathBuf::from_path_buf(temp_dir.path().to_path_buf())
        .expect("valid utf-8 path");

    // Create nested directory structure (10 levels with short names to work within Windows MAX_PATH)
    let mut deep_path = root_path.clone();
    for i in 0..10 {
        deep_path = deep_path.join(format!("l{}", i));
    }
    std::fs::create_dir_all(&deep_path).expect("create deep directories");

    // Create root Cargo.toml with glob pattern to find nested crates
    let root_manifest = r#"
[package]
name = "deep-root"
version = "0.1.0"
edition = "2021"

[workspace]
members = ["l0/l1/l2/l3/l4/l5/l6/l7/l8/l9"]
"#;
    std::fs::write(root_path.join("Cargo.toml"), root_manifest).expect("write root manifest");

    // Create deep Cargo.toml
    let deep_manifest = r#"
[package]
name = "deep-crate"
version = "0.1.0"
edition = "2021"
"#;
    std::fs::write(deep_path.join("Cargo.toml"), deep_manifest).expect("write deep manifest");

    // Discover manifests
    let manifests = discover_manifests(&root_path).expect("discovery should succeed");

    // Should find root + deep manifest
    assert!(
        manifests.len() >= 2,
        "Should find at least root and deep manifest. Found: {:?}",
        manifests
    );
    assert!(
        manifests.iter().any(|m| m.as_str() == "Cargo.toml"),
        "Should find root manifest"
    );
}

/// Test handling of moderately long file names.
/// Note: Windows has a MAX_PATH limitation of 260 characters, so we use
/// a shorter name that still tests the long-name handling without hitting OS limits.
#[test]
fn handles_long_file_names() {
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let root_path = camino::Utf8PathBuf::from_path_buf(temp_dir.path().to_path_buf())
        .expect("valid utf-8 path");

    // Create a moderately long directory name (100 characters - long enough to test but within Windows limits)
    // Windows temp paths are typically ~100 chars, leaving ~160 for our directory name
    let long_name = "crate".repeat(20); // 100 characters
    let long_dir = root_path.join(&long_name);

    // Skip test if directory creation fails (e.g., on filesystems with strict limits)
    if std::fs::create_dir_all(&long_dir).is_err() {
        eprintln!("Skipping test: cannot create directory with long name on this filesystem");
        return;
    }

    // Create root Cargo.toml with wildcard member
    let root_manifest = r#"
[package]
name = "long-name-root"
version = "0.1.0"
edition = "2021"

[workspace]
members = ["*"]
"#;
    std::fs::write(root_path.join("Cargo.toml"), root_manifest).expect("write root manifest");

    // Create Cargo.toml in long-named directory
    let deep_manifest = r#"
[package]
name = "long-name-crate"
version = "0.1.0"
edition = "2021"
"#;
    std::fs::write(long_dir.join("Cargo.toml"), deep_manifest).expect("write long name manifest");

    // Discovery should handle long paths without panic
    let result = discover_manifests(&root_path);
    assert!(result.is_ok(), "Discovery should handle long file names");
    let manifests = result.unwrap();
    assert!(manifests.len() >= 1, "Should find at least root manifest");
}

/// Test handling of Unicode characters in paths.
#[test]
fn handles_unicode_paths() {
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let root_path = camino::Utf8PathBuf::from_path_buf(temp_dir.path().to_path_buf())
        .expect("valid utf-8 path");

    // Create directories with unicode names
    let unicode_dir = root_path.join("crates-日本語");
    std::fs::create_dir_all(&unicode_dir).expect("create unicode directory");

    // Create root Cargo.toml
    let root_manifest = r#"
[package]
name = "unicode-root"
version = "0.1.0"
edition = "2021"

[workspace]
members = ["crates-*"]
"#;
    std::fs::write(root_path.join("Cargo.toml"), root_manifest).expect("write root manifest");

    // Create Cargo.toml in unicode directory
    let unicode_manifest = r#"
[package]
name = "unicode-crate"
version = "0.1.0"
edition = "2021"
"#;
    std::fs::write(unicode_dir.join("Cargo.toml"), unicode_manifest)
        .expect("write unicode manifest");

    // Discovery should handle unicode paths
    let result = discover_manifests(&root_path);
    assert!(result.is_ok(), "Discovery should handle unicode paths");
    let manifests = result.unwrap();
    assert!(
        manifests.len() >= 2,
        "Should find root and unicode manifest"
    );
}

/// Test behavior with circular workspace references.
///
/// Note: Per docs in discover.rs, circular workspace references are NOT currently
/// detected by this implementation. Cargo would error during resolution, but
/// this implementation may include nested workspaces. This test documents the
/// current behavior rather than asserting correct handling.
#[test]
fn circular_workspace_references_behavior() {
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let root_path = camino::Utf8PathBuf::from_path_buf(temp_dir.path().to_path_buf())
        .expect("valid utf-8 path");

    // Create workspace A
    let workspace_a = root_path.join("workspace-a");
    std::fs::create_dir_all(&workspace_a).expect("create workspace-a");

    // Create workspace B
    let workspace_b = root_path.join("workspace-b");
    std::fs::create_dir_all(&workspace_b).expect("create workspace-b");

    // Workspace A points to workspace B as a member
    let manifest_a = r#"
[package]
name = "workspace-a"
version = "0.1.0"
edition = "2021"

[workspace]
members = ["../workspace-b"]
"#;
    std::fs::write(workspace_a.join("Cargo.toml"), manifest_a).expect("write workspace-a manifest");

    // Workspace B points back to workspace A (circular reference)
    let manifest_b = r#"
[package]
name = "workspace-b"
version = "0.1.0"
edition = "2021"

[workspace]
members = ["../workspace-a"]
"#;
    std::fs::write(workspace_b.join("Cargo.toml"), manifest_b).expect("write workspace-b manifest");

    // Discovery on workspace A - current behavior is to NOT detect the cycle
    // The discovery will include both manifests (potentially with duplicates)
    let result = discover_manifests(&workspace_a);

    // The current implementation should succeed (not error on circular refs)
    // This documents the behavior that circular refs are not detected
    assert!(
        result.is_ok(),
        "Discovery should complete without error on circular refs"
    );

    // The result should contain at least the root manifest
    let manifests = result.expect("discovery result");
    assert!(
        manifests.iter().any(|m| m.as_str() == "Cargo.toml"),
        "Should find root manifest of workspace-a"
    );

    // Note: The behavior for nested/circular workspace inclusion is undefined
    // This test documents that discovery doesn't panic or error
}
