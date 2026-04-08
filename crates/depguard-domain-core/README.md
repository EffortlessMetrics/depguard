# depguard-domain-core

Core model and policy primitives for depguard domain logic.

This crate defines the fundamental types shared by the domain layer and its adapters. It provides the data structures that represent a Cargo workspace and the policy configuration for evaluating it.

## Purpose

The domain-core crate provides:
- **Workspace model types**: In-memory representation of Cargo manifests
- **Policy types**: Configuration for check behavior and severity
- **Shared abstractions**: Types used by both domain and infrastructure layers

This crate has minimal dependencies and remains completely pure—no I/O, no side effects.

## Key Features

### Workspace Model

The workspace model represents a Cargo workspace in memory:

```rust
pub struct WorkspaceModel {
    pub root_path: RepoPath,
    pub manifests: Vec<ManifestModel>,
    pub workspace_dependencies: BTreeMap<String, WorkspaceDependency>,
}

pub struct ManifestModel {
    pub path: RepoPath,
    pub package: Option<PackageMeta>,
    pub dependencies: Vec<DependencyDecl>,
    pub features: BTreeMap<String, Vec<String>>,
}

pub struct DependencyDecl {
    pub name: String,
    pub kind: DepKind,
    pub spec: DepSpec,
    pub location: Location,
    pub inline_suppressions: Vec<String>,
}
```

### Policy Types

Policy configuration controls check behavior:

```rust
pub struct PolicyConfig {
    pub checks: BTreeMap<String, CheckPolicy>,
    pub fail_on: FailOn,
    pub scope: Scope,
}

pub struct CheckPolicy {
    pub enabled: bool,
    pub severity: Severity,
    pub allow: Vec<String>,
}

pub enum FailOn {
    Warning,
    Error,
    Never,
}

pub enum Scope {
    Repo,
    Diff,
}
```

### Dependency Specification

Rich representation of dependency specs:

```rust
pub struct DepSpec {
    pub version: Option<String>,
    pub path: Option<String>,
    pub git: Option<String>,
    pub branch: Option<String>,
    pub tag: Option<String>,
    pub rev: Option<String>,
    pub registry: Option<String>,
    pub default_features: Option<bool>,
    pub features: Vec<String>,
    pub optional: Option<bool>,
    pub workspace: Option<bool>,
}

pub enum DepKind {
    Normal,
    Dev,
    Build,
}
```

## Public API

```rust
// Model types
pub mod model {
    pub struct WorkspaceModel { /* ... */ }
    pub struct ManifestModel { /* ... */ }
    pub struct DependencyDecl { /* ... */ }
    pub struct DepSpec { /* ... */ }
    pub enum DepKind { /* ... */ }
    pub struct PackageMeta { /* ... */ }
    pub struct WorkspaceDependency { /* ... */ }
}

// Policy types
pub mod policy {
    pub struct PolicyConfig { /* ... */ }
    pub struct CheckPolicy { /* ... */ }
    pub enum FailOn { /* ... */ }
    pub enum Scope { /* ... */ }
}
```

## Usage Example

```rust
use depguard_domain_core::model::{WorkspaceModel, ManifestModel, DependencyDecl, DepKind, DepSpec};
use depguard_domain_core::policy::{PolicyConfig, CheckPolicy, FailOn, Scope};
use depguard_types::Severity;

// Build a policy configuration
let mut policy = PolicyConfig::default();
policy.checks.insert(
    "deps.no_wildcards".to_string(),
    CheckPolicy {
        enabled: true,
        severity: Severity::Error,
        allow: vec![],
    },
);

// Build a workspace model (typically done by depguard-repo)
let model = WorkspaceModel {
    root_path: RepoPath::new("Cargo.toml"),
    manifests: vec![/* ... */],
    workspace_dependencies: BTreeMap::new(),
};
```

## Dependencies

| Crate | Purpose |
|-------|---------|
| `depguard-types` | Shared types, IDs, severity |
| `depguard-yanked` | Yanked version index type |
| `serde` | Serialization support |

## Design Constraints

- **No I/O**: Pure data structures only
- **Minimal dependencies**: Only essential crates
- **Serializable**: All types support `serde`
- **Stable API**: Types are used across layers

## Related Crates

- [`depguard-domain`](../depguard-domain/) - Main domain entry point
- [`depguard-domain-checks`](../depguard-domain-checks/) - Check implementations
- [`depguard-repo`](../depguard-repo/) - Model construction from filesystem
- [`depguard-types`](../depguard-types/) - Shared types
