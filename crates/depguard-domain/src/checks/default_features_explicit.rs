use crate::checks::utils::{build_allowlist, is_allowed};
use crate::fingerprint::fingerprint_for_dep;
use crate::model::WorkspaceModel;
use crate::policy::EffectiveConfig;
use depguard_types::{ids, Finding};
use serde_json::json;

pub fn run(model: &WorkspaceModel, cfg: &EffectiveConfig, out: &mut Vec<Finding>) {
    let Some(policy) = cfg.check_policy(ids::CHECK_DEPS_DEFAULT_FEATURES_EXPLICIT) else {
        return;
    };
    let allow = build_allowlist(&policy.allow);

    for manifest in &model.manifests {
        for dep in &manifest.dependencies {
            // Skip workspace deps (they inherit from workspace definition)
            if dep.spec.workspace {
                continue;
            }

            // Check if this dep has inline options but no explicit default-features
            // Inline options include: features, optional, path, git
            let has_inline_options =
                dep.spec.path.is_some() || dep.spec.git.is_some() || dep.spec.optional;

            // If it's just a simple version string (no inline options), skip
            if !has_inline_options {
                continue;
            }

            // If default-features is explicitly set, skip
            if dep.spec.default_features.is_some() {
                continue;
            }

            // Allowlist hook
            if is_allowed(allow.as_ref(), &dep.name) {
                continue;
            }

            let fingerprint = fingerprint_for_dep(
                ids::CHECK_DEPS_DEFAULT_FEATURES_EXPLICIT,
                ids::CODE_DEFAULT_FEATURES_IMPLICIT,
                manifest.path.as_str(),
                &dep.name,
                None,
            );

            out.push(Finding {
                severity: policy.severity,
                check_id: ids::CHECK_DEPS_DEFAULT_FEATURES_EXPLICIT.to_string(),
                code: ids::CODE_DEFAULT_FEATURES_IMPLICIT.to_string(),
                message: format!(
                    "dependency '{}' has inline options but no explicit default-features declaration",
                    dep.name
                ),
                location: dep.location.clone(),
                help: Some(
                    "Add `default-features = true` or `default-features = false` to make the intent explicit."
                        .to_string(),
                ),
                url: None,
                fingerprint: Some(fingerprint),
                data: json!({
                    "dependency": dep.name,
                    "manifest": manifest.path.as_str(),
                }),
            });
        }
    }
}
