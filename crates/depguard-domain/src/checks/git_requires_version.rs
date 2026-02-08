use crate::checks::utils::{build_allowlist, is_allowed, section_name, spec_to_json};
use crate::fingerprint::fingerprint_for_dep;
use crate::model::WorkspaceModel;
use crate::policy::EffectiveConfig;
use depguard_types::{Finding, ids};
use serde_json::json;

pub fn run(model: &WorkspaceModel, cfg: &EffectiveConfig, out: &mut Vec<Finding>) {
    let Some(policy) = cfg.check_policy(ids::CHECK_DEPS_GIT_REQUIRES_VERSION) else {
        return;
    };
    let allow = build_allowlist(&policy.allow);

    for manifest in &model.manifests {
        // Common policy: only enforce for crates that can be published.
        if !policy.ignore_publish_false && !manifest.is_publishable() {
            continue;
        }

        for dep in &manifest.dependencies {
            // Check if it's a git dependency without a version
            if dep.spec.git.is_some() && dep.spec.version.is_none() && !dep.spec.workspace {
                // Allowlist hook
                if is_allowed(allow.as_ref(), &dep.name) {
                    continue;
                }
                let fingerprint = fingerprint_for_dep(
                    ids::CHECK_DEPS_GIT_REQUIRES_VERSION,
                    ids::CODE_GIT_WITHOUT_VERSION,
                    manifest.path.as_str(),
                    &dep.name,
                    dep.spec.git.as_deref(),
                );

                out.push(Finding {
                    severity: policy.severity,
                    check_id: ids::CHECK_DEPS_GIT_REQUIRES_VERSION.to_string(),
                    code: ids::CODE_GIT_WITHOUT_VERSION.to_string(),
                    message: format!(
                        "dependency '{}' uses a git dependency without an explicit version",
                        dep.name
                    ),
                    location: dep.location.clone(),
                    help: Some(
                        "Add an explicit version alongside `git = ...`, or use `workspace = true` with a workspace dependency."
                            .to_string(),
                    ),
                    url: None,
                    fingerprint: Some(fingerprint),
                    data: {
                        let mut d = json!({
                            "current_spec": spec_to_json(&dep.spec),
                            "dependency": dep.name,
                            "fix_action": ids::FIX_ACTION_ADD_VERSION_WITH_GIT,
                            "fix_hint": "Add version alongside the git dependency",
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
