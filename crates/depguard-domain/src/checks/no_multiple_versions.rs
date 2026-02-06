use crate::checks::utils::{build_allowlist, is_allowed};
use crate::model::WorkspaceModel;
use crate::policy::EffectiveConfig;
use depguard_types::{Finding, ids};
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};

pub fn run(model: &WorkspaceModel, cfg: &EffectiveConfig, out: &mut Vec<Finding>) {
    let Some(policy) = cfg.check_policy(ids::CHECK_DEPS_NO_MULTIPLE_VERSIONS) else {
        return;
    };
    let allow = build_allowlist(&policy.allow);

    // Build a map of crate_name -> set of (version, manifest_path)
    let mut version_map: BTreeMap<String, BTreeSet<(String, String)>> = BTreeMap::new();

    for manifest in &model.manifests {
        for dep in &manifest.dependencies {
            // Skip workspace deps (they use workspace version)
            if dep.spec.workspace {
                continue;
            }

            // Skip deps without explicit versions
            let Some(version) = &dep.spec.version else {
                continue;
            };

            version_map
                .entry(dep.name.clone())
                .or_default()
                .insert((version.clone(), manifest.path.as_str().to_string()));
        }
    }

    // Find crates with multiple distinct versions
    for (crate_name, versions) in &version_map {
        // Extract unique versions (ignoring which manifest they came from)
        let unique_versions: BTreeSet<&str> = versions.iter().map(|(v, _)| v.as_str()).collect();

        if unique_versions.len() <= 1 {
            continue;
        }

        // Allowlist hook
        if is_allowed(allow.as_ref(), crate_name) {
            continue;
        }

        // Create a finding for this duplicate
        let version_list: Vec<String> = unique_versions.iter().map(|v| v.to_string()).collect();

        // Use a stable fingerprint based on crate name and sorted versions
        let fingerprint = format!(
            "{}|{}|{}",
            ids::CHECK_DEPS_NO_MULTIPLE_VERSIONS,
            ids::CODE_DUPLICATE_DIFFERENT_VERSIONS,
            crate_name
        );
        let fingerprint_hash = {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut hasher = DefaultHasher::new();
            fingerprint.hash(&mut hasher);
            format!("{:016x}", hasher.finish())
        };

        out.push(Finding {
            severity: policy.severity,
            check_id: ids::CHECK_DEPS_NO_MULTIPLE_VERSIONS.to_string(),
            code: ids::CODE_DUPLICATE_DIFFERENT_VERSIONS.to_string(),
            message: format!(
                "crate '{}' has multiple versions across workspace: {}",
                crate_name,
                version_list.join(", ")
            ),
            location: None, // Workspace-level finding, no specific location
            help: Some(
                "Align all workspace members to use the same version via [workspace.dependencies]."
                    .to_string(),
            ),
            url: None,
            fingerprint: Some(fingerprint_hash),
            data: json!({
                "crate": crate_name,
                "versions": version_list,
                "occurrences": versions.iter().collect::<Vec<_>>(),
            }),
        });
    }
}
