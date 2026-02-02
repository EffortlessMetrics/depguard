use crate::model::WorkspaceModel;
use crate::policy::EffectiveConfig;
use depguard_types::{ids, Finding, Severity};
use serde_json::json;

pub fn run(model: &WorkspaceModel, cfg: &EffectiveConfig, out: &mut Vec<Finding>) {
    let Some(policy) = cfg.check_policy(ids::CHECK_DEPS_NO_WILDCARDS) else {
        return;
    };

    for manifest in &model.manifests {
        for dep in &manifest.dependencies {
            let Some(version) = dep.spec.version.as_deref() else { continue };
            if version.contains('*') {
                out.push(Finding {
                    severity: policy.severity,
                    check_id: ids::CHECK_DEPS_NO_WILDCARDS.to_string(),
                    code: ids::CODE_WILDCARD_VERSION.to_string(),
                    message: format!("dependency '{}' uses a wildcard version: {}", dep.name, version),
                    location: dep.location.clone(),
                    help: Some("Replace wildcard versions with an explicit semver requirement.".to_string()),
                    url: None,
                    fingerprint: None,
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
