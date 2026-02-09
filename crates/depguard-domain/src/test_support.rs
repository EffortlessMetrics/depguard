use crate::model::{
    DepKind, DepSpec, DependencyDecl, ManifestModel, PackageMeta, WorkspaceDependency,
    WorkspaceModel,
};
use crate::policy::{CheckPolicy, EffectiveConfig, FailOn, Scope};
use depguard_types::{Location, RepoPath, Severity};
use std::collections::BTreeMap;

pub fn dep_decl(name: &str, kind: DepKind, spec: DepSpec, target: Option<&str>) -> DependencyDecl {
    DependencyDecl {
        kind,
        name: name.to_string(),
        spec,
        location: Some(Location {
            path: RepoPath::new("Cargo.toml"),
            line: Some(1),
            col: None,
        }),
        target: target.map(|t| t.to_string()),
    }
}

pub fn manifest(
    path: &str,
    publish: bool,
    deps: Vec<DependencyDecl>,
    features: BTreeMap<String, Vec<String>>,
) -> ManifestModel {
    ManifestModel {
        path: RepoPath::new(path),
        package: Some(PackageMeta {
            name: "pkg".to_string(),
            publish,
        }),
        dependencies: deps,
        features,
    }
}

pub fn model(
    manifests: Vec<ManifestModel>,
    workspace_dependencies: BTreeMap<String, WorkspaceDependency>,
) -> WorkspaceModel {
    WorkspaceModel {
        repo_root: RepoPath::new("."),
        workspace_dependencies,
        manifests,
    }
}

pub fn workspace_dep(name: &str) -> (String, WorkspaceDependency) {
    (
        name.to_string(),
        WorkspaceDependency {
            name: name.to_string(),
            version: None,
            path: None,
            workspace: true,
        },
    )
}

pub fn config_with_check(check_id: &str, severity: Severity) -> EffectiveConfig {
    let mut checks = BTreeMap::new();
    checks.insert(check_id.to_string(), CheckPolicy::enabled(severity));
    EffectiveConfig {
        profile: "test".to_string(),
        scope: Scope::Repo,
        fail_on: FailOn::Error,
        max_findings: 200,
        checks,
    }
}

pub fn config_with_check_allow(
    check_id: &str,
    severity: Severity,
    allow: Vec<&str>,
    ignore_publish_false: bool,
) -> EffectiveConfig {
    let mut policy = CheckPolicy::enabled(severity);
    policy.allow = allow.into_iter().map(|s| s.to_string()).collect();
    policy.ignore_publish_false = ignore_publish_false;

    let mut checks = BTreeMap::new();
    checks.insert(check_id.to_string(), policy);

    EffectiveConfig {
        profile: "test".to_string(),
        scope: Scope::Repo,
        fail_on: FailOn::Error,
        max_findings: 200,
        checks,
    }
}
