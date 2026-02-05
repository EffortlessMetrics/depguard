use crate::checks::utils::{build_allowlist, is_allowed};
use crate::fingerprint::fingerprint_for_dep;
use crate::model::WorkspaceModel;
use crate::policy::EffectiveConfig;
use depguard_types::{ids, Finding};
use serde_json::json;

pub fn run(model: &WorkspaceModel, cfg: &EffectiveConfig, out: &mut Vec<Finding>) {
    let Some(policy) = cfg.check_policy(ids::CHECK_DEPS_NO_WILDCARDS) else {
        return;
    };
    let allow = build_allowlist(&policy.allow);

    for manifest in &model.manifests {
        for dep in &manifest.dependencies {
            let Some(version) = dep.spec.version.as_deref() else {
                continue;
            };
            if version.contains('*') {
                if is_allowed(allow.as_ref(), &dep.name) {
                    continue;
                }
                let fingerprint = fingerprint_for_dep(
                    ids::CHECK_DEPS_NO_WILDCARDS,
                    ids::CODE_WILDCARD_VERSION,
                    manifest.path.as_str(),
                    &dep.name,
                    dep.spec.path.as_deref(),
                );
                out.push(Finding {
                    severity: policy.severity,
                    check_id: ids::CHECK_DEPS_NO_WILDCARDS.to_string(),
                    code: ids::CODE_WILDCARD_VERSION.to_string(),
                    message: format!(
                        "dependency '{}' uses a wildcard version: {}",
                        dep.name, version
                    ),
                    location: dep.location.clone(),
                    help: Some(
                        "Replace wildcard versions with an explicit semver requirement."
                            .to_string(),
                    ),
                    url: None,
                    fingerprint: Some(fingerprint),
                    data: json!({
                        "dependency": dep.name,
                        "version": version,
                        "manifest": manifest.path.as_str(),
                    }),
                });
            }
        }
    }
}
