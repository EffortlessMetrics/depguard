use crate::checks::utils::{build_allowlist, is_allowed};
use crate::fingerprint::fingerprint_for_dep;
use crate::model::WorkspaceModel;
use crate::policy::EffectiveConfig;
use depguard_types::{Finding, ids};
use serde_json::json;

pub fn run(model: &WorkspaceModel, cfg: &EffectiveConfig, out: &mut Vec<Finding>) {
    let Some(policy) = cfg.check_policy(ids::CHECK_DEPS_OPTIONAL_UNUSED) else {
        return;
    };
    let allow = build_allowlist(&policy.allow);

    for manifest in &model.manifests {
        // Collect all dependency names referenced in features
        let mut referenced_deps: std::collections::HashSet<String> =
            std::collections::HashSet::new();

        for feature_deps in manifest.features.values() {
            for dep in feature_deps {
                // Features can reference deps as:
                // - "dep:crate-name" (explicit dep syntax)
                // - "crate-name/feature" (enable feature of dep)
                // - "crate-name" (enable dep as feature, legacy)
                if let Some(crate_name) = dep.strip_prefix("dep:") {
                    referenced_deps.insert(crate_name.to_string());
                } else if let Some((crate_name, _feature)) = dep.split_once('/') {
                    referenced_deps.insert(crate_name.to_string());
                } else {
                    // Could be a feature name or a dep name
                    referenced_deps.insert(dep.clone());
                }
            }
        }

        for dep in &manifest.dependencies {
            // Only check optional dependencies
            if !dep.spec.optional {
                continue;
            }

            // Check if this optional dep is referenced in any feature
            if referenced_deps.contains(&dep.name) {
                continue;
            }

            // Allowlist hook
            if is_allowed(allow.as_ref(), &dep.name) {
                continue;
            }

            let fingerprint = fingerprint_for_dep(
                ids::CHECK_DEPS_OPTIONAL_UNUSED,
                ids::CODE_OPTIONAL_NOT_IN_FEATURES,
                manifest.path.as_str(),
                &dep.name,
                None,
            );

            out.push(Finding {
                severity: policy.severity,
                check_id: ids::CHECK_DEPS_OPTIONAL_UNUSED.to_string(),
                code: ids::CODE_OPTIONAL_NOT_IN_FEATURES.to_string(),
                message: format!(
                    "optional dependency '{}' is not referenced in any feature",
                    dep.name
                ),
                location: dep.location.clone(),
                help: Some(
                    "Add a feature that enables this dependency, or remove `optional = true`."
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
