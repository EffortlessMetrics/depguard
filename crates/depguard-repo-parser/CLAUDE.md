# CLAUDE.md — depguard-repo-parser

## Purpose

Pure Cargo manifest parser for depguard domain models. This crate is IO-free and deterministic: all inputs are TOML source strings and file paths supplied as values.

## Key Functions

```rust
/// Parse a root workspace manifest (with [workspace] section).
/// Returns workspace dependencies and the manifest model.
pub fn parse_root_manifest(
    manifest_path: &RepoPath,
    text: &str,
) -> anyhow::Result<(BTreeMap<String, WorkspaceDependency>, ManifestModel)>;

/// Parse a member manifest (package manifest without [workspace]).
pub fn parse_member_manifest(
    manifest_path: &RepoPath,
    text: &str,
) -> anyhow::Result<ManifestModel>;
```

## Parsing Features

- Extracts all dependency sections:
  - `[dependencies]`
  - `[dev-dependencies]`
  - `[build-dependencies]`
- Handles target-specific dependencies:
  - `[target.'cfg(...)'.dependencies]`
  - `[target.<triple>.dependencies]`
- Tracks byte offsets → line numbers for error reporting
- Handles both inline tables and expanded table syntax:
  ```toml
  # Inline table
  serde = { version = "1.0", features = ["derive"] }
  
  # Expanded table
  [dependencies.serde]
  version = "1.0"
  features = ["derive"]
  ```
- Parses inline suppressions from comments
- Extracts `[features]` table
- Parses `[workspace.dependencies]` from root manifests

## Design Constraints

- **No I/O**: Takes string input, returns models
- **No panics**: Malformed TOML returns errors
- **Deterministic**: Same input → same output
- **Span preservation**: Uses `toml_edit` to track line numbers

## Internal Functions

| Function | Purpose |
|----------|---------|
| `byte_offset_to_line()` | Convert byte offset to 1-based line number |
| `parse_package()` | Extract `[package]` metadata |
| `parse_dep_table()` | Parse a dependency table (normal/dev/build) |
| `parse_target_dependencies()` | Parse `[target.*]` sections |
| `parse_features()` | Extract `[features]` table |
| `parse_workspace_dependencies()` | Extract `[workspace.dependencies]` |

## Dependencies

- `depguard-domain-core` — Model types (`ManifestModel`, `DependencyDecl`, etc.)
- `depguard-inline-suppressions` — Suppression parsing
- `depguard-types` — `RepoPath`, `Location`
- `toml_edit` — TOML parsing with span preservation
- `anyhow` — Error handling

Dev dependencies:
- `proptest` — Property-based testing

## Testing

```bash
cargo test -p depguard-repo-parser
```

Tests cover:
- All dependency syntaxes
- Target-specific dependencies
- Malformed TOML handling
- Line number accuracy
- Inline suppression extraction

## Fuzzing

This crate is fuzzed via `fuzz/fuzz_targets/fuzz_manifest_parser.rs` to ensure no panics on arbitrary TOML input.
