# depguard-domain

Pure dependency policy evaluation engine for depguard.

This is the business-logic core of depguard. It evaluates an in-memory workspace model against policy configuration and returns findings, verdict, and summary data. The crate is completely pure—no I/O, no side effects.

## Purpose

The domain crate serves as the heart of depguard's hexagonal architecture:
- Implements all dependency hygiene checks
- Produces deterministic, ordered findings
- Calculates verdicts based on severity thresholds
- Remains completely isolated from infrastructure concerns

## Critical Constraint

**No filesystem, network, subprocess, stdout/stderr, or CLI dependencies.**

This crate must remain pure to ensure:
- Deterministic behavior for identical inputs
- Easy testing without mocking
- Safe execution in any environment
- Clear separation of concerns

## Implemented Checks

| Check ID | Codes | Description |
|----------|-------|-------------|
| `deps.no_wildcards` | `wildcard_version` | Disallow `*` version specs |
| `deps.path_requires_version` | `path_without_version` | Path dependencies should declare version |
| `deps.path_safety` | `absolute_path`, `parent_escape` | No absolute paths or `../` escapes |
| `deps.workspace_inheritance` | `missing_workspace_true` | Enforce workspace dependency inheritance |
| `deps.git_requires_version` | `git_without_version` | Git dependencies should declare version |
| `deps.dev_only_in_normal` | `dev_only_in_normal` | Dev-only crates appearing in normal deps |
| `deps.default_features_explicit` | `default_features_implicit` | Require explicit `default-features` |
| `deps.no_multiple_versions` | `multiple_versions` | No duplicate crate versions in workspace |
| `deps.optional_unused` | `optional_unused` | Optional dependencies should be used |
| `deps.yanked_versions` | `yanked_version` | No yanked crate versions |

## Public API

```rust
use depguard_domain::{evaluate, DomainReport, WorkspaceModel, EffectiveConfig};

// Main evaluation entry point
pub fn evaluate(model: &WorkspaceModel, cfg: &EffectiveConfig) -> DomainReport;

// Domain model types (re-exported from domain-core)
pub use depguard_domain_core::model::*;
pub use depguard_domain_core::policy::*;
```

## Determinism

Findings are **always** ordered deterministically before truncation:

```
severity -> path -> line -> check_id -> code -> message
```

This ensures byte-stable output for identical inputs, which is critical for:
- Golden file testing
- Reproducible CI builds
- Baseline suppression matching

## Usage Example

```rust
use depguard_domain::{evaluate, WorkspaceModel, EffectiveConfig};

// Build the workspace model (typically done by depguard-repo)
let model: WorkspaceModel = /* ... */;

// Build effective config (typically done by depguard-settings)
let config: EffectiveConfig = /* ... */;

// Evaluate policies
let report = evaluate(&model, &config);

// Inspect results
println!("Verdict: {:?}", report.verdict);
for finding in &report.findings {
    println!("{}: {} at {}:{}", finding.severity, finding.code, finding.path, finding.line);
}
```

## Domain Report Structure

```rust
pub struct DomainReport {
    pub findings: Vec<Finding>,
    pub verdict: VerdictStatus,
    pub counts: VerdictCounts,
    pub fingerprints: BTreeMap<String, String>,
}
```

## Dependencies

| Crate | Purpose |
|-------|---------|
| `depguard-domain-core` | Core model and policy types |
| `depguard-domain-checks` | Check implementations |
| `depguard-types` | Shared types and IDs |

## Feature Flags

All checks are enabled by default. Disable specific checks at compile time:

```toml
[dependencies]
depguard-domain = { version = "0.1.0", default-features = false, features = [
    "check-no-wildcards",
    "check-path-safety",
] }
```

## Related Crates

- [`depguard-domain-core`](../depguard-domain-core/) - Core model types
- [`depguard-domain-checks`](../depguard-domain-checks/) - Check implementations
- [`depguard-app`](../depguard-app/) - Use case orchestration
- [`depguard-repo`](../depguard-repo/) - Model construction from filesystem
