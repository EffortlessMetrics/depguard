# CLAUDE.md — depguard-domain-core

## Purpose

Core model and policy primitives shared by domain and adapters. This crate defines the fundamental data structures for workspace representation and policy configuration.

## Critical Constraint

**This crate must remain pure.** No filesystem access, no stdout/stderr, no network. All inputs come via function parameters; all outputs via return values.

## Key Modules

| Module | Contents |
|--------|----------|
| [`model.rs`] | `WorkspaceModel`, `ManifestModel`, `DependencyDecl`, `DepSpec`, `DepKind`, `PackageMeta`, `WorkspaceDependency` |
| [`policy.rs`] | `EffectiveConfig`, `CheckPolicy`, `Scope`, `FailOn` |

## Core Types

### Model Types

```rust
// Top-level workspace representation
pub struct WorkspaceModel {
    pub repo_root: RepoPath,
    pub workspace_dependencies: BTreeMap<String, WorkspaceDependency>,
    pub manifests: Vec<ManifestModel>,
}

// Single Cargo.toml representation
pub struct ManifestModel {
    pub path: RepoPath,
    pub package: Option<PackageMeta>,
    pub dependencies: Vec<DependencyDecl>,
    pub features: BTreeMap<String, Vec<String>>,
}

// Dependency declaration with location
pub struct DependencyDecl {
    pub kind: DepKind,
    pub name: String,
    pub spec: DepSpec,
    pub location: Option<Location>,
    pub target: Option<String>,
    pub inline_suppressions: Vec<String>,
}

// Dependency specification details
pub struct DepSpec {
    pub version: Option<String>,
    pub path: Option<String>,
    pub workspace: bool,
    pub git: Option<String>,
    pub branch: Option<String>,
    pub tag: Option<String>,
    pub rev: Option<String>,
    pub default_features: Option<bool>,
    pub optional: bool,
    pub package: Option<String>,
}

pub enum DepKind { Normal, Dev, Build }
```

### Policy Types

```rust
pub struct EffectiveConfig {
    pub profile: String,
    pub scope: Scope,
    pub fail_on: FailOn,
    pub max_findings: usize,
    pub yanked_index: Option<YankedIndex>,
    pub checks: BTreeMap<String, CheckPolicy>,
}

pub struct CheckPolicy {
    pub enabled: bool,
    pub severity: Severity,
    pub allow: Vec<String>,
    pub ignore_publish_false: bool,
}

pub enum Scope { Repo, Diff }
pub enum FailOn { Error, Warning }
```

## Design Constraints

- **No I/O**: Pure data structures
- **Serializable**: All types derive `Serialize`/`Deserialize`
- **Stable**: Model types form the contract between layers
- **No feature gates**: Core types are always available

## Dependencies

- `depguard-types` — `RepoPath`, `Location`, `Severity`
- `depguard-yanked` — `YankedIndex` for yanked version checking
- `serde` — Serialization

## Testing

```bash
cargo test -p depguard-domain-core
```

Tests cover model construction, policy helpers, and serialization roundtrips.
