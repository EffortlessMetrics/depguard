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
}

#[derive(Clone, Debug, Default)]
pub struct DepSpec {
    pub version: Option<String>,
    pub path: Option<String>,
    pub workspace: bool,
}

impl ManifestModel {
    pub fn is_publishable(&self) -> bool {
        self.package.as_ref().map(|p| p.publish).unwrap_or(false)
    }

    pub fn package_name(&self) -> Option<&str> {
        self.package.as_ref().map(|p| p.name.as_str())
    }
}
