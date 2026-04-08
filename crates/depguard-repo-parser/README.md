# depguard-repo-parser

Pure Cargo manifest parsing for depguard without filesystem access.

This crate provides deterministic, IO-free TOML parsing for `Cargo.toml` manifests. It extracts dependency declarations, package metadata, workspace configuration, and inline suppressions into domain model types.

## Purpose

The repo-parser crate:
- Parses TOML source text into domain model types
- Extracts dependency declarations with full metadata
- Captures inline suppression comments
- Remains completely pure with no filesystem or network access

## Key Features

### Root Manifest Parsing

Parse workspace root manifests with workspace dependency extraction:

```rust
use depguard_repo_parser::parse_root_manifest;

let (workspace_deps, manifest_model) = parse_root_manifest(
    &manifest_path,
    &toml_source,
)?;
```

### Member Manifest Parsing

Parse workspace member or single-package manifests:

```rust
use depguard_repo_parser::parse_member_manifest;

let manifest_model = parse_member_manifest(
    &manifest_path,
    &toml_source,
)?;
```

### Comprehensive Dependency Extraction

Extracts all dependency kinds:
- `[dependencies]` - Normal dependencies
- `[dev-dependencies]` - Development dependencies
- `[build-dependencies]` - Build dependencies
- `[target.'cfg(...)'.dependencies]` - Target-specific dependencies

### Inline Suppression Support

Automatically captures inline suppression comments:

```toml
# depguard: allow(no_wildcards)
serde = "*"
```

## Public API

```rust
/// Parse a root/workspace Cargo.toml
pub fn parse_root_manifest(
    manifest_path: &RepoPath,
    text: &str,
) -> anyhow::Result<(BTreeMap<String, WorkspaceDependency>, ManifestModel)>;

/// Parse a member/single-package Cargo.toml
pub fn parse_member_manifest(
    manifest_path: &RepoPath,
    text: &str,
) -> anyhow::Result<ManifestModel>;
```

## Usage Example

```rust
use depguard_repo_parser::{parse_root_manifest, parse_member_manifest};
use depguard_types::RepoPath;

// Parse a root manifest
let root_path = RepoPath::new("Cargo.toml");
let root_toml = std::fs::read_to_string("Cargo.toml")?;
let (workspace_deps, root_model) = parse_root_manifest(&root_path, &root_toml)?;

println!("Package: {:?}", root_model.package);
println!("Dependencies: {}", root_model.dependencies.len());
println!("Workspace deps: {}", workspace_deps.len());

// Parse a member manifest
let member_path = RepoPath::new("crates/my-crate/Cargo.toml");
let member_toml = std::fs::read_to_string("crates/my-crate/Cargo.toml")?;
let member_model = parse_member_manifest(&member_path, &member_toml)?;

for dep in &member_model.dependencies {
    println!("{}: {:?} at {}:{}", dep.name, dep.kind, dep.location.path, dep.location.line);
}
```

## Manifest Model Structure

```rust
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
```

## Design Constraints

- **IO-free**: All inputs are provided as string values
- **Deterministic**: Same input always produces same output
- **Panic-free**: Handles malformed TOML gracefully with errors
- **No filesystem access**: Caller provides file contents

## What This Crate Does NOT Own

- Filesystem traversal
- Repository discovery
- Cache invalidation
- Git integration

These responsibilities belong to [`depguard-repo`](../depguard-repo/).

## Dependencies

| Crate | Purpose |
|-------|---------|
| `depguard-domain-core` | Model types for output |
| `depguard-inline-suppressions` | Comment parsing |
| `depguard-types` | RepoPath and Location types |
| `anyhow` | Error handling |
| `toml_edit` | TOML parsing |

## Related Crates

- [`depguard-repo`](../depguard-repo/) - Filesystem I/O and discovery
- [`depguard-domain-core`](../depguard-domain-core/) - Output model types
- [`depguard-inline-suppressions`](../depguard-inline-suppressions/) - Suppression parsing
