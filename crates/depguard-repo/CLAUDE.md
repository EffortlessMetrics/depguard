# CLAUDE.md — depguard-repo

## Purpose

Repository adapters for filesystem I/O, workspace discovery, and model assembly.
Pure TOML parsing is delegated to `depguard-repo-parser`.

## Key Modules

| Module | Contents |
|--------|----------|
| `discover.rs` | `discover_manifests()` — walks workspace, handles globs |
| `cache.rs` | Manifest cache IO and invalidation |
| `fuzz.rs` | Fuzz-friendly APIs that never panic |

## Public API

```rust
// Discover all Cargo.toml files in workspace
pub fn discover_manifests(repo_root: &Utf8Path) -> Result<Vec<RepoPath>>

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
- Tracks byte offsets → line numbers for error reporting (via parser crate)
- Handles inline tables and expanded table syntax

## Fuzz Module

The `fuzz` module exposes parsing APIs that should not panic on malformed input:

```rust
pub mod fuzz {
    pub fn parse_root_manifest(text: &str) -> anyhow::Result<()>
    pub fn parse_member_manifest(text: &str) -> anyhow::Result<()>
    pub fn expand_globs(patterns: &[String], candidates: &[String]) -> anyhow::Result<Vec<String>>
}
```

## Dependencies

- `depguard-types` — `RepoPath`, DTOs
- `depguard-domain` — `WorkspaceModel`, `ManifestModel`
- `depguard-repo-parser` — pure manifest parsing (`parse_root_manifest`, `parse_member_manifest`)
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
