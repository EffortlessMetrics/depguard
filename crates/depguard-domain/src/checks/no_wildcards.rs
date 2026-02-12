use crate::checks::utils::{build_allowlist, is_allowed, section_name, spec_to_json};
use crate::fingerprint::fingerprint_for_dep;
use crate::model::WorkspaceModel;
use crate::policy::EffectiveConfig;
use depguard_types::{Finding, ids};
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
                    data: {
                        let mut d = json!({
                            "current_spec": spec_to_json(&dep.spec),
                            "dependency": dep.name,
                            "fix_action": ids::FIX_ACTION_PIN_VERSION,
                            "fix_hint": "Pin to a specific semver requirement",
                            "manifest": manifest.path.as_str(),
                            "section": section_name(dep.kind),
                        });
                        if let Some(ref t) = dep.target {
                            d["target"] = json!(t);
                        }
                        d
                    },
                });
            }
        }
    }
}
