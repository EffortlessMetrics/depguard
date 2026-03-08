//! Property-based tests for the domain crate.
//!
//! These tests use proptest to verify invariants around:
//! - Dependency spec handling and normalization
//! - Findings ordering determinism
//! - Check behavior with edge cases

use crate::engine::evaluate;
use crate::model::{
    DepKind, DepSpec, DependencyDecl, ManifestModel, PackageMeta, WorkspaceDependency,
    WorkspaceModel,
};
use crate::policy::{CheckPolicy, EffectiveConfig, FailOn, Scope};
use depguard_types::{Finding, Location, RepoPath, Severity, Verdict, ids};
use proptest::prelude::*;
use std::collections::BTreeMap;

// ============================================================================
// Strategies for generating arbitrary values
// ============================================================================

/// Strategy for valid dependency names (alphanumeric, underscore, hyphen).
/// Crate names must start with a letter and be non-empty.
fn arb_dep_name() -> impl Strategy<Value = String> {
    prop::string::string_regex("[a-z][a-z0-9_-]{0,31}")
        .unwrap()
        .prop_filter("name must not be empty", |s| !s.is_empty())
}

/// Strategy for valid semver version strings.
fn arb_version() -> impl Strategy<Value = String> {
    prop_oneof![
        // Simple version
        (1u32..100, 0u32..100, 0u32..100)
            .prop_map(|(major, minor, patch)| format!("{}.{}.{}", major, minor, patch)),
        // Version with caret
        (1u32..100, 0u32..100, 0u32..100)
            .prop_map(|(major, minor, patch)| format!("^{}.{}.{}", major, minor, patch)),
        // Version with tilde
        (1u32..100, 0u32..100, 0u32..100)
            .prop_map(|(major, minor, patch)| format!("~{}.{}.{}", major, minor, patch)),
        // Version range
        (1u32..50, 0u32..100, 0u32..100).prop_map(|(major, minor, patch)| format!(
            ">={}.{}.{}, <{}",
            major,
            minor,
            patch,
            major + 1
        )),
        // Exact version
        (1u32..100, 0u32..100, 0u32..100)
            .prop_map(|(major, minor, patch)| format!("={}.{}.{}", major, minor, patch)),
    ]
}

/// Strategy for wildcard version strings.
fn arb_wildcard_version() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("*".to_string()),
        (1u32..100).prop_map(|major| format!("{}.*", major)),
        (1u32..100, 0u32..100).prop_map(|(major, minor)| format!("{}.{}.*", major, minor)),
    ]
}

/// Strategy for valid relative paths (no absolute paths, no escapes).
/// Used for generating arbitrary path dependencies in tests.
#[allow(dead_code)]
fn arb_relative_path() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("../foo".to_string()),
        Just("../bar/baz".to_string()),
        prop::string::string_regex("[a-z][a-z0-9_-]{0,15}(/[a-z][a-z0-9_-]{0,15}){0,3}")
            .unwrap()
            .prop_filter("path must not be empty", |s| !s.is_empty()),
    ]
}

/// Strategy for safe relative paths (within repo, no parent escapes from root).
fn arb_safe_relative_path() -> impl Strategy<Value = String> {
    prop::string::string_regex("[a-z][a-z0-9_-]{0,15}(/[a-z][a-z0-9_-]{0,15}){0,3}")
        .unwrap()
        .prop_filter("path must not be empty", |s| !s.is_empty())
}

/// Strategy for paths that escape the repo root.
fn arb_escaping_path() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("..".to_string()),
        Just("../..".to_string()),
        Just("../external".to_string()),
        Just("../../external/crate".to_string()),
    ]
}

/// Strategy for absolute paths.
fn arb_absolute_path() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("/absolute/path".to_string()),
        Just("/usr/local/lib".to_string()),
        Just("C:/Users/project".to_string()),
        Just("D:/code/crate".to_string()),
    ]
}

/// Strategy for DepKind values.
/// Used for generating arbitrary dependency declarations in tests.
#[allow(dead_code)]
fn arb_dep_kind() -> impl Strategy<Value = DepKind> {
    prop_oneof![
        Just(DepKind::Normal),
        Just(DepKind::Dev),
        Just(DepKind::Build),
    ]
}

/// Strategy for Severity values.
fn arb_severity() -> impl Strategy<Value = Severity> {
    prop_oneof![
        Just(Severity::Info),
        Just(Severity::Warning),
        Just(Severity::Error),
    ]
}

/// Strategy for DepSpec variants (the different shapes of dependency specs).
fn arb_dep_spec() -> impl Strategy<Value = DepSpec> {
    prop_oneof![
        // String version: `"1.0"` -> version only
        arb_version().prop_map(|v| DepSpec {
            version: Some(v),
            ..DepSpec::default()
        }),
        // Inline table with version: `{ version = "1.0" }`
        arb_version().prop_map(|v| DepSpec {
            version: Some(v),
            ..DepSpec::default()
        }),
        // Inline table with path and version: `{ path = "../foo", version = "1.0" }`
        (arb_safe_relative_path(), arb_version()).prop_map(|(p, v)| DepSpec {
            version: Some(v),
            path: Some(p),
            ..DepSpec::default()
        }),
        // Inline table with path only: `{ path = "../foo" }`
        arb_safe_relative_path().prop_map(|p| DepSpec {
            path: Some(p),
            ..DepSpec::default()
        }),
        // Workspace reference: `{ workspace = true }`
        Just(DepSpec {
            workspace: true,
            ..DepSpec::default()
        }),
        // Workspace reference with features: `{ workspace = true, features = [...] }`
        // (features not tracked in DepSpec, but workspace = true is the key)
        Just(DepSpec {
            workspace: true,
            ..DepSpec::default()
        }),
    ]
}

/// Strategy for creating a DependencyDecl.
/// Used for generating arbitrary dependency declarations in tests.
#[allow(dead_code)]
fn arb_dependency_decl() -> impl Strategy<Value = DependencyDecl> {
    (
        arb_dep_kind(),
        arb_dep_name(),
        arb_dep_spec(),
        any::<Option<u32>>(),
    )
        .prop_map(|(kind, name, spec, line)| DependencyDecl {
            kind,
            name: name.clone(),
            spec,
            location: Some(Location {
                path: RepoPath::new("Cargo.toml"),
                line,
                col: None,
            }),
            target: None,
        })
}

/// Strategy for creating a Finding (used for ordering tests).
fn arb_finding() -> impl Strategy<Value = Finding> {
    (
        arb_severity(),
        prop_oneof![
            Just(ids::CHECK_DEPS_NO_WILDCARDS.to_string()),
            Just(ids::CHECK_DEPS_PATH_REQUIRES_VERSION.to_string()),
            Just(ids::CHECK_DEPS_PATH_SAFETY.to_string()),
            Just(ids::CHECK_DEPS_WORKSPACE_INHERITANCE.to_string()),
        ],
        prop_oneof![
            Just(ids::CODE_WILDCARD_VERSION.to_string()),
            Just(ids::CODE_PATH_WITHOUT_VERSION.to_string()),
            Just(ids::CODE_ABSOLUTE_PATH.to_string()),
            Just(ids::CODE_PARENT_ESCAPE.to_string()),
            Just(ids::CODE_MISSING_WORKSPACE_TRUE.to_string()),
        ],
        "test message [a-z]{1,20}",
        prop::option::of((
            prop_oneof![
                Just("Cargo.toml".to_string()),
                Just("crates/foo/Cargo.toml".to_string()),
                Just("crates/bar/Cargo.toml".to_string()),
            ],
            prop::option::of(1u32..1000),
        )),
    )
        .prop_map(|(severity, check_id, code, message, location)| Finding {
            severity,
            check_id,
            code,
            message,
            location: location.map(|(path, line)| Location {
                path: RepoPath::new(path),
                line,
                col: None,
            }),
            help: None,
            url: None,
            fingerprint: None,
            data: serde_json::Value::Null,
        })
}

/// Create an EffectiveConfig with all checks enabled at the given severity.
fn config_all_enabled(severity: Severity) -> EffectiveConfig {
    let mut checks = BTreeMap::new();
    checks.insert(
        ids::CHECK_DEPS_NO_WILDCARDS.to_string(),
        CheckPolicy::enabled(severity),
    );
    checks.insert(
        ids::CHECK_DEPS_PATH_REQUIRES_VERSION.to_string(),
        CheckPolicy::enabled(severity),
    );
    checks.insert(
        ids::CHECK_DEPS_PATH_SAFETY.to_string(),
        CheckPolicy::enabled(severity),
    );
    checks.insert(
        ids::CHECK_DEPS_WORKSPACE_INHERITANCE.to_string(),
        CheckPolicy::enabled(severity),
    );

    EffectiveConfig {
        profile: "test".to_string(),
        scope: Scope::Repo,
        fail_on: FailOn::Error,
        max_findings: 200,
        yanked_index: None,
        checks,
    }
}

/// Create an EffectiveConfig with all checks disabled.
fn config_all_disabled() -> EffectiveConfig {
    let mut checks = BTreeMap::new();
    checks.insert(
        ids::CHECK_DEPS_NO_WILDCARDS.to_string(),
        CheckPolicy::disabled(),
    );
    checks.insert(
        ids::CHECK_DEPS_PATH_REQUIRES_VERSION.to_string(),
        CheckPolicy::disabled(),
    );
    checks.insert(
        ids::CHECK_DEPS_PATH_SAFETY.to_string(),
        CheckPolicy::disabled(),
    );
    checks.insert(
        ids::CHECK_DEPS_WORKSPACE_INHERITANCE.to_string(),
        CheckPolicy::disabled(),
    );

    EffectiveConfig {
        profile: "test".to_string(),
        scope: Scope::Repo,
        fail_on: FailOn::Error,
        max_findings: 200,
        yanked_index: None,
        checks,
    }
}

// ============================================================================
// Property tests: Dependency spec normalization
// ============================================================================

proptest! {
    /// All valid DepSpec shapes should be representable and extractable.
    #[test]
    fn dep_spec_shapes_are_valid(spec in arb_dep_spec()) {
        // Invariant: At least one of version, path, or workspace must be set.
        let has_version = spec.version.is_some();
        let has_path = spec.path.is_some();
        let has_workspace = spec.workspace;

        prop_assert!(
            has_version || has_path || has_workspace,
            "DepSpec must have at least one field set: version={:?}, path={:?}, workspace={}",
            spec.version,
            spec.path,
            spec.workspace
        );
    }

    /// Version strings should be non-empty when present.
    #[test]
    fn version_strings_are_non_empty(version in arb_version()) {
        prop_assert!(!version.is_empty(), "Version should not be empty");
        prop_assert!(
            !version.contains('\0'),
            "Version should not contain null bytes"
        );
    }

    /// Dependency names must follow Cargo naming conventions.
    #[test]
    fn dep_names_are_valid(name in arb_dep_name()) {
        prop_assert!(!name.is_empty(), "Name should not be empty");
        prop_assert!(
            name.chars().next().unwrap().is_ascii_lowercase(),
            "Name should start with a lowercase letter"
        );
        prop_assert!(
            name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-'),
            "Name should only contain alphanumeric, underscore, or hyphen"
        );
    }

    /// Relative paths should not be absolute.
    #[test]
    fn relative_paths_are_not_absolute(path in arb_safe_relative_path()) {
        prop_assert!(!path.starts_with('/'), "Path should not start with /");
        prop_assert!(
            !(path.len() >= 2 && path.chars().nth(1) == Some(':')),
            "Path should not be a Windows absolute path"
        );
    }
}

// ============================================================================
// Property tests: Findings ordering determinism
// ============================================================================

/// Helper: Sort findings using the same comparator as the engine.
/// This must stay in sync with the compare_findings function in engine.rs.
fn sort_findings(f: &mut [Finding]) {
    f.sort_by(|a, b| {
        let rank = |s: Severity| match s {
            Severity::Error => 0u8,
            Severity::Warning => 1u8,
            Severity::Info => 2u8,
        };
        let (ap, al) = match &a.location {
            Some(l) => (l.path.as_str(), l.line.unwrap_or(u32::MAX)),
            None => ("~", u32::MAX),
        };
        let (bp, bl) = match &b.location {
            Some(l) => (l.path.as_str(), l.line.unwrap_or(u32::MAX)),
            None => ("~", u32::MAX),
        };
        rank(a.severity)
            .cmp(&rank(b.severity))
            .then(ap.cmp(bp))
            .then(al.cmp(&bl))
            .then(a.check_id.cmp(&b.check_id))
            .then(a.code.cmp(&b.code))
            .then(a.message.cmp(&b.message))
    });
}

/// Helper: Verify that a slice of findings is sorted according to the documented order.
fn assert_sorted(findings: &[Finding]) -> Result<(), proptest::test_runner::TestCaseError> {
    for i in 1..findings.len() {
        let prev = &findings[i - 1];
        let curr = &findings[i];

        let rank = |s: Severity| match s {
            Severity::Error => 0u8,
            Severity::Warning => 1u8,
            Severity::Info => 2u8,
        };
        let (prev_path, prev_line) = match &prev.location {
            Some(l) => (l.path.as_str(), l.line.unwrap_or(u32::MAX)),
            None => ("~", u32::MAX),
        };
        let (curr_path, curr_line) = match &curr.location {
            Some(l) => (l.path.as_str(), l.line.unwrap_or(u32::MAX)),
            None => ("~", u32::MAX),
        };

        let cmp = rank(prev.severity)
            .cmp(&rank(curr.severity))
            .then(prev_path.cmp(curr_path))
            .then(prev_line.cmp(&curr_line))
            .then(prev.check_id.cmp(&curr.check_id))
            .then(prev.code.cmp(&curr.code))
            .then(prev.message.cmp(&curr.message));

        prop_assert!(
            cmp != std::cmp::Ordering::Greater,
            "Findings at index {} and {} are not in correct order: {:?} should come before {:?}",
            i - 1,
            i,
            prev,
            curr
        );
    }
    Ok(())
}

proptest! {
    /// Given any permutation of the same findings, output order should be identical.
    #[test]
    fn findings_ordering_is_deterministic(findings in prop::collection::vec(arb_finding(), 0..20)) {
        use rand::seq::SliceRandom;
        use rand::SeedableRng;

        // Create multiple permutations and sort them
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);

        let mut permutation1 = findings.clone();
        let mut permutation2 = findings.clone();
        let mut permutation3 = findings.clone();

        permutation1.shuffle(&mut rng);
        permutation2.shuffle(&mut rng);
        permutation3.shuffle(&mut rng);

        sort_findings(&mut permutation1);
        sort_findings(&mut permutation2);
        sort_findings(&mut permutation3);

        // All sorted permutations should be identical
        prop_assert_eq!(
            permutation1.len(),
            permutation2.len(),
            "Permutations should have the same length"
        );
        prop_assert_eq!(
            permutation2.len(),
            permutation3.len(),
            "Permutations should have the same length"
        );

        for i in 0..permutation1.len() {
            prop_assert_eq!(
                &permutation1[i].check_id,
                &permutation2[i].check_id,
                "check_id mismatch at index {}", i
            );
            prop_assert_eq!(
                &permutation1[i].code,
                &permutation2[i].code,
                "code mismatch at index {}", i
            );
            prop_assert_eq!(
                &permutation1[i].message,
                &permutation2[i].message,
                "message mismatch at index {}", i
            );
        }
    }

    /// Same input should always produce the same output (idempotence of sorting).
    #[test]
    fn findings_ordering_is_stable(findings in prop::collection::vec(arb_finding(), 0..20)) {
        let mut sorted1 = findings.clone();
        let mut sorted2 = findings.clone();

        sort_findings(&mut sorted1);
        sort_findings(&mut sorted2);

        // Sorting the same input twice should yield identical results
        for i in 0..sorted1.len() {
            prop_assert_eq!(
                &sorted1[i].check_id,
                &sorted2[i].check_id,
                "check_id mismatch at index {}", i
            );
            prop_assert_eq!(
                &sorted1[i].code,
                &sorted2[i].code,
                "code mismatch at index {}", i
            );
            prop_assert_eq!(
                &sorted1[i].message,
                &sorted2[i].message,
                "message mismatch at index {}", i
            );
        }
    }

    /// Severity ordering: Error < Warning < Info (Error comes first)
    #[test]
    fn severity_ordering_is_error_warning_info(findings in prop::collection::vec(arb_finding(), 2..30)) {
        let mut sorted = findings.clone();
        sort_findings(&mut sorted);

        // Verify all errors come before warnings, and all warnings come before infos
        let mut seen_warning = false;
        let mut seen_info = false;

        for f in &sorted {
            match f.severity {
                Severity::Error => {
                    prop_assert!(!seen_warning && !seen_info,
                        "Error found after Warning or Info");
                }
                Severity::Warning => {
                    seen_warning = true;
                    prop_assert!(!seen_info,
                        "Warning found after Info");
                }
                Severity::Info => {
                    seen_info = true;
                }
            }
        }
    }

    /// After sorting, the findings slice should be in sorted order according to all criteria.
    #[test]
    fn sorted_findings_are_in_correct_order(findings in prop::collection::vec(arb_finding(), 0..30)) {
        let mut sorted = findings.clone();
        sort_findings(&mut sorted);
        assert_sorted(&sorted)?;
    }

    /// Shuffling and re-sorting produces the same result every time.
    #[test]
    fn shuffle_and_resort_is_idempotent(
        findings in prop::collection::vec(arb_finding(), 1..25),
        seed in any::<u64>(),
    ) {
        use rand::seq::SliceRandom;
        use rand::SeedableRng;

        // First sort to establish baseline
        let mut baseline = findings.clone();
        sort_findings(&mut baseline);

        // Shuffle with random seed and re-sort
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
        let mut shuffled = findings.clone();
        shuffled.shuffle(&mut rng);
        sort_findings(&mut shuffled);

        // Should match baseline
        prop_assert_eq!(baseline.len(), shuffled.len());
        for i in 0..baseline.len() {
            prop_assert_eq!(
                &baseline[i].severity,
                &shuffled[i].severity,
                "severity mismatch at index {}", i
            );
            prop_assert_eq!(
                &baseline[i].check_id,
                &shuffled[i].check_id,
                "check_id mismatch at index {}", i
            );
            prop_assert_eq!(
                &baseline[i].code,
                &shuffled[i].code,
                "code mismatch at index {}", i
            );
            prop_assert_eq!(
                &baseline[i].message,
                &shuffled[i].message,
                "message mismatch at index {}", i
            );
        }
    }

    /// Within the same severity level, paths are sorted alphabetically.
    #[test]
    fn paths_sorted_alphabetically_within_severity(
        findings in prop::collection::vec(arb_finding(), 2..20),
    ) {
        let mut sorted = findings.clone();
        sort_findings(&mut sorted);

        // Group by severity and verify path ordering within each group
        let mut prev_severity: Option<Severity> = None;
        let mut prev_path: Option<String> = None;

        for f in &sorted {
            let curr_path = f.location.as_ref().map(|l| l.path.as_str().to_string()).unwrap_or_else(|| "~".to_string());

            if prev_severity == Some(f.severity)
                && let Some(ref pp) = prev_path {
                    prop_assert!(
                        pp <= &curr_path,
                        "Path order violation within severity {:?}: {} > {}",
                        f.severity,
                        pp,
                        curr_path
                    );
                }

            prev_severity = Some(f.severity);
            prev_path = Some(curr_path);
        }
    }
}

// ============================================================================
// Property tests: Check invariants
// ============================================================================

proptest! {
    /// Empty workspace model should produce no findings for any check.
    #[test]
    fn empty_workspace_produces_no_findings(severity in arb_severity()) {
        let model = WorkspaceModel {
            repo_root: RepoPath::new("."),
            workspace_dependencies: BTreeMap::new(),
            manifests: vec![],
        };

        let cfg = config_all_enabled(severity);
        let report = evaluate(&model, &cfg);

        prop_assert!(
            report.findings.is_empty(),
            "Empty workspace should produce no findings, got {:?}",
            report.findings
        );
    }

    /// Disabled checks should produce no findings regardless of violations.
    #[test]
    fn disabled_checks_produce_no_findings(
        dep_name in arb_dep_name(),
        wildcard_version in arb_wildcard_version(),
        escaping_path in arb_escaping_path(),
    ) {
        // Create a model with multiple violations
        let model = WorkspaceModel {
            repo_root: RepoPath::new("."),
            workspace_dependencies: {
                let mut m = BTreeMap::new();
                m.insert(
                    dep_name.clone(),
                    WorkspaceDependency {
                        name: dep_name.clone(),
                        version: Some("1.0.0".to_string()),
                        path: None,
                        workspace: false,
                    },
                );
                m
            },
            manifests: vec![ManifestModel {
                path: RepoPath::new("Cargo.toml"),
                package: Some(PackageMeta {
                    name: "test-pkg".to_string(),
                    publish: true,
                }),
                features: BTreeMap::new(),
                dependencies: vec![
                    // Wildcard violation
                    DependencyDecl {
                        kind: DepKind::Normal,
                        name: format!("{}-wildcard", dep_name),
                        spec: DepSpec {
                            version: Some(wildcard_version),
                            ..DepSpec::default()
                        },
                        location: Some(Location {
                            path: RepoPath::new("Cargo.toml"),
                            line: Some(10),
                            col: None,
                        }),
                        target: None,
                    },
                    // Path without version violation
                    DependencyDecl {
                        kind: DepKind::Normal,
                        name: format!("{}-path", dep_name),
                        spec: DepSpec {
                            version: None,
                            path: Some(escaping_path),
                            ..DepSpec::default()
                        },
                        location: Some(Location {
                            path: RepoPath::new("Cargo.toml"),
                            line: Some(20),
                            col: None,
                        }),
                        target: None,
                    },
                    // Workspace inheritance violation (not using workspace = true)
                    DependencyDecl {
                        kind: DepKind::Normal,
                        name: dep_name.clone(),
                        spec: DepSpec {
                            version: Some("2.0.0".to_string()),
                            ..DepSpec::default()
                        },
                        location: Some(Location {
                            path: RepoPath::new("Cargo.toml"),
                            line: Some(30),
                            col: None,
                        }),
                        target: None,
                    },
                ],
            }],
        };

        let cfg = config_all_disabled();
        let report = evaluate(&model, &cfg);

        prop_assert!(
            report.findings.is_empty(),
            "Disabled checks should produce no findings, got {:?}",
            report.findings
        );
    }

    /// Workspace reference dependencies should not trigger path_requires_version check.
    #[test]
    fn workspace_ref_does_not_trigger_path_requires_version(dep_name in arb_dep_name()) {
        let model = WorkspaceModel {
            repo_root: RepoPath::new("."),
            workspace_dependencies: {
                let mut m = BTreeMap::new();
                m.insert(
                    dep_name.clone(),
                    WorkspaceDependency {
                        name: dep_name.clone(),
                        version: Some("1.0.0".to_string()),
                        path: None,
                        workspace: false,
                    },
                );
                m
            },
            manifests: vec![ManifestModel {
                path: RepoPath::new("Cargo.toml"),
                package: Some(PackageMeta {
                    name: "test-pkg".to_string(),
                    publish: true,
                }),
                features: BTreeMap::new(),
                dependencies: vec![
                    DependencyDecl {
                        kind: DepKind::Normal,
                        name: dep_name.clone(),
                        spec: DepSpec {
                            workspace: true,
                            ..DepSpec::default()
                        },
                        location: Some(Location {
                            path: RepoPath::new("Cargo.toml"),
                            line: Some(10),
                            col: None,
                        }),
                        target: None,
                    },
                    DependencyDecl {
                        kind: DepKind::Normal,
                        name: format!("{dep_name}_wild"),
                        spec: DepSpec {
                            version: Some("*".to_string()),
                            ..DepSpec::default()
                        },
                        location: Some(Location {
                            path: RepoPath::new("Cargo.toml"),
                            line: Some(12),
                            col: None,
                        }),
                        target: None,
                    },
                ],
            }],
        };

        let cfg = config_all_enabled(Severity::Error);
        let report = evaluate(&model, &cfg);

        // Should not have path_requires_version findings
        let path_version_findings: Vec<_> = report
            .findings
            .iter()
            .filter(|f| f.check_id == ids::CHECK_DEPS_PATH_REQUIRES_VERSION)
            .collect();

        prop_assert!(
            path_version_findings.is_empty(),
            "workspace = true should not trigger path_requires_version, got {:?}",
            path_version_findings
        );
    }

    /// Wildcard versions should always be detected by no_wildcards check.
    #[test]
    fn wildcards_are_always_detected(
        dep_name in arb_dep_name(),
        wildcard in arb_wildcard_version(),
    ) {
        let model = WorkspaceModel {
            repo_root: RepoPath::new("."),
            workspace_dependencies: BTreeMap::new(),
            manifests: vec![ManifestModel {
                path: RepoPath::new("Cargo.toml"),
                package: Some(PackageMeta {
                    name: "test-pkg".to_string(),
                    publish: true,
                }),
                features: BTreeMap::new(),
                dependencies: vec![DependencyDecl {
                    kind: DepKind::Normal,
                    name: dep_name.clone(),
                    spec: DepSpec {
                        version: Some(wildcard.clone()),
                        ..DepSpec::default()
                    },
                    location: Some(Location {
                        path: RepoPath::new("Cargo.toml"),
                        line: Some(10),
                        col: None,
                    }),
                    target: None,
                }],
            }],
        };

        let cfg = config_all_enabled(Severity::Warning);
        let report = evaluate(&model, &cfg);

        let wildcard_findings: Vec<_> = report
            .findings
            .iter()
            .filter(|f| f.check_id == ids::CHECK_DEPS_NO_WILDCARDS)
            .collect();

        prop_assert!(
            !wildcard_findings.is_empty(),
            "Wildcard version '{}' should be detected for dependency '{}'",
            wildcard,
            dep_name
        );
    }

    /// Absolute paths should always be detected by path_safety check.
    #[test]
    fn absolute_paths_are_always_detected(
        dep_name in arb_dep_name(),
        abs_path in arb_absolute_path(),
    ) {
        let model = WorkspaceModel {
            repo_root: RepoPath::new("."),
            workspace_dependencies: BTreeMap::new(),
            manifests: vec![ManifestModel {
                path: RepoPath::new("Cargo.toml"),
                package: Some(PackageMeta {
                    name: "test-pkg".to_string(),
                    publish: true,
                }),
                features: BTreeMap::new(),
                dependencies: vec![DependencyDecl {
                    kind: DepKind::Normal,
                    name: dep_name.clone(),
                    spec: DepSpec {
                        path: Some(abs_path.clone()),
                        ..DepSpec::default()
                    },
                    location: Some(Location {
                        path: RepoPath::new("Cargo.toml"),
                        line: Some(10),
                        col: None,
                    }),
                    target: None,
                }],
            }],
        };

        let cfg = config_all_enabled(Severity::Error);
        let report = evaluate(&model, &cfg);

        let absolute_path_findings: Vec<_> = report
            .findings
            .iter()
            .filter(|f| f.code == ids::CODE_ABSOLUTE_PATH)
            .collect();

        prop_assert!(
            !absolute_path_findings.is_empty(),
            "Absolute path '{}' should be detected for dependency '{}'",
            abs_path,
            dep_name
        );
    }

    /// Non-publishable packages should not trigger path_requires_version check.
    #[test]
    fn non_publishable_skips_path_requires_version(
        dep_name in arb_dep_name(),
        path in arb_safe_relative_path(),
    ) {
        let model = WorkspaceModel {
            repo_root: RepoPath::new("."),
            workspace_dependencies: BTreeMap::new(),
            manifests: vec![ManifestModel {
                path: RepoPath::new("Cargo.toml"),
                package: Some(PackageMeta {
                    name: "test-pkg".to_string(),
                    publish: false, // Not publishable
                }),
                features: BTreeMap::new(),
                dependencies: vec![
                    DependencyDecl {
                        kind: DepKind::Normal,
                        name: dep_name.clone(),
                        spec: DepSpec {
                            path: Some(path),
                            ..DepSpec::default()
                        },
                        location: Some(Location {
                            path: RepoPath::new("Cargo.toml"),
                            line: Some(10),
                            col: None,
                        }),
                        target: None,
                    },
                    DependencyDecl {
                        kind: DepKind::Normal,
                        name: format!("{dep_name}_wild"),
                        spec: DepSpec {
                            version: Some("*".to_string()),
                            ..DepSpec::default()
                        },
                        location: Some(Location {
                            path: RepoPath::new("Cargo.toml"),
                            line: Some(12),
                            col: None,
                        }),
                        target: None,
                    },
                ],
            }],
        };

        let cfg = config_all_enabled(Severity::Error);
        let report = evaluate(&model, &cfg);

        let path_version_findings: Vec<_> = report
            .findings
            .iter()
            .filter(|f| f.check_id == ids::CHECK_DEPS_PATH_REQUIRES_VERSION)
            .collect();

        prop_assert!(
            path_version_findings.is_empty(),
            "Non-publishable package should skip path_requires_version check, got {:?}",
            path_version_findings
        );
    }
}

// ============================================================================
// Property tests: Engine evaluation invariants
// ============================================================================

proptest! {
    /// The number of findings emitted should never exceed max_findings.
    #[test]
    fn findings_count_respects_max_findings(
        num_deps in 1usize..50,
        max_findings in 1usize..100,
    ) {
        // Create a model with many wildcard violations
        let deps: Vec<DependencyDecl> = (0..num_deps)
            .map(|i| DependencyDecl {
                kind: DepKind::Normal,
                name: format!("dep{}", i),
                spec: DepSpec {
                    version: Some("*".to_string()),
                    ..DepSpec::default()
                },
                location: Some(Location {
                    path: RepoPath::new("Cargo.toml"),
                    line: Some(i as u32 + 1),
                    col: None,
                }),
                target: None,
            })
            .collect();

        let model = WorkspaceModel {
            repo_root: RepoPath::new("."),
            workspace_dependencies: BTreeMap::new(),
            manifests: vec![ManifestModel {
                path: RepoPath::new("Cargo.toml"),
                package: Some(PackageMeta {
                    name: "test-pkg".to_string(),
                    publish: true,
                }),
                features: BTreeMap::new(),
                dependencies: deps,
            }],
        };

        let mut checks = BTreeMap::new();
        checks.insert(
            ids::CHECK_DEPS_NO_WILDCARDS.to_string(),
            CheckPolicy::enabled(Severity::Warning),
        );

        let cfg = EffectiveConfig {
            profile: "test".to_string(),
            scope: Scope::Repo,
            fail_on: FailOn::Error,
            max_findings,
            yanked_index: None,
            checks,
        };

        let report = evaluate(&model, &cfg);

        prop_assert!(
            report.findings.len() <= max_findings,
            "Findings count {} exceeds max_findings {}",
            report.findings.len(),
            max_findings
        );
    }

    /// Findings should be sorted deterministically in the report.
    #[test]
    fn report_findings_are_sorted(num_deps in 1usize..20) {
        // Create a model with violations that will produce findings in random order
        let deps: Vec<DependencyDecl> = (0..num_deps)
            .map(|i| {
                let spec = match i % 3 {
                    0 => DepSpec {
                        version: Some("*".to_string()),
                        ..DepSpec::default()
                    },
                    1 => DepSpec {
                        path: Some("local/dep".to_string()),
                        ..DepSpec::default()
                    },
                    _ => DepSpec {
                        path: Some("/abs/path".to_string()),
                        ..DepSpec::default()
                    },
                };
                DependencyDecl {
                    kind: DepKind::Normal,
                    name: format!("dep{}", num_deps - i), // Reverse order names
                    spec,
                    location: Some(Location {
                        path: RepoPath::new(if i % 2 == 0 {
                            "crates/a/Cargo.toml"
                        } else {
                            "Cargo.toml"
                        }),
                        line: Some((num_deps - i) as u32),
                        col: None,
                    }),
                    target: None,
                }
            })
            .collect();

        let model = WorkspaceModel {
            repo_root: RepoPath::new("."),
            workspace_dependencies: BTreeMap::new(),
            manifests: vec![ManifestModel {
                path: RepoPath::new("Cargo.toml"),
                package: Some(PackageMeta {
                    name: "test-pkg".to_string(),
                    publish: true,
                }),
                features: BTreeMap::new(),
                dependencies: deps,
            }],
        };

        let mut cfg = config_all_enabled(Severity::Warning);
        if let Some(check) = cfg.checks.get_mut(ids::CHECK_DEPS_NO_WILDCARDS) {
            check.severity = Severity::Error;
        }
        if let Some(check) = cfg.checks.get_mut(ids::CHECK_DEPS_PATH_SAFETY) {
            check.severity = Severity::Info;
        }
        let report = evaluate(&model, &cfg);

        // Verify findings are sorted by severity, path, line, check_id, code, message
        for i in 1..report.findings.len() {
            let prev = &report.findings[i - 1];
            let curr = &report.findings[i];

            let (prev_path, prev_line) = match &prev.location {
                Some(l) => (l.path.as_str(), l.line.unwrap_or(u32::MAX)),
                None => ("~", u32::MAX),
            };
            let (curr_path, curr_line) = match &curr.location {
                Some(l) => (l.path.as_str(), l.line.unwrap_or(u32::MAX)),
                None => ("~", u32::MAX),
            };

            let rank = |s: Severity| match s {
                Severity::Error => 0u8,
                Severity::Warning => 1u8,
                Severity::Info => 2u8,
            };
            let cmp = rank(prev.severity)
                .cmp(&rank(curr.severity))
                .then(prev_path.cmp(curr_path))
                .then(prev_line.cmp(&curr_line))
                .then(prev.check_id.cmp(&curr.check_id))
                .then(prev.code.cmp(&curr.code))
                .then(prev.message.cmp(&curr.message));

            prop_assert!(
                cmp != std::cmp::Ordering::Greater,
                "Findings not sorted: {:?} should come before {:?}",
                prev,
                curr
            );
        }
    }
}

// ============================================================================
// Property tests: Determinism (byte-identical outputs)
// ============================================================================

proptest! {
    /// Running the engine twice with identical inputs must produce byte-identical outputs.
    /// This tests the core determinism guarantee: same inputs → same outputs.
    #[test]
    fn engine_determinism_same_input_same_output(
        dep_name in arb_dep_name(),
        severity in arb_severity(),
    ) {
        let model = WorkspaceModel {
            repo_root: RepoPath::new("."),
            workspace_dependencies: BTreeMap::new(),
            manifests: vec![ManifestModel {
                path: RepoPath::new("Cargo.toml"),
                package: Some(PackageMeta {
                    name: "test-pkg".to_string(),
                    publish: true,
                }),
                features: BTreeMap::new(),
                dependencies: vec![
                    DependencyDecl {
                        kind: DepKind::Normal,
                        name: dep_name.clone(),
                        spec: DepSpec {
                            version: Some("*".to_string()),
                            ..DepSpec::default()
                        },
                        location: Some(Location {
                            path: RepoPath::new("Cargo.toml"),
                            line: Some(10),
                            col: None,
                        }),
                        target: None,
                    },
                ],
            }],
        };

        let cfg = config_all_enabled(severity);

        // Run the engine twice
        let report1 = evaluate(&model, &cfg);
        let report2 = evaluate(&model, &cfg);

        // Verdicts must match
        prop_assert_eq!(report1.verdict, report2.verdict, "Verdict mismatch");

        // Findings count must match
        prop_assert_eq!(report1.findings.len(), report2.findings.len(), "Findings count mismatch");

        // Each finding must be identical
        for (i, (f1, f2)) in report1.findings.iter().zip(report2.findings.iter()).enumerate() {
            prop_assert_eq!(f1.severity, f2.severity, "Severity mismatch at index {}", i);
            prop_assert_eq!(&f1.check_id, &f2.check_id, "check_id mismatch at index {}", i);
            prop_assert_eq!(&f1.code, &f2.code, "code mismatch at index {}", i);
            prop_assert_eq!(&f1.message, &f2.message, "message mismatch at index {}", i);
            prop_assert_eq!(&f1.location, &f2.location, "location mismatch at index {}", i);
        }

        // Data must match
        prop_assert_eq!(report1.data.findings_total, report2.data.findings_total);
        prop_assert_eq!(report1.data.findings_emitted, report2.data.findings_emitted);
        prop_assert_eq!(report1.data.truncated_reason, report2.data.truncated_reason);
    }

    /// Multiple evaluations with the same model/config must produce identical findings order.
    #[test]
    fn engine_determinism_multiple_runs_identical_order(
        num_deps in 1usize..15,
        seed in any::<u64>(),
    ) {
        use rand::seq::SliceRandom;
        use rand::SeedableRng;

        // Create a model with various violation types
        let mut deps: Vec<DependencyDecl> = (0..num_deps)
            .map(|i| {
                let violation_type = i % 4;
                let spec = match violation_type {
                    0 => DepSpec { version: Some("*".to_string()), ..DepSpec::default() },
                    1 => DepSpec { path: Some("/abs/path".to_string()), ..DepSpec::default() },
                    2 => DepSpec { path: Some("../escape".to_string()), ..DepSpec::default() },
                    _ => DepSpec { version: Some("1.0".to_string()), ..DepSpec::default() },
                };
                DependencyDecl {
                    kind: DepKind::Normal,
                    name: format!("dep{}", i),
                    spec,
                    location: Some(Location {
                        path: RepoPath::new(if i % 2 == 0 { "Cargo.toml" } else { "crates/foo/Cargo.toml" }),
                        line: Some((i + 1) as u32),
                        col: None,
                    }),
                    target: None,
                }
            })
            .collect();

        // Shuffle deps with seed
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
        deps.shuffle(&mut rng);

        let model = WorkspaceModel {
            repo_root: RepoPath::new("."),
            workspace_dependencies: BTreeMap::new(),
            manifests: vec![ManifestModel {
                path: RepoPath::new("Cargo.toml"),
                package: Some(PackageMeta {
                    name: "test-pkg".to_string(),
                    publish: true,
                }),
                features: BTreeMap::new(),
                dependencies: deps,
            }],
        };

        let cfg = config_all_enabled(Severity::Warning);

        // Run engine 3 times
        let report1 = evaluate(&model, &cfg);
        let report2 = evaluate(&model, &cfg);
        let report3 = evaluate(&model, &cfg);

        // All reports must have identical findings in identical order
        for i in 0..report1.findings.len() {
            prop_assert_eq!(&report1.findings[i].check_id, &report2.findings[i].check_id,
                "check_id mismatch between run1 and run2 at index {}", i);
            prop_assert_eq!(&report2.findings[i].check_id, &report3.findings[i].check_id,
                "check_id mismatch between run2 and run3 at index {}", i);
            prop_assert_eq!(&report1.findings[i].code, &report2.findings[i].code,
                "code mismatch between run1 and run2 at index {}", i);
            prop_assert_eq!(&report1.findings[i].message, &report2.findings[i].message,
                "message mismatch at index {}", i);
        }
    }

    /// JSON serialization of reports must be deterministic.
    #[test]
    fn engine_determinism_json_serialization(
        dep_name in arb_dep_name(),
    ) {
        let model = WorkspaceModel {
            repo_root: RepoPath::new("."),
            workspace_dependencies: BTreeMap::new(),
            manifests: vec![ManifestModel {
                path: RepoPath::new("Cargo.toml"),
                package: Some(PackageMeta {
                    name: "test-pkg".to_string(),
                    publish: true,
                }),
                features: BTreeMap::new(),
                dependencies: vec![DependencyDecl {
                    kind: DepKind::Normal,
                    name: dep_name,
                    spec: DepSpec {
                        version: Some("*".to_string()),
                        ..DepSpec::default()
                    },
                    location: Some(Location {
                        path: RepoPath::new("Cargo.toml"),
                        line: Some(10),
                        col: None,
                    }),
                    target: None,
                }],
            }],
        };

        let cfg = config_all_enabled(Severity::Warning);
        let report = evaluate(&model, &cfg);

        // Serialize twice and compare bytes
        let json1 = serde_json::to_string(&report.findings).unwrap();
        let json2 = serde_json::to_string(&report.findings).unwrap();

        prop_assert_eq!(json1, json2, "JSON serialization must be deterministic");
    }
}

// ============================================================================
// Property tests: Truncation invariants
// ============================================================================

proptest! {
    /// When findings exceed max_findings, the report must indicate truncation correctly.
    #[test]
    fn truncation_sets_reason_when_exceeded(
        num_deps in 10usize..50,
        max_findings in 1usize..9,
    ) {
        // Create a model with many violations
        let deps: Vec<DependencyDecl> = (0..num_deps)
            .map(|i| DependencyDecl {
                kind: DepKind::Normal,
                name: format!("dep{}", i),
                spec: DepSpec {
                    version: Some("*".to_string()),
                    ..DepSpec::default()
                },
                location: Some(Location {
                    path: RepoPath::new("Cargo.toml"),
                    line: Some(i as u32 + 1),
                    col: None,
                }),
                target: None,
            })
            .collect();

        let model = WorkspaceModel {
            repo_root: RepoPath::new("."),
            workspace_dependencies: BTreeMap::new(),
            manifests: vec![ManifestModel {
                path: RepoPath::new("Cargo.toml"),
                package: Some(PackageMeta {
                    name: "test-pkg".to_string(),
                    publish: true,
                }),
                features: BTreeMap::new(),
                dependencies: deps,
            }],
        };

        let mut checks = BTreeMap::new();
        checks.insert(
            ids::CHECK_DEPS_NO_WILDCARDS.to_string(),
            CheckPolicy::enabled(Severity::Warning),
        );

        let cfg = EffectiveConfig {
            profile: "test".to_string(),
            scope: Scope::Repo,
            fail_on: FailOn::Error,
            max_findings,
            yanked_index: None,
            checks,
        };

        let report = evaluate(&model, &cfg);

        // When truncated, truncated_reason must be set
        if report.data.findings_total > report.data.findings_emitted {
            prop_assert!(
                report.data.truncated_reason.is_some(),
                "truncated_reason must be set when findings are truncated"
            );
            let reason = report.data.truncated_reason.unwrap();
            prop_assert!(
                reason.contains("max_findings"),
                "truncated_reason should mention max_findings: {}", reason
            );
        }

        // findings_emitted must never exceed max_findings
        prop_assert!(
            report.findings.len() <= max_findings,
            "findings.len() {} exceeds max_findings {}",
            report.findings.len(),
            max_findings
        );

        // findings_total must reflect the true count before truncation
        prop_assert!(
            report.data.findings_total >= report.data.findings_emitted,
            "findings_total {} should be >= findings_emitted {}",
            report.data.findings_total,
            report.data.findings_emitted
        );
    }

    /// Truncation must preserve deterministic sort order (highest severity first).
    #[test]
    fn truncation_preserves_sort_order(
        num_deps in 5usize..30,
        max_findings in 1usize..10,
    ) {
        let deps: Vec<DependencyDecl> = (0..num_deps)
            .map(|i| {
                // Mix of violation severities
                let spec = match i % 3 {
                    0 => DepSpec { version: Some("*".to_string()), ..DepSpec::default() },
                    1 => DepSpec { path: Some("/abs/path".to_string()), ..DepSpec::default() },
                    _ => DepSpec { path: Some("../escape".to_string()), ..DepSpec::default() },
                };
                DependencyDecl {
                    kind: DepKind::Normal,
                    name: format!("dep{}", i),
                    spec,
                    location: Some(Location {
                        path: RepoPath::new("Cargo.toml"),
                        line: Some(i as u32 + 1),
                        col: None,
                    }),
                    target: None,
                }
            })
            .collect();

        let model = WorkspaceModel {
            repo_root: RepoPath::new("."),
            workspace_dependencies: BTreeMap::new(),
            manifests: vec![ManifestModel {
                path: RepoPath::new("Cargo.toml"),
                package: Some(PackageMeta {
                    name: "test-pkg".to_string(),
                    publish: true,
                }),
                features: BTreeMap::new(),
                dependencies: deps,
            }],
        };

        let mut checks = BTreeMap::new();
        checks.insert(
            ids::CHECK_DEPS_NO_WILDCARDS.to_string(),
            CheckPolicy::enabled(Severity::Error),
        );
        checks.insert(
            ids::CHECK_DEPS_PATH_SAFETY.to_string(),
            CheckPolicy::enabled(Severity::Warning),
        );

        let cfg = EffectiveConfig {
            profile: "test".to_string(),
            scope: Scope::Repo,
            fail_on: FailOn::Error,
            max_findings,
            yanked_index: None,
            checks,
        };

        let report = evaluate(&model, &cfg);

        // Even after truncation, findings must be sorted
        assert_sorted(&report.findings)?;
    }

    /// Truncation must never corrupt the findings_total count.
    #[test]
    fn truncation_total_count_is_accurate(
        num_deps in 1usize..20,
        max_findings in 1usize..100,
    ) {
        let deps: Vec<DependencyDecl> = (0..num_deps)
            .map(|i| DependencyDecl {
                kind: DepKind::Normal,
                name: format!("dep{}", i),
                spec: DepSpec {
                    version: Some("*".to_string()),
                    ..DepSpec::default()
                },
                location: Some(Location {
                    path: RepoPath::new("Cargo.toml"),
                    line: Some(i as u32 + 1),
                    col: None,
                }),
                target: None,
            })
            .collect();

        let model = WorkspaceModel {
            repo_root: RepoPath::new("."),
            workspace_dependencies: BTreeMap::new(),
            manifests: vec![ManifestModel {
                path: RepoPath::new("Cargo.toml"),
                package: Some(PackageMeta {
                    name: "test-pkg".to_string(),
                    publish: true,
                }),
                features: BTreeMap::new(),
                dependencies: deps,
            }],
        };

        let mut checks = BTreeMap::new();
        checks.insert(
            ids::CHECK_DEPS_NO_WILDCARDS.to_string(),
            CheckPolicy::enabled(Severity::Warning),
        );

        let cfg = EffectiveConfig {
            profile: "test".to_string(),
            scope: Scope::Repo,
            fail_on: FailOn::Error,
            max_findings,
            yanked_index: None,
            checks,
        };

        let report = evaluate(&model, &cfg);

        // findings_total should equal num_deps (each dep has a wildcard)
        prop_assert_eq!(
            report.data.findings_total,
            num_deps as u32,
            "findings_total should equal the number of wildcard violations"
        );
    }
}

// ============================================================================
// Property tests: No-panic guarantees (fuzz-like)
// ============================================================================

proptest! {
    /// The engine must never panic on any valid DepSpec.
    #[test]
    fn engine_no_panic_on_arbitrary_dep_spec(
        name in arb_dep_name(),
        spec in arb_dep_spec(),
    ) {
        let model = WorkspaceModel {
            repo_root: RepoPath::new("."),
            workspace_dependencies: BTreeMap::new(),
            manifests: vec![ManifestModel {
                path: RepoPath::new("Cargo.toml"),
                package: Some(PackageMeta {
                    name: "test-pkg".to_string(),
                    publish: true,
                }),
                features: BTreeMap::new(),
                dependencies: vec![DependencyDecl {
                    kind: DepKind::Normal,
                    name,
                    spec,
                    location: Some(Location {
                        path: RepoPath::new("Cargo.toml"),
                        line: Some(10),
                        col: None,
                    }),
                    target: None,
                }],
            }],
        };

        let cfg = config_all_enabled(Severity::Warning);

        // This should never panic
        let report = std::panic::catch_unwind(|| evaluate(&model, &cfg));

        prop_assert!(
            report.is_ok(),
            "Engine panicked on valid input"
        );

        let report = report.unwrap();
        prop_assert!(report.findings.len() <= cfg.max_findings);
    }

    /// The engine must never panic on empty or minimal models.
    #[test]
    fn engine_no_panic_on_minimal_models(
        repo_root in prop::option::of(arb_safe_relative_path()),
    ) {
        let model = WorkspaceModel {
            repo_root: RepoPath::new(repo_root.unwrap_or_else(|| ".".to_string()).as_str()),
            workspace_dependencies: BTreeMap::new(),
            manifests: vec![],
        };

        let cfg = config_all_enabled(Severity::Error);

        let report = std::panic::catch_unwind(|| evaluate(&model, &cfg));

        prop_assert!(
            report.is_ok(),
            "Engine panicked on empty model"
        );

        let report = report.unwrap();
        prop_assert!(report.findings.is_empty());
        prop_assert_eq!(report.verdict, Verdict::Pass);
    }

    /// The engine must handle extreme max_findings values without panic.
    #[test]
    fn engine_no_panic_on_extreme_max_findings(
        max_findings in any::<usize>(),
        dep_name in arb_dep_name(),
    ) {
        // Bound max_findings to reasonable range to avoid allocation issues
        let max_findings = max_findings % 10000 + 1;

        let model = WorkspaceModel {
            repo_root: RepoPath::new("."),
            workspace_dependencies: BTreeMap::new(),
            manifests: vec![ManifestModel {
                path: RepoPath::new("Cargo.toml"),
                package: Some(PackageMeta {
                    name: "test-pkg".to_string(),
                    publish: true,
                }),
                features: BTreeMap::new(),
                dependencies: vec![DependencyDecl {
                    kind: DepKind::Normal,
                    name: dep_name,
                    spec: DepSpec {
                        version: Some("*".to_string()),
                        ..DepSpec::default()
                    },
                    location: Some(Location {
                        path: RepoPath::new("Cargo.toml"),
                        line: Some(10),
                        col: None,
                    }),
                    target: None,
                }],
            }],
        };

        let mut checks = BTreeMap::new();
        checks.insert(
            ids::CHECK_DEPS_NO_WILDCARDS.to_string(),
            CheckPolicy::enabled(Severity::Warning),
        );

        let cfg = EffectiveConfig {
            profile: "test".to_string(),
            scope: Scope::Repo,
            fail_on: FailOn::Error,
            max_findings,
            yanked_index: None,
            checks,
        };

        let report = std::panic::catch_unwind(|| evaluate(&model, &cfg));

        prop_assert!(
            report.is_ok(),
            "Engine panicked with max_findings = {}", max_findings
        );
    }

    /// The engine must handle many dependencies without panic.
    #[test]
    fn engine_no_panic_on_many_dependencies(num_deps in 0usize..100) {
        let deps: Vec<DependencyDecl> = (0..num_deps)
            .map(|i| DependencyDecl {
                kind: DepKind::Normal,
                name: format!("dep{}", i),
                spec: DepSpec {
                    version: Some("*".to_string()),
                    ..DepSpec::default()
                },
                location: Some(Location {
                    path: RepoPath::new("Cargo.toml"),
                    line: Some(i as u32 + 1),
                    col: None,
                }),
                target: None,
            })
            .collect();

        let model = WorkspaceModel {
            repo_root: RepoPath::new("."),
            workspace_dependencies: BTreeMap::new(),
            manifests: vec![ManifestModel {
                path: RepoPath::new("Cargo.toml"),
                package: Some(PackageMeta {
                    name: "test-pkg".to_string(),
                    publish: true,
                }),
                features: BTreeMap::new(),
                dependencies: deps,
            }],
        };

        let cfg = config_all_enabled(Severity::Warning);

        let report = std::panic::catch_unwind(|| evaluate(&model, &cfg));

        prop_assert!(report.is_ok(), "Engine panicked with {} dependencies", num_deps);
    }
}

// ============================================================================
// Property tests: UTF-8 and path safety
// ============================================================================

proptest! {
    /// Paths in findings must never be truncated or corrupted.
    #[test]
    fn paths_are_not_truncated(
        path_segment in prop::string::string_regex("[a-z]{1,10}").unwrap(),
        num_deps in 1usize..5,
    ) {
        let manifest_path = format!("crates/{}/Cargo.toml", path_segment);

        let deps: Vec<DependencyDecl> = (0..num_deps)
            .map(|i| DependencyDecl {
                kind: DepKind::Normal,
                name: format!("dep{}", i),
                spec: DepSpec {
                    version: Some("*".to_string()),
                    ..DepSpec::default()
                },
                location: Some(Location {
                    path: RepoPath::new(&manifest_path),
                    line: Some(i as u32 + 1),
                    col: None,
                }),
                target: None,
            })
            .collect();

        let model = WorkspaceModel {
            repo_root: RepoPath::new("."),
            workspace_dependencies: BTreeMap::new(),
            manifests: vec![ManifestModel {
                path: RepoPath::new(&manifest_path),
                package: Some(PackageMeta {
                    name: "test-pkg".to_string(),
                    publish: true,
                }),
                features: BTreeMap::new(),
                dependencies: deps,
            }],
        };

        let cfg = config_all_enabled(Severity::Warning);
        let report = evaluate(&model, &cfg);

        // All findings should have the full path
        for finding in &report.findings {
            if let Some(ref loc) = finding.location {
                prop_assert_eq!(
                    loc.path.as_str(),
                    manifest_path.as_str(),
                    "Path was truncated or corrupted"
                );
            }
        }
    }

    /// Messages must be valid UTF-8 and not corrupted.
    #[test]
    fn messages_are_valid_utf8(dep_name in arb_dep_name()) {
        let model = WorkspaceModel {
            repo_root: RepoPath::new("."),
            workspace_dependencies: BTreeMap::new(),
            manifests: vec![ManifestModel {
                path: RepoPath::new("Cargo.toml"),
                package: Some(PackageMeta {
                    name: "test-pkg".to_string(),
                    publish: true,
                }),
                features: BTreeMap::new(),
                dependencies: vec![DependencyDecl {
                    kind: DepKind::Normal,
                    name: dep_name,
                    spec: DepSpec {
                        version: Some("*".to_string()),
                        ..DepSpec::default()
                    },
                    location: Some(Location {
                        path: RepoPath::new("Cargo.toml"),
                        line: Some(10),
                        col: None,
                    }),
                    target: None,
                }],
            }],
        };

        let cfg = config_all_enabled(Severity::Warning);
        let report = evaluate(&model, &cfg);

        // All messages must be valid UTF-8 (verified by Rust's string type)
        for finding in &report.findings {
            prop_assert!(
                finding.message.is_char_boundary(0),
                "Message does not start at char boundary"
            );
            prop_assert!(
                finding.message.is_char_boundary(finding.message.len()),
                "Message does not end at char boundary"
            );
        }
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;
    use proptest::strategy::ValueTree;
    use proptest::test_runner::TestRunner;

    #[test]
    fn test_dep_spec_string_version_shape() {
        // String version: `"1.0"` -> DepSpec with version only
        let spec = DepSpec {
            version: Some("1.0.0".to_string()),
            ..DepSpec::default()
        };
        assert!(spec.version.is_some());
        assert!(spec.path.is_none());
        assert!(!spec.workspace);
    }

    #[test]
    fn test_dep_spec_inline_table_shape() {
        // Inline table: `{ version = "1.0" }`
        let spec = DepSpec {
            version: Some("1.0.0".to_string()),
            ..DepSpec::default()
        };
        assert!(spec.version.is_some());
        assert!(spec.path.is_none());
        assert!(!spec.workspace);
    }

    #[test]
    fn test_dep_spec_path_with_version_shape() {
        // Inline table with path: `{ path = "../foo", version = "1.0" }`
        let spec = DepSpec {
            version: Some("1.0.0".to_string()),
            path: Some("../foo".to_string()),
            ..DepSpec::default()
        };
        assert!(spec.version.is_some());
        assert!(spec.path.is_some());
        assert!(!spec.workspace);
    }

    #[test]
    fn test_dep_spec_workspace_ref_shape() {
        // Workspace reference: `{ workspace = true }`
        let spec = DepSpec {
            workspace: true,
            ..DepSpec::default()
        };
        assert!(spec.version.is_none());
        assert!(spec.path.is_none());
        assert!(spec.workspace);
    }

    #[test]
    fn test_findings_ordering_comprehensive() {
        // Test that findings are ordered correctly by all criteria
        let findings = vec![
            Finding {
                severity: Severity::Error,
                check_id: "b.check".to_string(),
                code: "code1".to_string(),
                message: "msg".to_string(),
                location: Some(Location {
                    path: RepoPath::new("Cargo.toml"),
                    line: Some(10),
                    col: None,
                }),
                help: None,
                url: None,
                fingerprint: None,
                data: serde_json::Value::Null,
            },
            Finding {
                severity: Severity::Warning,
                check_id: "a.check".to_string(),
                code: "code1".to_string(),
                message: "msg".to_string(),
                location: Some(Location {
                    path: RepoPath::new("Cargo.toml"),
                    line: Some(10),
                    col: None,
                }),
                help: None,
                url: None,
                fingerprint: None,
                data: serde_json::Value::Null,
            },
            Finding {
                severity: Severity::Error,
                check_id: "a.check".to_string(),
                code: "code1".to_string(),
                message: "msg".to_string(),
                location: Some(Location {
                    path: RepoPath::new("Cargo.toml"),
                    line: Some(5),
                    col: None,
                }),
                help: None,
                url: None,
                fingerprint: None,
                data: serde_json::Value::Null,
            },
            Finding {
                severity: Severity::Error,
                check_id: "c.check".to_string(),
                code: "code1".to_string(),
                message: "msg".to_string(),
                location: None,
                help: None,
                url: None,
                fingerprint: None,
                data: serde_json::Value::Null,
            },
            Finding {
                severity: Severity::Info,
                check_id: "a.check".to_string(),
                code: "code1".to_string(),
                message: "msg".to_string(),
                location: Some(Location {
                    path: RepoPath::new("crates/foo/Cargo.toml"),
                    line: Some(1),
                    col: None,
                }),
                help: None,
                url: None,
                fingerprint: None,
                data: serde_json::Value::Null,
            },
        ];

        let cmp = |a: &Finding, b: &Finding| {
            let rank = |s: Severity| match s {
                Severity::Error => 0u8,
                Severity::Warning => 1u8,
                Severity::Info => 2u8,
            };
            let (ap, al) = match &a.location {
                Some(l) => (l.path.as_str(), l.line.unwrap_or(u32::MAX)),
                None => ("~", u32::MAX),
            };
            let (bp, bl) = match &b.location {
                Some(l) => (l.path.as_str(), l.line.unwrap_or(u32::MAX)),
                None => ("~", u32::MAX),
            };
            rank(a.severity)
                .cmp(&rank(b.severity))
                .then(ap.cmp(bp))
                .then(al.cmp(&bl))
                .then(a.check_id.cmp(&b.check_id))
                .then(a.code.cmp(&b.code))
                .then(a.message.cmp(&b.message))
        };

        let _ = cmp(&findings[0], &findings[3]);

        let mut sorted = findings.clone();
        sorted.sort_by(|a, b| cmp(a, b));

        // Expected order: Error Cargo.toml:5, Error Cargo.toml:10 (b.check), Error with no location, Warning Cargo.toml:10 (a.check), Info crates/foo/Cargo.toml:1
        assert_eq!(sorted[0].location.as_ref().unwrap().line, Some(5));
        assert_eq!(sorted[1].check_id, "b.check");
        assert_eq!(sorted[1].location.as_ref().unwrap().line, Some(10));
        assert!(sorted[2].location.is_none());
        assert_eq!(sorted[2].check_id, "c.check");
        assert_eq!(sorted[3].check_id, "a.check");
        assert_eq!(sorted[3].location.as_ref().unwrap().line, Some(10));
        assert_eq!(
            sorted[4].location.as_ref().unwrap().path.as_str(),
            "crates/foo/Cargo.toml"
        );
    }

    #[test]
    fn strategy_helpers_produce_values() {
        let mut runner = TestRunner::default();

        let _ = arb_relative_path().new_tree(&mut runner).unwrap().current();
        let _ = arb_dep_kind().new_tree(&mut runner).unwrap().current();
        let _ = arb_dependency_decl()
            .new_tree(&mut runner)
            .unwrap()
            .current();
    }

    #[test]
    fn assert_sorted_detects_out_of_order() {
        let findings = vec![
            Finding {
                severity: Severity::Warning,
                check_id: "b.check".to_string(),
                code: "code1".to_string(),
                message: "msg".to_string(),
                location: Some(Location {
                    path: RepoPath::new("Cargo.toml"),
                    line: Some(10),
                    col: None,
                }),
                help: None,
                url: None,
                fingerprint: None,
                data: serde_json::Value::Null,
            },
            Finding {
                severity: Severity::Error,
                check_id: "a.check".to_string(),
                code: "code1".to_string(),
                message: "msg".to_string(),
                location: Some(Location {
                    path: RepoPath::new("Cargo.toml"),
                    line: Some(5),
                    col: None,
                }),
                help: None,
                url: None,
                fingerprint: None,
                data: serde_json::Value::Null,
            },
        ];

        assert!(assert_sorted(&findings).is_err());
    }
}
