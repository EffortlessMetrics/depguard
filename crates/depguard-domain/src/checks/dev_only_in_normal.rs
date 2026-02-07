use crate::checks::utils::{build_allowlist, is_allowed, section_name, spec_to_json};
use crate::fingerprint::fingerprint_for_dep;
use crate::model::{DepKind, WorkspaceModel};
use crate::policy::EffectiveConfig;
use depguard_types::{Finding, ids};
use serde_json::json;

/// Crates that are typically only used in dev/test contexts.
const DEV_ONLY_CRATES: &[&str] = &[
    // Test frameworks
    "proptest",
    "quickcheck",
    "rstest",
    "test-case",
    "test-strategy",
    // Mocking
    "mockall",
    "mockito",
    "wiremock",
    "httpmock",
    // Snapshot testing
    "insta",
    "expect-test",
    // Benchmarking
    "criterion",
    "divan",
    "iai",
    // Test utilities
    "tempfile",
    "assert_cmd",
    "assert_fs",
    "predicates",
    "fake",
    "arbitrary",
    // Coverage
    "cargo-llvm-cov",
];

pub fn run(model: &WorkspaceModel, cfg: &EffectiveConfig, out: &mut Vec<Finding>) {
    let Some(policy) = cfg.check_policy(ids::CHECK_DEPS_DEV_ONLY_IN_NORMAL) else {
        return;
    };
    let allow = build_allowlist(&policy.allow);

    for manifest in &model.manifests {
        for dep in &manifest.dependencies {
            // Only check normal dependencies (not dev or build)
            if dep.kind != DepKind::Normal {
                continue;
            }

            // Check if this is a dev-only crate
            if !DEV_ONLY_CRATES.contains(&dep.name.as_str()) {
                continue;
            }

            // Allowlist hook
            if is_allowed(allow.as_ref(), &dep.name) {
                continue;
            }

            let fingerprint = fingerprint_for_dep(
                ids::CHECK_DEPS_DEV_ONLY_IN_NORMAL,
                ids::CODE_DEV_DEP_IN_NORMAL,
                manifest.path.as_str(),
                &dep.name,
                None,
            );

            out.push(Finding {
                severity: policy.severity,
                check_id: ids::CHECK_DEPS_DEV_ONLY_IN_NORMAL.to_string(),
                code: ids::CODE_DEV_DEP_IN_NORMAL.to_string(),
                message: format!(
                    "dependency '{}' is typically a dev-only crate but appears in [dependencies]",
                    dep.name
                ),
                location: dep.location.clone(),
                help: Some(
                    "Move this dependency to [dev-dependencies] unless it's genuinely needed in production code."
                        .to_string(),
                ),
                url: None,
                fingerprint: Some(fingerprint),
                data: json!({
                    "current_spec": spec_to_json(&dep.spec),
                    "dependency": dep.name,
                    "fix_hint": "Move to [dev-dependencies]",
                    "manifest": manifest.path.as_str(),
                    "section": section_name(dep.kind),
                }),
            });
        }
    }
}
