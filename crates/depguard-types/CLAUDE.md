# CLAUDE.md — depguard-types

## Purpose

Core data types, stable IDs, and explanation registry. This crate defines the **schema contract** between depguard and its consumers.

## Key Modules

| Module | Contents |
|--------|----------|
| [`receipt.rs`] | `DepguardReport`, `Finding`, `Severity`, `Verdict`, `Location`, `DepguardData` |
| [`ids.rs`] | Stable check IDs and error codes as constants |
| [`explain.rs`] | `Explanation` struct and `lookup_explanation()` registry |
| [`path.rs`] | `RepoPath` for canonical repo-relative paths |
| [`baseline.rs`] | `BaselineV1` for suppression baselines |
| [`buildfix.rs`] | `BuildfixPlanV1` for auto-fix plans |

## Stability Constraints

- **Never rename** check IDs or codes; deprecate via aliases only
- All types derive `schemars::JsonSchema` for JSON schema generation
- `(check_id, code)` pairs must have an entry in the explain registry

## Check IDs and Codes

```rust
// Check IDs
pub const CHECK_DEPS_NO_WILDCARDS: &str = "deps.no_wildcards";
pub const CHECK_DEPS_PATH_REQUIRES_VERSION: &str = "deps.path_requires_version";
pub const CHECK_DEPS_PATH_SAFETY: &str = "deps.path_safety";
pub const CHECK_DEPS_WORKSPACE_INHERITANCE: &str = "deps.workspace_inheritance";
pub const CHECK_DEPS_GIT_REQUIRES_VERSION: &str = "deps.git_requires_version";
pub const CHECK_DEPS_DEV_ONLY_IN_NORMAL: &str = "deps.dev_only_in_normal";
pub const CHECK_DEPS_DEFAULT_FEATURES_EXPLICIT: &str = "deps.default_features_explicit";
pub const CHECK_DEPS_NO_MULTIPLE_VERSIONS: &str = "deps.no_multiple_versions";
pub const CHECK_DEPS_OPTIONAL_UNUSED: &str = "deps.optional_unused";
pub const CHECK_DEPS_YANKED_VERSIONS: &str = "deps.yanked_versions";

// Codes
pub const CODE_WILDCARD_VERSION: &str = "wildcard_version";
pub const CODE_PATH_WITHOUT_VERSION: &str = "path_without_version";
pub const CODE_ABSOLUTE_PATH: &str = "absolute_path";
pub const CODE_PARENT_ESCAPE: &str = "parent_escape";
pub const CODE_MISSING_WORKSPACE_TRUE: &str = "missing_workspace_true";
pub const CODE_GIT_WITHOUT_VERSION: &str = "git_without_version";
pub const CODE_DEV_IN_NORMAL: &str = "dev_in_normal";
pub const CODE_DEFAULT_FEATURES_IMPLICIT: &str = "default_features_implicit";
pub const CODE_MULTIPLE_VERSIONS: &str = "multiple_versions";
pub const CODE_OPTIONAL_UNUSED: &str = "optional_unused";
pub const CODE_YANKED_VERSION: &str = "yanked_version";
```

## Core Types

```rust
pub struct DepguardReport {
    pub schema: String,
    pub tool: ToolInfo,
    pub run: RunInfo,
    pub verdict: Verdict,
    pub findings: Vec<Finding>,
}

pub struct Finding {
    pub check_id: String,
    pub code: String,
    pub severity: Severity,
    pub message: String,
    pub location: Location,
    pub data: Option<serde_json::Value>,
}

pub enum Severity { Info, Warning, Error }
pub enum Verdict { Pass, Warn, Fail }
```

## Explanation Registry

Every `(check_id, code)` pair must have an explanation:

```rust
pub fn lookup_explanation(identifier: &str) -> Option<Explanation>;

pub struct Explanation {
    pub check_id: &'static str,
    pub code: Option<&'static str>,
    pub summary: &'static str,
    pub remediation: &'static str,
}
```

## Dependencies

| Dependency | Purpose |
|------------|---------|
| `camino` | UTF-8 path handling |
| `serde` | Serialization |
| `serde_json` | JSON handling |
| `schemars` | JSON schema generation |
| `time` | Timestamps |

## Testing

```bash
cargo test -p depguard-types
```

Tests validate:
- All check IDs and codes have explanations in the registry
- Serialization roundtrips
- Schema generation

## Adding New Checks

When adding a new check:
1. Add check ID constant to `ids.rs`
2. Add code constants for each finding type
3. Add explanations to `explain.rs` registry
4. Ensure test passes: all IDs/codes must have explanations
