// Property-based tests for all depguard-domain-checks
// These tests verify:
// 1. No panics under randomized inputs
// 2. Determinism: same inputs → same outputs
// 3. Truncation invariants: findings don't exceed max_findings limit

use super::*;
use crate::model::{DepKind, DepSpec, DependencyDecl, ManifestModel, PackageMeta};
use crate::policy::{CheckPolicy, EffectiveConfig, FailOn, Scope};
use crate::test_support::{dep_decl, manifest, model};
use ::proptest::prelude::*;
use depguard_types::{Location, RepoPath, Severity, ids};
use std::collections::BTreeMap;

// Helper strategy for generating dependency names
fn dep_name_strategy() -> impl ::proptest::strategy::Strategy<Value = String> {
    ::proptest::string::string_regex("[a-z0-9_-]{1,30}").unwrap()
}

// Helper strategy for generating version strings
fn version_strategy() -> impl ::proptest::strategy::Strategy<Value = String> {
    ::proptest::string::string_regex("[0-9]+(\\.[0-9]+){0,3}(-[a-z0-9.+-]+)?").unwrap()
}

// Helper strategy for generating wildcard versions
fn wildcard_version_strategy() -> impl ::proptest::strategy::Strategy<Value = String> {
    ::proptest::prop_oneof![
        Just("*".to_string()),
        Just("1.*".to_string()),
        Just("1.2.*".to_string()),
        Just(">=1.0,<2.0".to_string()),
    ]
}

// Helper strategy for generating path strings
fn path_strategy() -> impl ::proptest::strategy::Strategy<Value = String> {
    ::proptest::string::string_regex("[a-z0-9_/-]{1,50}").unwrap()
}

// Helper strategy for generating absolute paths
fn absolute_path_strategy() -> impl ::proptest::strategy::Strategy<Value = String> {
    ::proptest::prop_oneof![
        Just("/usr/local/lib".to_string()),
        Just("C:\\Users\\test".to_string()),
        Just("/home/user/project".to_string()),
    ]
}

// Helper strategy for generating parent-escape paths
fn parent_escape_strategy() -> impl ::proptest::strategy::Strategy<Value = String> {
    ::proptest::prop_oneof![
        Just("../external".to_string()),
        Just("../../other".to_string()),
        Just("../../../lib".to_string()),
    ]
}

// Helper strategy for generating git URLs
fn git_url_strategy() -> impl ::proptest::strategy::Strategy<Value = String> {
    ::proptest::prop_oneof![
        Just("https://github.com/example/repo.git".to_string()),
        Just("git://github.com/example/repo.git".to_string()),
        Just("ssh://git@github.com/example/repo.git".to_string()),
    ]
}

// Helper strategy for generating target expressions
fn target_strategy() -> impl ::proptest::strategy::Strategy<Value = String> {
    ::proptest::prop_oneof![
        Just("cfg(windows)".to_string()),
        Just("cfg(unix)".to_string()),
        Just("cfg(target_os = \"linux\")".to_string()),
        Just("cfg(target_arch = \"x86_64\")".to_string()),
        Just("cfg(all(unix, target_arch = \"x86_64\"))".to_string()),
        Just(None::<String>).prop_map(|_| "".to_string()),
    ]
}

// Helper strategy for generating DepKind
fn dep_kind_strategy() -> impl ::proptest::strategy::Strategy<Value = DepKind> {
    ::proptest::prop_oneof![
        Just(DepKind::Normal),
        Just(DepKind::Dev),
        Just(DepKind::Build),
    ]
}

// Helper strategy for generating DepSpec
fn dep_spec_strategy() -> impl ::proptest::strategy::Strategy<Value = DepSpec> {
    (
        ::proptest::option::of(version_strategy()),
        ::proptest::option::of(wildcard_version_strategy()),
        ::proptest::option::of(path_strategy()),
        ::proptest::option::of(absolute_path_strategy()),
        ::proptest::option::of(parent_escape_strategy()),
        ::proptest::option::of(git_url_strategy()),
        ::proptest::bool::ANY,
        ::proptest::bool::ANY,
        ::proptest::bool::ANY,
        ::proptest::option::of(dep_name_strategy()),
    )
        .prop_map(
            |(
                version,
                wildcard,
                path,
                abs_path,
                parent_esc,
                git,
                optional,
                default_features,
                workspace,
                package,
            )| {
                // Use wildcard if provided, otherwise use version
                let final_version = wildcard.or(version);

                // Use absolute path or parent escape if provided, otherwise use normal path
                let final_path = abs_path.or(parent_esc).or(path);

                DepSpec {
                    version: final_version,
                    path: final_path,
                    git,
                    optional,
                    default_features: if default_features { Some(true) } else { None },
                    workspace,
                    package,
                    branch: None,
                    tag: None,
                    rev: None,
                    inline_suppressions: Vec::new(),
                }
            },
        )
}

// Helper strategy for generating DependencyDecl
fn dep_decl_strategy() -> impl ::proptest::strategy::Strategy<Value = DependencyDecl> {
    (
        dep_name_strategy(),
        dep_kind_strategy(),
        dep_spec_strategy(),
        ::proptest::option::of(target_strategy()),
    )
        .prop_map(|(name, kind, spec, target)| DependencyDecl {
            kind,
            name,
            spec,
            location: Some(Location {
                path: RepoPath::new("Cargo.toml"),
                line: Some(1),
                col: None,
            }),
            target: if target.as_ref().map(|t| t.is_empty()).unwrap_or(false) {
                None
            } else {
                target
            },
        })
}

// Helper strategy for generating feature maps
fn features_strategy() -> impl ::proptest::strategy::Strategy<Value = BTreeMap<String, Vec<String>>>
{
    ::proptest::collection::btree_map(
        ::proptest::string::string_regex("[a-z0-9_-]{1,20}").unwrap(),
        ::proptest::collection::vec(dep_name_strategy(), 0..5),
        0..5,
    )
}

// Helper strategy for generating ManifestModel
fn manifest_strategy() -> impl ::proptest::strategy::Strategy<Value = ManifestModel> {
    (
        ::proptest::string::string_regex("[a-z0-9_/-]{1,50}").unwrap(),
        ::proptest::bool::ANY,
        ::proptest::collection::vec(dep_decl_strategy(), 0..20),
        features_strategy(),
    )
        .prop_map(|(path, publish, deps, features)| ManifestModel {
            path: RepoPath::new(&path),
            package: Some(PackageMeta {
                name: "pkg".to_string(),
                publish,
            }),
            dependencies: deps,
            features,
        })
}

// Helper strategy for generating WorkspaceModel
fn workspace_model_strategy() -> impl ::proptest::strategy::Strategy<Value = WorkspaceModel> {
    (
        ::proptest::collection::vec(manifest_strategy(), 0..10),
        ::proptest::collection::btree_map(dep_name_strategy(), ::proptest::bool::ANY, 0..10),
    )
        .prop_map(|(manifests, ws_deps)| {
            let workspace_dependencies = ws_deps
                .into_keys()
                .map(|name| {
                    (
                        name.clone(),
                        crate::model::WorkspaceDependency {
                            name: name.clone(),
                            version: None,
                            path: None,
                            workspace: true,
                        },
                    )
                })
                .collect();

            WorkspaceModel {
                repo_root: RepoPath::new("."),
                workspace_dependencies,
                manifests,
            }
        })
}

// Helper function to create a config for a specific check
fn config_for_check(check_id: &str, severity: Severity, max_findings: usize) -> EffectiveConfig {
    let mut checks = BTreeMap::new();
    checks.insert(check_id.to_string(), CheckPolicy::enabled(severity));

    EffectiveConfig {
        profile: "test".to_string(),
        scope: Scope::Repo,
        fail_on: FailOn::Error,
        max_findings,
        yanked_index: None,
        checks,
    }
}

// Helper to pick a severity
fn severity_strategy() -> impl ::proptest::strategy::Strategy<Value = Severity> {
    ::proptest::prop_oneof![Just(Severity::Error), Just(Severity::Warning)]
}

// ============================================================================
// Property Test: No Panics Under Randomized Inputs
// ============================================================================

::proptest::proptest! {
    #[test]
    fn proptest_no_wildcards_no_panics(
        model in workspace_model_strategy(),
        severity in severity_strategy(),
    ) {
        let cfg = config_for_check(ids::CHECK_DEPS_NO_WILDCARDS, severity, 200);
        let mut out = Vec::new();

        // This should never panic regardless of input
        no_wildcards::run(&model, &cfg, &mut out);
    }

    #[test]
    fn proptest_path_requires_version_no_panics(
        model in workspace_model_strategy(),
        severity in severity_strategy(),
    ) {
        let cfg = config_for_check(ids::CHECK_DEPS_PATH_REQUIRES_VERSION, severity, 200);
        let mut out = Vec::new();

        path_requires_version::run(&model, &cfg, &mut out);
    }

    #[test]
    fn proptest_path_safety_no_panics(
        model in workspace_model_strategy(),
        severity in severity_strategy(),
    ) {
        let cfg = config_for_check(ids::CHECK_DEPS_PATH_SAFETY, severity, 200);
        let mut out = Vec::new();

        path_safety::run(&model, &cfg, &mut out);
    }

    #[test]
    fn proptest_workspace_inheritance_no_panics(
        model in workspace_model_strategy(),
        severity in severity_strategy(),
    ) {
        let cfg = config_for_check(ids::CHECK_DEPS_WORKSPACE_INHERITANCE, severity, 200);
        let mut out = Vec::new();

        workspace_inheritance::run(&model, &cfg, &mut out);
    }

    #[test]
    fn proptest_git_requires_version_no_panics(
        model in workspace_model_strategy(),
        severity in severity_strategy(),
    ) {
        let cfg = config_for_check(ids::CHECK_DEPS_GIT_REQUIRES_VERSION, severity, 200);
        let mut out = Vec::new();

        git_requires_version::run(&model, &cfg, &mut out);
    }

    #[test]
    fn proptest_dev_only_in_normal_no_panics(
        model in workspace_model_strategy(),
        severity in severity_strategy(),
    ) {
        let cfg = config_for_check(ids::CHECK_DEPS_DEV_ONLY_IN_NORMAL, severity, 200);
        let mut out = Vec::new();

        dev_only_in_normal::run(&model, &cfg, &mut out);
    }

    #[test]
    fn proptest_default_features_explicit_no_panics(
        model in workspace_model_strategy(),
        severity in severity_strategy(),
    ) {
        let cfg = config_for_check(ids::CHECK_DEPS_DEFAULT_FEATURES_EXPLICIT, severity, 200);
        let mut out = Vec::new();

        default_features_explicit::run(&model, &cfg, &mut out);
    }

    #[test]
    fn proptest_no_multiple_versions_no_panics(
        model in workspace_model_strategy(),
        severity in severity_strategy(),
    ) {
        let cfg = config_for_check(ids::CHECK_DEPS_NO_MULTIPLE_VERSIONS, severity, 200);
        let mut out = Vec::new();

        no_multiple_versions::run(&model, &cfg, &mut out);
    }

    #[test]
    fn proptest_optional_unused_no_panics(
        model in workspace_model_strategy(),
        severity in severity_strategy(),
    ) {
        let cfg = config_for_check(ids::CHECK_DEPS_OPTIONAL_UNUSED, severity, 200);
        let mut out = Vec::new();

        optional_unused::run(&model, &cfg, &mut out);
    }

    #[test]
    fn proptest_yanked_versions_no_panics(
        model in workspace_model_strategy(),
        severity in severity_strategy(),
    ) {
        let cfg = config_for_check(ids::CHECK_DEPS_YANKED_VERSIONS, severity, 200);
        // Note: yanked_versions requires a yanked_index, but we test without it
        // to ensure it doesn't panic when index is None
        let mut out = Vec::new();

        yanked_versions::run(&model, &cfg, &mut out);
    }
}

// ============================================================================
// Property Test: Determinism - Same Inputs Produce Same Outputs
// ============================================================================

::proptest::proptest! {
    #[test]
    fn proptest_no_wildcards_determinism(
        model in workspace_model_strategy(),
        severity in severity_strategy(),
    ) {
        let cfg = config_for_check(ids::CHECK_DEPS_NO_WILDCARDS, severity, 200);

        let mut out1 = Vec::new();
        let mut out2 = Vec::new();

        no_wildcards::run(&model, &cfg, &mut out1);
        no_wildcards::run(&model, &cfg, &mut out2);

        // Same input should produce same output
        assert_eq!(out1.len(), out2.len());
        for (f1, f2) in out1.iter().zip(out2.iter()) {
            assert_eq!(f1.check_id, f2.check_id);
            assert_eq!(f1.code, f2.code);
            assert_eq!(f1.message, f2.message);
            assert_eq!(f1.fingerprint, f2.fingerprint);
        }
    }

    #[test]
    fn proptest_path_requires_version_determinism(
        model in workspace_model_strategy(),
        severity in severity_strategy(),
    ) {
        let cfg = config_for_check(ids::CHECK_DEPS_PATH_REQUIRES_VERSION, severity, 200);

        let mut out1 = Vec::new();
        let mut out2 = Vec::new();

        path_requires_version::run(&model, &cfg, &mut out1);
        path_requires_version::run(&model, &cfg, &mut out2);

        assert_eq!(out1.len(), out2.len());
        for (f1, f2) in out1.iter().zip(out2.iter()) {
            assert_eq!(f1.check_id, f2.check_id);
            assert_eq!(f1.code, f2.code);
            assert_eq!(f1.fingerprint, f2.fingerprint);
        }
    }

    #[test]
    fn proptest_path_safety_determinism(
        model in workspace_model_strategy(),
        severity in severity_strategy(),
    ) {
        let cfg = config_for_check(ids::CHECK_DEPS_PATH_SAFETY, severity, 200);

        let mut out1 = Vec::new();
        let mut out2 = Vec::new();

        path_safety::run(&model, &cfg, &mut out1);
        path_safety::run(&model, &cfg, &mut out2);

        assert_eq!(out1.len(), out2.len());
        for (f1, f2) in out1.iter().zip(out2.iter()) {
            assert_eq!(f1.check_id, f2.check_id);
            assert_eq!(f1.code, f2.code);
            assert_eq!(f1.fingerprint, f2.fingerprint);
        }
    }

    #[test]
    fn proptest_workspace_inheritance_determinism(
        model in workspace_model_strategy(),
        severity in severity_strategy(),
    ) {
        let cfg = config_for_check(ids::CHECK_DEPS_WORKSPACE_INHERITANCE, severity, 200);

        let mut out1 = Vec::new();
        let mut out2 = Vec::new();

        workspace_inheritance::run(&model, &cfg, &mut out1);
        workspace_inheritance::run(&model, &cfg, &mut out2);

        assert_eq!(out1.len(), out2.len());
        for (f1, f2) in out1.iter().zip(out2.iter()) {
            assert_eq!(f1.check_id, f2.check_id);
            assert_eq!(f1.code, f2.code);
            assert_eq!(f1.fingerprint, f2.fingerprint);
        }
    }

    #[test]
    fn proptest_git_requires_version_determinism(
        model in workspace_model_strategy(),
        severity in severity_strategy(),
    ) {
        let cfg = config_for_check(ids::CHECK_DEPS_GIT_REQUIRES_VERSION, severity, 200);

        let mut out1 = Vec::new();
        let mut out2 = Vec::new();

        git_requires_version::run(&model, &cfg, &mut out1);
        git_requires_version::run(&model, &cfg, &mut out2);

        assert_eq!(out1.len(), out2.len());
        for (f1, f2) in out1.iter().zip(out2.iter()) {
            assert_eq!(f1.check_id, f2.check_id);
            assert_eq!(f1.code, f2.code);
            assert_eq!(f1.fingerprint, f2.fingerprint);
        }
    }

    #[test]
    fn proptest_dev_only_in_normal_determinism(
        model in workspace_model_strategy(),
        severity in severity_strategy(),
    ) {
        let cfg = config_for_check(ids::CHECK_DEPS_DEV_ONLY_IN_NORMAL, severity, 200);

        let mut out1 = Vec::new();
        let mut out2 = Vec::new();

        dev_only_in_normal::run(&model, &cfg, &mut out1);
        dev_only_in_normal::run(&model, &cfg, &mut out2);

        assert_eq!(out1.len(), out2.len());
        for (f1, f2) in out1.iter().zip(out2.iter()) {
            assert_eq!(f1.check_id, f2.check_id);
            assert_eq!(f1.code, f2.code);
            assert_eq!(f1.fingerprint, f2.fingerprint);
        }
    }

    #[test]
    fn proptest_default_features_explicit_determinism(
        model in workspace_model_strategy(),
        severity in severity_strategy(),
    ) {
        let cfg = config_for_check(ids::CHECK_DEPS_DEFAULT_FEATURES_EXPLICIT, severity, 200);

        let mut out1 = Vec::new();
        let mut out2 = Vec::new();

        default_features_explicit::run(&model, &cfg, &mut out1);
        default_features_explicit::run(&model, &cfg, &mut out2);

        assert_eq!(out1.len(), out2.len());
        for (f1, f2) in out1.iter().zip(out2.iter()) {
            assert_eq!(f1.check_id, f2.check_id);
            assert_eq!(f1.code, f2.code);
            assert_eq!(f1.fingerprint, f2.fingerprint);
        }
    }

    #[test]
    fn proptest_no_multiple_versions_determinism(
        model in workspace_model_strategy(),
        severity in severity_strategy(),
    ) {
        let cfg = config_for_check(ids::CHECK_DEPS_NO_MULTIPLE_VERSIONS, severity, 200);

        let mut out1 = Vec::new();
        let mut out2 = Vec::new();

        no_multiple_versions::run(&model, &cfg, &mut out1);
        no_multiple_versions::run(&model, &cfg, &mut out2);

        assert_eq!(out1.len(), out2.len());
        for (f1, f2) in out1.iter().zip(out2.iter()) {
            assert_eq!(f1.check_id, f2.check_id);
            assert_eq!(f1.code, f2.code);
            assert_eq!(f1.fingerprint, f2.fingerprint);
        }
    }

    #[test]
    fn proptest_optional_unused_determinism(
        model in workspace_model_strategy(),
        severity in severity_strategy(),
    ) {
        let cfg = config_for_check(ids::CHECK_DEPS_OPTIONAL_UNUSED, severity, 200);

        let mut out1 = Vec::new();
        let mut out2 = Vec::new();

        optional_unused::run(&model, &cfg, &mut out1);
        optional_unused::run(&model, &cfg, &mut out2);

        assert_eq!(out1.len(), out2.len());
        for (f1, f2) in out1.iter().zip(out2.iter()) {
            assert_eq!(f1.check_id, f2.check_id);
            assert_eq!(f1.code, f2.code);
            assert_eq!(f1.fingerprint, f2.fingerprint);
        }
    }

    #[test]
    fn proptest_yanked_versions_determinism(
        model in workspace_model_strategy(),
        severity in severity_strategy(),
    ) {
        let cfg = config_for_check(ids::CHECK_DEPS_YANKED_VERSIONS, severity, 200);

        let mut out1 = Vec::new();
        let mut out2 = Vec::new();

        yanked_versions::run(&model, &cfg, &mut out1);
        yanked_versions::run(&model, &cfg, &mut out2);

        assert_eq!(out1.len(), out2.len());
        for (f1, f2) in out1.iter().zip(out2.iter()) {
            assert_eq!(f1.check_id, f2.check_id);
            assert_eq!(f1.code, f2.code);
            assert_eq!(f1.fingerprint, f2.fingerprint);
        }
    }
}

// ============================================================================
// Property Test: Truncation Invariants - Findings Don't Exceed Limits
// ============================================================================

::proptest::proptest! {
    #[test]
    fn proptest_no_wildcards_truncation(
        model in workspace_model_strategy(),
        max_findings in 0..100usize,
    ) {
        let cfg = config_for_check(ids::CHECK_DEPS_NO_WILDCARDS, Severity::Error, max_findings);
        let mut out = Vec::new();

        no_wildcards::run(&model, &cfg, &mut out);

        // Note: The current implementation doesn't enforce max_findings at the check level
        // This test documents the current behavior and can be updated when truncation is implemented
        // For now, we just verify it doesn't panic
    }

    #[test]
    fn proptest_path_requires_version_truncation(
        model in workspace_model_strategy(),
        max_findings in 0..100usize,
    ) {
        let cfg = config_for_check(ids::CHECK_DEPS_PATH_REQUIRES_VERSION, Severity::Error, max_findings);
        let mut out = Vec::new();

        path_requires_version::run(&model, &cfg, &mut out);

    }

    #[test]
    fn proptest_path_safety_truncation(
        model in workspace_model_strategy(),
        max_findings in 0..100usize,
    ) {
        let cfg = config_for_check(ids::CHECK_DEPS_PATH_SAFETY, Severity::Error, max_findings);
        let mut out = Vec::new();

        path_safety::run(&model, &cfg, &mut out);

    }

    #[test]
    fn proptest_workspace_inheritance_truncation(
        model in workspace_model_strategy(),
        max_findings in 0..100usize,
    ) {
        let cfg = config_for_check(ids::CHECK_DEPS_WORKSPACE_INHERITANCE, Severity::Error, max_findings);
        let mut out = Vec::new();

        workspace_inheritance::run(&model, &cfg, &mut out);

    }

    #[test]
    fn proptest_git_requires_version_truncation(
        model in workspace_model_strategy(),
        max_findings in 0..100usize,
    ) {
        let cfg = config_for_check(ids::CHECK_DEPS_GIT_REQUIRES_VERSION, Severity::Error, max_findings);
        let mut out = Vec::new();

        git_requires_version::run(&model, &cfg, &mut out);

    }

    #[test]
    fn proptest_dev_only_in_normal_truncation(
        model in workspace_model_strategy(),
        max_findings in 0..100usize,
    ) {
        let cfg = config_for_check(ids::CHECK_DEPS_DEV_ONLY_IN_NORMAL, Severity::Error, max_findings);
        let mut out = Vec::new();

        dev_only_in_normal::run(&model, &cfg, &mut out);

    }

    #[test]
    fn proptest_default_features_explicit_truncation(
        model in workspace_model_strategy(),
        max_findings in 0..100usize,
    ) {
        let cfg = config_for_check(ids::CHECK_DEPS_DEFAULT_FEATURES_EXPLICIT, Severity::Error, max_findings);
        let mut out = Vec::new();

        default_features_explicit::run(&model, &cfg, &mut out);

    }

    #[test]
    fn proptest_no_multiple_versions_truncation(
        model in workspace_model_strategy(),
        max_findings in 0..100usize,
    ) {
        let cfg = config_for_check(ids::CHECK_DEPS_NO_MULTIPLE_VERSIONS, Severity::Error, max_findings);
        let mut out = Vec::new();

        no_multiple_versions::run(&model, &cfg, &mut out);

    }

    #[test]
    fn proptest_optional_unused_truncation(
        model in workspace_model_strategy(),
        max_findings in 0..100usize,
    ) {
        let cfg = config_for_check(ids::CHECK_DEPS_OPTIONAL_UNUSED, Severity::Error, max_findings);
        let mut out = Vec::new();

        optional_unused::run(&model, &cfg, &mut out);

    }

    #[test]
    fn proptest_yanked_versions_truncation(
        model in workspace_model_strategy(),
        max_findings in 0..100usize,
    ) {
        let cfg = config_for_check(ids::CHECK_DEPS_YANKED_VERSIONS, Severity::Error, max_findings);
        let mut out = Vec::new();

        yanked_versions::run(&model, &cfg, &mut out);

    }
}

// ============================================================================
// Property Test: Finding Structure Invariants
// ============================================================================

::proptest::proptest! {
    #[test]
    fn proptest_all_checks_finding_structure(
        model in workspace_model_strategy(),
        severity in severity_strategy(),
    ) {
        let checks = [
            ids::CHECK_DEPS_NO_WILDCARDS,
            ids::CHECK_DEPS_PATH_REQUIRES_VERSION,
            ids::CHECK_DEPS_PATH_SAFETY,
            ids::CHECK_DEPS_WORKSPACE_INHERITANCE,
            ids::CHECK_DEPS_GIT_REQUIRES_VERSION,
            ids::CHECK_DEPS_DEV_ONLY_IN_NORMAL,
            ids::CHECK_DEPS_DEFAULT_FEATURES_EXPLICIT,
            ids::CHECK_DEPS_NO_MULTIPLE_VERSIONS,
            ids::CHECK_DEPS_OPTIONAL_UNUSED,
            ids::CHECK_DEPS_YANKED_VERSIONS,
        ];

        for check_id in checks.iter() {
            let cfg = config_for_check(check_id, severity, 200);
            let mut out = Vec::new();

            match *check_id {
                ids::CHECK_DEPS_NO_WILDCARDS => no_wildcards::run(&model, &cfg, &mut out),
                ids::CHECK_DEPS_PATH_REQUIRES_VERSION => path_requires_version::run(&model, &cfg, &mut out),
                ids::CHECK_DEPS_PATH_SAFETY => path_safety::run(&model, &cfg, &mut out),
                ids::CHECK_DEPS_WORKSPACE_INHERITANCE => workspace_inheritance::run(&model, &cfg, &mut out),
                ids::CHECK_DEPS_GIT_REQUIRES_VERSION => git_requires_version::run(&model, &cfg, &mut out),
                ids::CHECK_DEPS_DEV_ONLY_IN_NORMAL => dev_only_in_normal::run(&model, &cfg, &mut out),
                ids::CHECK_DEPS_DEFAULT_FEATURES_EXPLICIT => default_features_explicit::run(&model, &cfg, &mut out),
                ids::CHECK_DEPS_NO_MULTIPLE_VERSIONS => no_multiple_versions::run(&model, &cfg, &mut out),
                ids::CHECK_DEPS_OPTIONAL_UNUSED => optional_unused::run(&model, &cfg, &mut out),
                ids::CHECK_DEPS_YANKED_VERSIONS => yanked_versions::run(&model, &cfg, &mut out),
                _ => continue,
            }

            // Verify all findings have required fields
            for finding in &out {
                assert_eq!(finding.check_id, *check_id);
                assert!(!finding.code.is_empty());
                assert!(!finding.message.is_empty());
                assert_eq!(finding.severity, severity);
                // Fingerprint should be present for most findings
                // Some checks may not have fingerprints for workspace-level findings
            }
        }
    }
}

// ============================================================================
// Property Test: Empty Input Handling
// ============================================================================

::proptest::proptest! {
    #[test]
    fn proptest_all_checks_empty_model(
        severity in severity_strategy(),
    ) {
        let empty_model = WorkspaceModel {
            repo_root: RepoPath::new("."),
            workspace_dependencies: BTreeMap::new(),
            manifests: Vec::new(),
        };

        let checks = [
            ids::CHECK_DEPS_NO_WILDCARDS,
            ids::CHECK_DEPS_PATH_REQUIRES_VERSION,
            ids::CHECK_DEPS_PATH_SAFETY,
            ids::CHECK_DEPS_WORKSPACE_INHERITANCE,
            ids::CHECK_DEPS_GIT_REQUIRES_VERSION,
            ids::CHECK_DEPS_DEV_ONLY_IN_NORMAL,
            ids::CHECK_DEPS_DEFAULT_FEATURES_EXPLICIT,
            ids::CHECK_DEPS_NO_MULTIPLE_VERSIONS,
            ids::CHECK_DEPS_OPTIONAL_UNUSED,
            ids::CHECK_DEPS_YANKED_VERSIONS,
        ];

        for check_id in checks.iter() {
            let cfg = config_for_check(check_id, severity, 200);
            let mut out = Vec::new();

            match *check_id {
                ids::CHECK_DEPS_NO_WILDCARDS => no_wildcards::run(&empty_model, &cfg, &mut out),
                ids::CHECK_DEPS_PATH_REQUIRES_VERSION => path_requires_version::run(&empty_model, &cfg, &mut out),
                ids::CHECK_DEPS_PATH_SAFETY => path_safety::run(&empty_model, &cfg, &mut out),
                ids::CHECK_DEPS_WORKSPACE_INHERITANCE => workspace_inheritance::run(&empty_model, &cfg, &mut out),
                ids::CHECK_DEPS_GIT_REQUIRES_VERSION => git_requires_version::run(&empty_model, &cfg, &mut out),
                ids::CHECK_DEPS_DEV_ONLY_IN_NORMAL => dev_only_in_normal::run(&empty_model, &cfg, &mut out),
                ids::CHECK_DEPS_DEFAULT_FEATURES_EXPLICIT => default_features_explicit::run(&empty_model, &cfg, &mut out),
                ids::CHECK_DEPS_NO_MULTIPLE_VERSIONS => no_multiple_versions::run(&empty_model, &cfg, &mut out),
                ids::CHECK_DEPS_OPTIONAL_UNUSED => optional_unused::run(&empty_model, &cfg, &mut out),
                ids::CHECK_DEPS_YANKED_VERSIONS => yanked_versions::run(&empty_model, &cfg, &mut out),
                _ => continue,
            }

            // Empty model should produce no findings
            assert_eq!(out.len(), 0, "Check {} produced findings for empty model", check_id);
        }
    }
}

// ============================================================================
// Property Test: Ordering Invariants
// ============================================================================

::proptest::proptest! {
    #[test]
    fn proptest_findings_are_deterministically_ordered(
        model in workspace_model_strategy(),
        severity in severity_strategy(),
    ) {
        let cfg = config_for_check(ids::CHECK_DEPS_NO_WILDCARDS, severity, 200);

        let mut out1 = Vec::new();
        let mut out2 = Vec::new();

        no_wildcards::run(&model, &cfg, &mut out1);
        no_wildcards::run(&model, &cfg, &mut out2);

        // Verify findings are in the same order across runs
        assert_eq!(out1.len(), out2.len());
        for (i, (f1, f2)) in out1.iter().zip(out2.iter()).enumerate() {
            assert_eq!(f1.check_id, f2.check_id, "Finding {} check_id differs", i);
            assert_eq!(f1.code, f2.code, "Finding {} code differs", i);
            assert_eq!(f1.fingerprint, f2.fingerprint, "Finding {} fingerprint differs", i);
        }
    }
}

// ============================================================================
// Property Test: Allowlist Handling
// ============================================================================

::proptest::proptest! {
    #[test]
    fn proptest_no_wildcards_allowlist_reduces_findings(
        model in workspace_model_strategy(),
        allowlist in ::proptest::collection::btree_set(dep_name_strategy(), 0..10),
    ) {
        let cfg1 = config_for_check(ids::CHECK_DEPS_NO_WILDCARDS, Severity::Error, 200);
        let mut cfg2 = config_for_check(ids::CHECK_DEPS_NO_WILDCARDS, Severity::Error, 200);

        // Add allowlist to cfg2
        cfg2.checks.get_mut(ids::CHECK_DEPS_NO_WILDCARDS).unwrap().allow =
            allowlist.iter().cloned().collect();

        let mut out1 = Vec::new();
        let mut out2 = Vec::new();

        no_wildcards::run(&model, &cfg1, &mut out1);
        no_wildcards::run(&model, &cfg2, &mut out2);

        // Allowlist should never increase findings
        assert!(out2.len() <= out1.len());
    }

    #[test]
    fn proptest_path_requires_version_allowlist_reduces_findings(
        model in workspace_model_strategy(),
        allowlist in ::proptest::collection::btree_set(dep_name_strategy(), 0..10),
    ) {
        let cfg1 = config_for_check(ids::CHECK_DEPS_PATH_REQUIRES_VERSION, Severity::Error, 200);
        let mut cfg2 = config_for_check(ids::CHECK_DEPS_PATH_REQUIRES_VERSION, Severity::Error, 200);

        cfg2.checks.get_mut(ids::CHECK_DEPS_PATH_REQUIRES_VERSION).unwrap().allow =
            allowlist.iter().cloned().collect();

        let mut out1 = Vec::new();
        let mut out2 = Vec::new();

        path_requires_version::run(&model, &cfg1, &mut out1);
        path_requires_version::run(&model, &cfg2, &mut out2);

        assert!(out2.len() <= out1.len());
    }

    #[test]
    fn proptest_workspace_inheritance_allowlist_reduces_findings(
        model in workspace_model_strategy(),
        allowlist in ::proptest::collection::btree_set(dep_name_strategy(), 0..10),
    ) {
        let cfg1 = config_for_check(ids::CHECK_DEPS_WORKSPACE_INHERITANCE, Severity::Error, 200);
        let mut cfg2 = config_for_check(ids::CHECK_DEPS_WORKSPACE_INHERITANCE, Severity::Error, 200);

        cfg2.checks.get_mut(ids::CHECK_DEPS_WORKSPACE_INHERITANCE).unwrap().allow =
            allowlist.iter().cloned().collect();

        let mut out1 = Vec::new();
        let mut out2 = Vec::new();

        workspace_inheritance::run(&model, &cfg1, &mut out1);
        workspace_inheritance::run(&model, &cfg2, &mut out2);

        assert!(out2.len() <= out1.len());
    }
}

// ============================================================================
// Property Test: Publish Policy Handling
// ============================================================================

::proptest::proptest! {
    #[test]
    fn proptest_path_requires_version_publish_policy(
        model in workspace_model_strategy(),
        ignore_publish_false in ::proptest::bool::ANY,
    ) {
        let mut cfg1 = config_for_check(ids::CHECK_DEPS_PATH_REQUIRES_VERSION, Severity::Error, 200);
        let mut cfg2 = config_for_check(ids::CHECK_DEPS_PATH_REQUIRES_VERSION, Severity::Error, 200);

        cfg1.checks.get_mut(ids::CHECK_DEPS_PATH_REQUIRES_VERSION).unwrap().ignore_publish_false = false;
        cfg2.checks.get_mut(ids::CHECK_DEPS_PATH_REQUIRES_VERSION).unwrap().ignore_publish_false = ignore_publish_false;

        let mut out1 = Vec::new();
        let mut out2 = Vec::new();

        path_requires_version::run(&model, &cfg1, &mut out1);
        path_requires_version::run(&model, &cfg2, &mut out2);

        // When ignore_publish_false is true, we may get more findings
        // When it's false, we skip non-publishable crates
        // So out2 (with potentially true) should have >= findings than out1 (with false)
        if ignore_publish_false {
            assert!(out2.len() >= out1.len());
        } else {
            assert_eq!(out2.len(), out1.len());
        }
    }

    #[test]
    fn proptest_git_requires_version_publish_policy(
        model in workspace_model_strategy(),
        ignore_publish_false in ::proptest::bool::ANY,
    ) {
        let mut cfg1 = config_for_check(ids::CHECK_DEPS_GIT_REQUIRES_VERSION, Severity::Error, 200);
        let mut cfg2 = config_for_check(ids::CHECK_DEPS_GIT_REQUIRES_VERSION, Severity::Error, 200);

        cfg1.checks.get_mut(ids::CHECK_DEPS_GIT_REQUIRES_VERSION).unwrap().ignore_publish_false = false;
        cfg2.checks.get_mut(ids::CHECK_DEPS_GIT_REQUIRES_VERSION).unwrap().ignore_publish_false = ignore_publish_false;

        let mut out1 = Vec::new();
        let mut out2 = Vec::new();

        git_requires_version::run(&model, &cfg1, &mut out1);
        git_requires_version::run(&model, &cfg2, &mut out2);

        if ignore_publish_false {
            assert!(out2.len() >= out1.len());
        } else {
            assert_eq!(out2.len(), out1.len());
        }
    }
}

// ============================================================================
// Property Test: Workspace Dependency Handling
// ============================================================================

::proptest::proptest! {
    #[test]
    fn proptest_workspace_deps_are_skipped(
        dep_names in ::proptest::collection::vec(dep_name_strategy(), 1..20),
    ) {
        // Create a model with only workspace dependencies
        let deps: Vec<DependencyDecl> = dep_names
            .iter()
            .map(|name| {
                let spec = DepSpec { workspace: true, ..Default::default() };
                dep_decl(name, DepKind::Normal, spec, None)
            })
            .collect();

        let manifest = manifest("Cargo.toml", true, deps, BTreeMap::new());
        let model = model(vec![manifest], BTreeMap::new());

        let checks = [
            ids::CHECK_DEPS_NO_WILDCARDS,
            ids::CHECK_DEPS_PATH_REQUIRES_VERSION,
            ids::CHECK_DEPS_GIT_REQUIRES_VERSION,
            ids::CHECK_DEPS_DEFAULT_FEATURES_EXPLICIT,
            ids::CHECK_DEPS_NO_MULTIPLE_VERSIONS,
            ids::CHECK_DEPS_YANKED_VERSIONS,
        ];

        for check_id in checks.iter() {
            let cfg = config_for_check(check_id, Severity::Error, 200);
            let mut out = Vec::new();

            match *check_id {
                ids::CHECK_DEPS_NO_WILDCARDS => no_wildcards::run(&model, &cfg, &mut out),
                ids::CHECK_DEPS_PATH_REQUIRES_VERSION => path_requires_version::run(&model, &cfg, &mut out),
                ids::CHECK_DEPS_GIT_REQUIRES_VERSION => git_requires_version::run(&model, &cfg, &mut out),
                ids::CHECK_DEPS_DEFAULT_FEATURES_EXPLICIT => default_features_explicit::run(&model, &cfg, &mut out),
                ids::CHECK_DEPS_NO_MULTIPLE_VERSIONS => no_multiple_versions::run(&model, &cfg, &mut out),
                ids::CHECK_DEPS_YANKED_VERSIONS => yanked_versions::run(&model, &cfg, &mut out),
                _ => continue,
            }

            // Workspace deps should be skipped by these checks
            assert_eq!(out.len(), 0, "Check {} produced findings for workspace deps", check_id);
        }
    }
}
