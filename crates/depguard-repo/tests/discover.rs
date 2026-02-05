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
