# depguard-repo

Repository adapters for workspace discovery and Cargo manifest parsing.

This crate is the filesystem boundary between depguard application logic and on-disk workspace data. It discovers workspace manifests, reads files, and builds domain models for policy evaluation.

## Purpose

The repo crate provides:
- Workspace manifest discovery via Cargo-compatible glob expansion
- Domain model construction from real filesystem data
- Diff scope filtering for CI/CD integration
- Fuzz-facing parser entry points that never panic

## Key Features

### Workspace Discovery

Discovers all `Cargo.toml` files in a workspace following Cargo's semantics:

```rust
use depguard_repo::discover_manifests;

let manifests = discover_manifests(&repo_root)?;
for manifest in &manifests {
    println!("Found: {}", manifest);
}
```

### Model Building

Builds a `WorkspaceModel` ready for domain evaluation:

```rust
use depguard_repo::{build_workspace_model, ScopeInput};

let model = build_workspace_model(
    &repo_root,
    ScopeInput::Repo,  // or ScopeInput::Diff(changed_files)
)?;
```

### Diff Scope Support

Filter analysis to only changed files:

```rust
let changed_files = vec!["crates/my-crate/Cargo.toml".into()];
let model = build_workspace_model(
    &repo_root,
    ScopeInput::Diff(changed_files),
)?;
```

### Fuzz-Facing API

Panic-free parsing for fuzzing and testing:

```rust
use depguard_repo::fuzz;

// Parse arbitrary text without panic risk
fuzz::parse_root_manifest(arbitrary_toml)?;
fuzz::parse_member_manifest(arbitrary_toml)?;

// Expand glob patterns without filesystem
let matched = fuzz::expand_globs(&["crates/*"], &candidates)?;
```

## Public API

```rust
/// Discover all Cargo.toml manifests in a workspace
pub fn discover_manifests(repo_root: &Utf8Path) -> anyhow::Result<Vec<RepoPath>>;

/// Build a workspace model from filesystem data
pub fn build_workspace_model(
    repo_root: &Utf8Path,
    scope: ScopeInput,
) -> anyhow::Result<WorkspaceModel>;

/// Scope input for filtering
pub enum ScopeInput {
    Repo,
    Diff(Vec<RepoPath>),
}

/// Fuzz-facing API (never panics)
pub mod fuzz {
    pub fn parse_root_manifest(text: &str) -> anyhow::Result<()>;
    pub fn parse_member_manifest(text: &str) -> anyhow::Result<()>;
    pub fn expand_globs(patterns: &[String], candidates: &[String]) -> anyhow::Result<Vec<String>>;
}
```

## Cargo-Compatible Glob Semantics

The workspace member glob expansion follows Cargo's actual behavior:

### Supported Features

1. **Double-star (`**`) patterns**: Matches zero or more directory components
   ```toml
   [workspace]
   members = ["crates/**"]
   ```

2. **Exclusion patterns**: Patterns starting with `!` exclude previously matched paths
   ```toml
   [workspace]
   members = ["crates/*", "!crates/excluded"]
   ```

3. **Empty member lists**: When `[workspace]` is present with no members, all `Cargo.toml` files are included

4. **Non-existent paths**: Patterns that match no files are silently ignored

5. **Relative path normalization**: Both `./path` and `path` forms are handled equivalently

6. **Combined exclude mechanisms**: Both `exclude` field and `!` prefix patterns work together

### Known Deviations from Cargo

1. **Circular workspace references**: Not currently detected
2. **default-members**: The `default-members` field is not honored during discovery
3. **Path dependencies with workspace inheritance**: Complex inheritance scenarios may behave differently

## Design Constraints

- **Filesystem I/O is allowed**: This is the I/O boundary crate
- **External subprocess calls are not owned**: Git integration happens at CLI layer
- **Deterministic for identical repository state**: Same files → same model
- **Parsing delegated to `depguard-repo-parser`**: This crate handles I/O, not parsing

## Dependencies

| Crate | Purpose |
|-------|---------|
| `depguard-types` | RepoPath and shared types |
| `depguard-domain-core` | WorkspaceModel and domain types |
| `depguard-repo-parser` | TOML manifest parsing |
| `anyhow` | Error handling |
| `camino` | UTF-8 paths |
| `globset` | Glob pattern matching |
| `walkdir` | Directory traversal |
| `rayon` | Parallel processing |
| `toml_edit` | TOML manipulation |

## Related Crates

- [`depguard-repo-parser`](../depguard-repo-parser/) - Pure TOML parsing
- [`depguard-domain-core`](../depguard-domain-core/) - Domain model types
- [`depguard-app`](../depguard-app/) - Use case orchestration

## Reference

- [Cargo Workspace Documentation](https://doc.rust-lang.org/cargo/reference/manifest.html#the-workspace-section)
