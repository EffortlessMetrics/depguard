//! Fuzz target for dependency specification parsing.
//!
//! Goal: The dependency spec parser should **never panic** on any input.
//! It may return errors or default specs, but panics are unacceptable.
//!
//! Run with:
//! ```bash
//! cargo +nightly fuzz run fuzz_dependency_spec
//! ```

#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;

/// Structured input for dependency specification fuzzing.
#[derive(Arbitrary, Debug)]
struct DependencySpecInput {
    /// Dependency name
    name: String,
    /// Version requirement string (e.g., "1.0.0", "^0.1", ">=1, <2")
    version: Option<String>,
    /// Path dependency (relative path)
    path: Option<String>,
    /// Git URL
    git: Option<String>,
    /// Git branch
    branch: Option<String>,
    /// Git tag
    tag: Option<String>,
    /// Git rev (commit hash)
    rev: Option<String>,
    /// Workspace inheritance
    workspace: bool,
    /// Default features flag
    default_features: Option<bool>,
    /// Optional flag
    optional: bool,
    /// Package rename
    package: Option<String>,
}

/// Input type for generating manifests with various dependency formats.
#[derive(Arbitrary, Debug)]
#[allow(dead_code)]
enum DepFormat {
    /// Simple string version: `dep = "1.0.0"`
    String(String),
    /// Inline table: `dep = { version = "1.0.0", ... }`
    InlineTable(DependencySpecInput),
    /// Full table: `[dependencies.dep]\nversion = "1.0.0"`
    FullTable(DependencySpecInput),
}

fuzz_target!(|input: DependencySpecInput| {
    // Limit string lengths to avoid OOM
    if input.name.len() > 256
        || input.version.as_ref().map_or(false, |v| v.len() > 512)
        || input.path.as_ref().map_or(false, |p| p.len() > 1024)
        || input.git.as_ref().map_or(false, |g| g.len() > 2048)
        || input.branch.as_ref().map_or(false, |b| b.len() > 256)
        || input.tag.as_ref().map_or(false, |t| t.len() > 256)
        || input.rev.as_ref().map_or(false, |r| r.len() > 128)
        || input.package.as_ref().map_or(false, |p| p.len() > 256)
    {
        return;
    }

    // Build a manifest with this dependency spec
    let manifest = build_manifest_from_spec(&input);

    // Parse as member manifest - should never panic
    let _ = depguard_repo::fuzz::parse_member_manifest(&manifest);

    // Also test as root manifest
    let _ = depguard_repo::fuzz::parse_root_manifest(&manifest);
});

/// Build a Cargo.toml manifest from a dependency specification.
fn build_manifest_from_spec(spec: &DependencySpecInput) -> String {
    let name = if spec.name.is_empty() {
        "test-dep".to_string()
    } else {
        spec.name.clone()
    };

    // Build the dependency value
    let dep_value = if let Some(version) = &spec.version {
        if spec.path.is_none()
            && spec.git.is_none()
            && !spec.workspace
            && spec.default_features.is_none()
            && !spec.optional
            && spec.package.is_none()
        {
            // Simple string form
            format!("\"{}\"", version.escape_default())
        } else {
            // Inline table form
            let mut parts = Vec::new();
            parts.push(format!("version = \"{}\"", version.escape_default()));

            if let Some(path) = &spec.path {
                parts.push(format!("path = \"{}\"", path.escape_default()));
            }
            if let Some(git) = &spec.git {
                parts.push(format!("git = \"{}\"", git.escape_default()));
            }
            if let Some(branch) = &spec.branch {
                parts.push(format!("branch = \"{}\"", branch.escape_default()));
            }
            if let Some(tag) = &spec.tag {
                parts.push(format!("tag = \"{}\"", tag.escape_default()));
            }
            if let Some(rev) = &spec.rev {
                parts.push(format!("rev = \"{}\"", rev.escape_default()));
            }
            if spec.workspace {
                parts.push("workspace = true".to_string());
            }
            if let Some(df) = spec.default_features {
                parts.push(format!("default-features = {}", df));
            }
            if spec.optional {
                parts.push("optional = true".to_string());
            }
            if let Some(pkg) = &spec.package {
                parts.push(format!("package = \"{}\"", pkg.escape_default()));
            }

            format!("{{ {} }}", parts.join(", "))
        }
    } else if spec.workspace {
        // Workspace inheritance
        let mut parts = vec!["workspace = true".to_string()];
        if spec.optional {
            parts.push("optional = true".to_string());
        }
        if let Some(df) = spec.default_features {
            parts.push(format!("default-features = {}", df));
        }
        format!("{{ {} }}", parts.join(", "))
    } else if let Some(path) = &spec.path {
        // Path-only dependency
        let mut parts = vec![format!("path = \"{}\"", path.escape_default())];
        if spec.optional {
            parts.push("optional = true".to_string());
        }
        format!("{{ {} }}", parts.join(", "))
    } else if let Some(git) = &spec.git {
        // Git dependency
        let mut parts = vec![format!("git = \"{}\"", git.escape_default())];
        if let Some(branch) = &spec.branch {
            parts.push(format!("branch = \"{}\"", branch.escape_default()));
        }
        if let Some(tag) = &spec.tag {
            parts.push(format!("tag = \"{}\"", tag.escape_default()));
        }
        if let Some(rev) = &spec.rev {
            parts.push(format!("rev = \"{}\"", rev.escape_default()));
        }
        format!("{{ {} }}", parts.join(", "))
    } else {
        // Default to empty inline table
        "{}".to_string()
    };

    format!(
        r#"[package]
name = "test-package"
version = "0.1.0"

[dependencies]
{} = {}
"#,
        name.escape_default(),
        dep_value
    )
}

/// Additional structured input for workspace inheritance edge cases.
#[derive(Arbitrary, Debug)]
#[allow(dead_code)]
struct WorkspaceInheritanceInput {
    /// Dependency name
    name: String,
    /// workspace = true
    workspace: bool,
    /// Optional features
    features: Vec<String>,
    /// default-features setting
    default_features: Option<bool>,
    /// optional setting
    optional: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_workspace_manifest(deps: &[WorkspaceInheritanceInput]) -> String {
        let dep_lines: Vec<String> = deps
            .iter()
            .filter(|d| !d.name.is_empty() && d.name.len() <= 256)
            .take(50)
            .map(|d| {
                let mut parts = vec!["workspace = true".to_string()];
                if !d.features.is_empty() {
                    let feats: String = d
                        .features
                        .iter()
                        .filter(|f| !f.is_empty() && f.len() <= 128)
                        .take(20)
                        .map(|f| format!("\"{}\"", f.escape_default()))
                        .collect::<Vec<_>>()
                        .join(", ");
                    parts.push(format!("features = [{}]", feats));
                }
                if let Some(df) = d.default_features {
                    parts.push(format!("default-features = {}", df));
                }
                if d.optional {
                    parts.push("optional = true".to_string());
                }
                format!("{} = {{ {} }}", d.name.escape_default(), parts.join(", "))
            })
            .collect();

        format!(
            r#"[package]
name = "workspace-member"
version = "0.1.0"

[dependencies]
{}
"#,
            dep_lines.join("\n")
        )
    }
}
