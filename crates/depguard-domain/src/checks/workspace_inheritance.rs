use crate::model::WorkspaceModel;
use crate::policy::EffectiveConfig;
use depguard_types::{ids, Finding};
use serde_json::json;

pub fn run(model: &WorkspaceModel, cfg: &EffectiveConfig, out: &mut Vec<Finding>) {
    let Some(policy) = cfg.check_policy(ids::CHECK_DEPS_WORKSPACE_INHERITANCE) else {
        return;
    };

    if model.workspace_dependencies.is_empty() {
        return;
    }

    for manifest in &model.manifests {
        for dep in &manifest.dependencies {
            if !model.workspace_dependencies.contains_key(&dep.name) {
                continue;
            }
            if dep.spec.workspace {
                continue;
            }

            // Allowlist hook (simple exact match for scaffold).
            if policy.allow.iter().any(|a| a == &dep.name) {
                continue;
            }

            out.push(Finding {
                severity: policy.severity,
                check_id: ids::CHECK_DEPS_WORKSPACE_INHERITANCE.to_string(),
                code: ids::CODE_MISSING_WORKSPACE_TRUE.to_string(),
                message: format!(
                    "dependency '{}' exists in [workspace.dependencies] but is not declared with `workspace = true`",
                    dep.name
                ),
                location: dep.location.clone(),
                help: Some(
                    "Prefer `workspace = true` to inherit the workspace dependency version and features."
                        .to_string(),
                ),
                url: None,
                fingerprint: None,
                data: json!({
                    "dependency": dep.name,
                    "manifest": manifest.path.as_str(),
                }),
            });
        }
    }
}
