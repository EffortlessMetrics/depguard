use crate::model::WorkspaceModel;
use crate::policy::EffectiveConfig;
use depguard_types::{ids, Finding};
use serde_json::json;

pub fn run(model: &WorkspaceModel, cfg: &EffectiveConfig, out: &mut Vec<Finding>) {
    let Some(policy) = cfg.check_policy(ids::CHECK_DEPS_PATH_REQUIRES_VERSION) else {
        return;
    };

    for manifest in &model.manifests {
        // Common policy: only enforce for crates that can be published.
        if !manifest.is_publishable() {
            continue;
        }

        for dep in &manifest.dependencies {
            if dep.spec.path.is_some() && dep.spec.version.is_none() && !dep.spec.workspace {
                // Allowlist hook (simple exact match for scaffold).
                if policy.allow.iter().any(|a| a == &dep.name) {
                    continue;
                }

                out.push(Finding {
                    severity: policy.severity,
                    check_id: ids::CHECK_DEPS_PATH_REQUIRES_VERSION.to_string(),
                    code: ids::CODE_PATH_WITHOUT_VERSION.to_string(),
                    message: format!(
                        "dependency '{}' uses a path dependency without an explicit version",
                        dep.name
                    ),
                    location: dep.location.clone(),
                    help: Some(
                        "Add an explicit version alongside `path = ...`, or use `workspace = true` with a workspace dependency."
                            .to_string(),
                    ),
                    url: None,
                    fingerprint: None,
                    data: json!({
                        "dependency": dep.name,
                        "manifest": manifest.path.as_str(),
                        "path": dep.spec.path,
                    }),
                });
            }
        }
    }
}
