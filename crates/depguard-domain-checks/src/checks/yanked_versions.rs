use crate::checks::utils::{build_allowlist, is_allowed, section_name, spec_to_json};
use crate::fingerprint::fingerprint_for_dep;
use crate::model::WorkspaceModel;
use crate::policy::EffectiveConfig;
use depguard_types::{Finding, ids};
use serde_json::json;

pub fn run(model: &WorkspaceModel, cfg: &EffectiveConfig, out: &mut Vec<Finding>) {
    let Some(policy) = cfg.check_policy(ids::CHECK_DEPS_YANKED_VERSIONS) else {
        return;
    };
    let Some(index) = cfg.yanked_index.as_ref() else {
        // No yanked index provided; check is configured but has no data source.
        return;
    };
    let allow = build_allowlist(&policy.allow);

    for manifest in &model.manifests {
        for dep in &manifest.dependencies {
            if dep.spec.workspace {
                continue;
            }

            let Some(version_req) = dep.spec.version.as_deref() else {
                continue;
            };
            let Some(pinned) = pinned_version(version_req) else {
                continue;
            };

            if !index.is_yanked(&dep.name, pinned) {
                continue;
            }
            if is_allowed(allow.as_ref(), &dep.name) {
                continue;
            }

            let fingerprint = fingerprint_for_dep(
                ids::CHECK_DEPS_YANKED_VERSIONS,
                ids::CODE_VERSION_YANKED,
                manifest.path.as_str(),
                &dep.name,
                Some(pinned),
            );

            out.push(Finding {
                severity: policy.severity,
                check_id: ids::CHECK_DEPS_YANKED_VERSIONS.to_string(),
                code: ids::CODE_VERSION_YANKED.to_string(),
                message: format!(
                    "dependency '{}' pins yanked version '{}'",
                    dep.name, version_req
                ),
                location: dep.location.clone(),
                help: Some(
                    "Upgrade to a non-yanked version and keep the dependency pinned.".to_string(),
                ),
                url: None,
                fingerprint: Some(fingerprint),
                data: {
                    let mut d = json!({
                        "current_spec": spec_to_json(&dep.spec),
                        "dependency": dep.name,
                        "fix_action": ids::FIX_ACTION_UPGRADE_YANKED_VERSION,
                        "fix_hint": "Upgrade to a non-yanked pinned version",
                        "manifest": manifest.path.as_str(),
                        "pinned_version": pinned,
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

fn pinned_version(version_req: &str) -> Option<&str> {
    let trimmed = version_req.trim();
    let rest = trimmed.strip_prefix('=')?.trim();
    if rest.is_empty() { None } else { Some(rest) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pinned_version_extracts_exact_pin() {
        assert_eq!(pinned_version("=1.2.3"), Some("1.2.3"));
        assert_eq!(pinned_version("= 1.2.3"), Some("1.2.3"));
        assert_eq!(pinned_version("^1.2.3"), None);
        assert_eq!(pinned_version("1.2.3"), None);
        assert_eq!(pinned_version("="), None);
    }
}
