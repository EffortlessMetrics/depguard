use crate::checks::utils::{build_allowlist, is_allowed};
use crate::fingerprint::fingerprint_for_dep;
use crate::model::WorkspaceModel;
use crate::policy::EffectiveConfig;
use depguard_types::{ids, Finding};
use serde_json::json;

pub fn run(model: &WorkspaceModel, cfg: &EffectiveConfig, out: &mut Vec<Finding>) {
    let Some(policy) = cfg.check_policy(ids::CHECK_DEPS_PATH_SAFETY) else {
        return;
    };
    let allow = build_allowlist(&policy.allow);

    for manifest in &model.manifests {
        let manifest_depth = manifest_dir_depth(manifest.path.as_str());

        for dep in &manifest.dependencies {
            let Some(path) = dep.spec.path.as_deref() else {
                continue;
            };

            if is_allowed(allow.as_ref(), path) {
                continue;
            }

            if is_absolute_path(path) {
                let fingerprint = fingerprint_for_dep(
                    ids::CHECK_DEPS_PATH_SAFETY,
                    ids::CODE_ABSOLUTE_PATH,
                    manifest.path.as_str(),
                    &dep.name,
                    Some(path),
                );
                out.push(Finding {
                    severity: policy.severity,
                    check_id: ids::CHECK_DEPS_PATH_SAFETY.to_string(),
                    code: ids::CODE_ABSOLUTE_PATH.to_string(),
                    message: format!("dependency '{}' uses an absolute path: {}", dep.name, path),
                    location: dep.location.clone(),
                    help: Some("Use repo-relative paths. Absolute paths are not portable and may leak host layout.".to_string()),
                    url: None,
                    fingerprint: Some(fingerprint),
                    data: json!({
                        "dependency": dep.name,
                        "manifest": manifest.path.as_str(),
                        "path": path,
                    }),
                });
                continue;
            }

            if escapes_repo_root(manifest_depth, path) {
                let fingerprint = fingerprint_for_dep(
                    ids::CHECK_DEPS_PATH_SAFETY,
                    ids::CODE_PARENT_ESCAPE,
                    manifest.path.as_str(),
                    &dep.name,
                    Some(path),
                );
                out.push(Finding {
                    severity: policy.severity,
                    check_id: ids::CHECK_DEPS_PATH_SAFETY.to_string(),
                    code: ids::CODE_PARENT_ESCAPE.to_string(),
                    message: format!(
                        "dependency '{}' uses a path that escapes the repo root: {}",
                        dep.name, path
                    ),
                    location: dep.location.clone(),
                    help: Some("Avoid `..` segments that escape the repository root.".to_string()),
                    url: None,
                    fingerprint: Some(fingerprint),
                    data: json!({
                        "dependency": dep.name,
                        "manifest": manifest.path.as_str(),
                        "path": path,
                    }),
                });
            }
        }
    }
}

fn is_absolute_path(p: &str) -> bool {
    // Unix absolute
    if p.starts_with('/') {
        return true;
    }
    // Windows drive absolute: C:\ or C:/
    if p.len() >= 2 {
        let bytes = p.as_bytes();
        if bytes[1] == b':' {
            return true;
        }
    }
    false
}

fn manifest_dir_depth(manifest_path: &str) -> i32 {
    // repo-relative path like `crates/foo/Cargo.toml`
    // Returns number of directory segments (not including the filename)
    // For root-level `Cargo.toml`, returns 0
    // For `crates/foo/Cargo.toml`, returns 2
    let trimmed = manifest_path.trim_matches('/');
    let mut parts: Vec<&str> = trimmed.split('/').collect();
    // Always drop the filename (last segment)
    if !parts.is_empty() {
        parts.pop();
    }
    parts
        .into_iter()
        .filter(|s| !s.is_empty() && *s != ".")
        .count() as i32
}

fn escapes_repo_root(start_depth: i32, rel_path: &str) -> bool {
    let mut depth = start_depth;
    for seg in rel_path.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                depth -= 1;
                if depth < 0 {
                    return true;
                }
            }
            _ => {
                depth += 1;
            }
        }
    }
    false
}
