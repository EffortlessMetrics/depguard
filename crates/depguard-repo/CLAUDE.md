# CLAUDE.md — depguard-repo

## Purpose

Repository adapters—filesystem I/O, workspace discovery, and TOML parsing with line number tracking.

## Key Modules

| Module | Contents |
|--------|----------|
| `discover.rs` | `discover_manifests()` — walks workspace, handles globs |
| `parse.rs` | `parse_root_manifest()`, `parse_member_manifest()` — TOML with locations |
| `fuzz.rs` | Fuzz-friendly APIs that never panic |

## Public API

```rust
// Discover all Cargo.toml files in workspace
pub fn discover_manifests(repo_root: &Utf8Path) -> Result<Vec<RepoPath>>

// Parse root manifest (extracts [workspace.dependencies])
pub fn parse_root_manifest(path: &RepoPath, text: &str) -> Result<(HashMap<String, DepSpec>, ManifestModel)>

// Parse member manifest
pub fn parse_member_manifest(path: &RepoPath, text: &str) -> Result<ManifestModel>

// Build complete workspace model
pub fn build_workspace_model(repo_root: &Utf8Path, scope: ScopeInput) -> Result<WorkspaceModel>
```

## Discovery Logic

1. Read root `Cargo.toml`
2. Parse `[workspace]` section for `members` globs and `exclude` patterns
3. Expand globs, filter exclusions
4. Return deterministic list of manifest paths

## Parsing Features

- Extracts all dependency sections: `[dependencies]`, `[dev-dependencies]`, `[build-dependencies]`
- Handles `[target.'cfg(...)'.dependencies]` sections
- Tracks byte offsets → line numbers for error reporting
- Handles inline tables and expanded table syntax

## Fuzz Module

The `fuzz` module exposes APIs that return `Option` instead of `Result` and never panic:

```rust
pub mod fuzz {
    pub fn parse_root_manifest(text: &str) -> Option<(HashMap<String, DepSpec>, ManifestModel)>
    pub fn parse_member_manifest(text: &str) -> Option<ManifestModel>
    pub fn expand_globs(patterns: &[&str]) -> Option<Vec<String>>
}
```

## Dependencies

- `depguard-types` — `RepoPath`, DTOs
- `depguard-domain` — `WorkspaceModel`, `ManifestModel`
- `toml_edit` — TOML parsing with span preservation
- `globset` — Workspace member glob expansion
- `walkdir` — Directory traversal
- `camino` — UTF-8 paths

## Testing

```bash
cargo test -p depguard-repo              # Unit tests
cargo +nightly fuzz run fuzz_toml_parser # Fuzzing
```

The TOML parser must never panic on any input—this is validated via fuzzing.
