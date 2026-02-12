use depguard_types::{Location, RepoPath};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default)]
pub struct WorkspaceModel {
    pub repo_root: RepoPath,

    /// `[workspace.dependencies]` from the root manifest, if present.
    pub workspace_dependencies: BTreeMap<String, WorkspaceDependency>,

    /// All manifests in scope (root + members).
    pub manifests: Vec<ManifestModel>,
}

#[derive(Clone, Debug, Default)]
pub struct WorkspaceDependency {
    pub name: String,
    pub version: Option<String>,
    pub path: Option<String>,
    pub workspace: bool,
}

#[derive(Clone, Debug, Default)]
pub struct ManifestModel {
    pub path: RepoPath,
    pub package: Option<PackageMeta>,
    pub dependencies: Vec<DependencyDecl>,
    /// Features defined in [features] table, mapped to their dependencies.
    pub features: BTreeMap<String, Vec<String>>,
}

#[derive(Clone, Debug)]
pub struct PackageMeta {
    pub name: String,
    pub publish: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DepKind {
    Normal,
    Dev,
    Build,
}

#[derive(Clone, Debug)]
pub struct DependencyDecl {
    pub kind: DepKind,
    pub name: String,
    pub spec: DepSpec,
    pub location: Option<Location>,
    /// Target platform filter (e.g. `cfg(unix)`, `x86_64-unknown-linux-gnu`).
    /// Present only for deps under `[target.<spec>.*]` tables.
    pub target: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct DepSpec {
    pub version: Option<String>,
    pub path: Option<String>,
    pub workspace: bool,
    /// Git repository URL (e.g., "https://github.com/...")
    pub git: Option<String>,
    /// Git branch reference
    pub branch: Option<String>,
    /// Git tag reference
    pub tag: Option<String>,
    /// Git commit revision
    pub rev: Option<String>,
    /// Whether default-features is explicitly set (None = not specified)
    pub default_features: Option<bool>,
    /// Whether this dependency is marked as optional
    pub optional: bool,
}

impl ManifestModel {
    pub fn is_publishable(&self) -> bool {
        self.package.as_ref().map(|p| p.publish).unwrap_or(false)
    }

    pub fn package_name(&self) -> Option<&str> {
        self.package.as_ref().map(|p| p.name.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn publishable_and_package_name_behavior() {
        let mut manifest = ManifestModel::default();
        assert!(!manifest.is_publishable());
        assert_eq!(manifest.package_name(), None);

        manifest.package = Some(PackageMeta {
            name: "depguard".to_string(),
            publish: true,
        });
        assert!(manifest.is_publishable());
        assert_eq!(manifest.package_name(), Some("depguard"));

        manifest.package = Some(PackageMeta {
            name: "private".to_string(),
            publish: false,
        });
        assert!(!manifest.is_publishable());
        assert_eq!(manifest.package_name(), Some("private"));
    }
}
