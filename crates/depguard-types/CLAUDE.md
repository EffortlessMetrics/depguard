# CLAUDE.md — depguard-types

## Purpose

Core data types, stable IDs, and explanation registry. This crate defines the **schema contract** between depguard and its consumers.

## Key Modules

| Module | Contents |
|--------|----------|
| `receipt.rs` | `DepguardReport`, `Finding`, `Severity`, `Verdict`, `Location`, `DepguardData` |
| `ids.rs` | Stable check IDs and error codes as constants |
| `explain.rs` | `Explanation` struct and `lookup_explanation()` registry |
| `path.rs` | `RepoPath` for canonical repo-relative paths |

## Stability Constraints

- **Never rename** check IDs or codes; deprecate via aliases only
- All types derive `schemars::JsonSchema` for JSON schema generation
- `(check_id, code)` pairs must have an entry in the explain registry

## Check IDs and Codes

```rust
// Check IDs
pub const DEPS_NO_WILDCARDS: &str = "deps.no_wildcards";
pub const DEPS_PATH_REQUIRES_VERSION: &str = "deps.path_requires_version";
pub const DEPS_PATH_SAFETY: &str = "deps.path_safety";
pub const DEPS_WORKSPACE_INHERITANCE: &str = "deps.workspace_inheritance";

// Codes
pub const WILDCARD_VERSION: &str = "wildcard_version";
pub const PATH_WITHOUT_VERSION: &str = "path_without_version";
pub const ABSOLUTE_PATH: &str = "absolute_path";
pub const PARENT_ESCAPE: &str = "parent_escape";
pub const MISSING_WORKSPACE_TRUE: &str = "missing_workspace_true";
```

## Dependencies

- `camino` — UTF-8 path handling
- `serde`, `serde_json` — Serialization
- `schemars` — JSON schema generation
- `time` — Timestamps

## Testing

```bash
cargo test -p depguard-types
```

Tests validate that all check IDs and codes have explanations in the registry.
