# CLAUDE.md — depguard-repo

## Purpose

Repository adapters for filesystem I/O, workspace discovery, and model assembly. Pure TOML parsing is delegated to `depguard-repo-parser`.

## Key Modules

| Module | Contents |
|--------|----------|
| [`discover.rs`] | `discover_manifests()` — walks workspace, handles globs |
| [`cache.rs`] | Manifest cache IO and invalidation |
| [`lib.rs`] | `build_workspace_model()` — orchestrates discovery and parsing |

## Public API

```rust
// Discover all Cargo.toml files in workspace
pub fn discover_manifests(repo_root: &Utf8Path) -> Result<Vec<RepoPath>>

// Build complete workspace model
pub fn build_workspace_model(
    repo_root: &Utf8Path,
    scope: ScopeInput,
) -> Result<WorkspaceModel>

// Scope input for diff mode
pub enum ScopeInput {
    Repo,
    Diff { base: String, head: String },
    DiffFiles(Vec<RepoPath>),
}
```

## Discovery Logic

1. Read root `Cargo.toml`
2. Parse `[workspace]` section for `members` globs and `exclude` patterns
3. Expand globs, filter exclusions
4. Return deterministic list of manifest paths

## Model Building Flow

```
repo_root
    → discover_manifests()
    → read each Cargo.toml
    → depguard-repo-parser::parse_root_manifest() / parse_member_manifest()
    → assemble WorkspaceModel
```

## Parsing Features (via depguard-repo-parser)

- Extracts all dependency sections: `[dependencies]`, `[dev-dependencies]`, `[build-dependencies]`
- Handles `[target.'cfg(...)'.dependencies]` sections
- Tracks byte offsets → line numbers for error reporting
- Handles inline tables and expanded table syntax
- Parses inline suppressions from comments
- Extracts `[workspace.dependencies]` from root manifests

## Design Constraints

- **I/O boundary**: This crate owns filesystem access for manifest reading
- **Deterministic**: Same files → same model
- **No panics**: Malformed manifests return errors
- **Parallel**: Uses rayon for parallel manifest parsing

## Dependencies

| Dependency | Purpose |
|------------|---------|
| `depguard-types` | `RepoPath`, DTOs |
| `depguard-domain-core` | `WorkspaceModel`, `ManifestModel` |
| `depguard-repo-parser` | Pure manifest parsing |
| `anyhow` | Error handling |
| `camino` | UTF-8 paths |
| `globset` | Workspace member glob expansion |
| `walkdir` | Directory traversal |
| `rayon` | Parallel processing |
| `serde`, `serde_json` | Serialization |
| `toml_edit` | TOML manipulation |

Dev dependencies:
- `proptest` — Property-based testing
- `tempfile` — Temporary directories for tests

## Testing

```bash
cargo test -p depguard-repo              # Unit tests
cargo +nightly fuzz run fuzz_workspace_discovery  # Fuzzing
```

## Fuzzing

The workspace discovery and glob expansion are fuzzed via:
- `fuzz/fuzz_targets/fuzz_workspace_discovery.rs`
- `fuzz/fuzz_targets/fuzz_glob_expansion.rs`

These ensure no panics on malformed input.

## Architecture Notes

This crate is the I/O boundary for manifest access. The parsing logic lives in `depguard-repo-parser` to allow:
- Pure parsing without filesystem concerns
- Fuzzing of parsing logic independently
- Reuse of parsing in other contexts (e.g., buildfix editing)
