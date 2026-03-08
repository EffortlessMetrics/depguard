# depguard-domain-checks

Pure check implementations and helpers for depguard domain-level rules.

This crate contains the actual implementation of each dependency hygiene check. It depends on `depguard-domain-core` for model/policy types and `depguard-check-catalog` for availability gating.

## Purpose

The domain-checks crate provides:
- Concrete implementations of all dependency hygiene checks
- Fingerprint calculation for findings
- Test support utilities for check testing
- Feature-gated check availability

## Key Features

### Check Implementations

Each check is implemented as a separate module with:
- **Feature gate**: Compile-time enable/disable via Cargo features
- **Pure functions**: No side effects, deterministic output
- **Comprehensive testing**: Unit tests and property tests

### Available Checks

| Module | Check ID | Description |
|--------|----------|-------------|
| `no_wildcards` | `deps.no_wildcards` | Disallow wildcard version specs (`*`) |
| `path_requires_version` | `deps.path_requires_version` | Path deps should declare version |
| `path_safety` | `deps.path_safety` | No absolute paths or parent directory escapes |
| `workspace_inheritance` | `deps.workspace_inheritance` | Enforce workspace dependency inheritance |
| `git_requires_version` | `deps.git_requires_version` | Git dependencies should declare version |
| `dev_only_in_normal` | `deps.dev_only_in_normal` | Dev-only crates in normal dependencies |
| `default_features_explicit` | `deps.default_features_explicit` | Require explicit default-features setting |
| `no_multiple_versions` | `deps.no_multiple_versions` | No duplicate crate versions |
| `optional_unused` | `deps.optional_unused` | Optional dependencies should be used |
| `yanked_versions` | `deps.yanked_versions` | No yanked crate versions |

## Public API

```rust
// Run all enabled checks against a workspace model
pub fn run_all(
    model: &WorkspaceModel,
    policy: &PolicyConfig,
    yanked_index: Option<&YankedIndex>,
) -> Vec<Finding>;

// Fingerprint calculation for findings
pub fn fingerprint_finding(finding: &Finding) -> String;

// Model types (re-exported from domain-core)
pub mod model;
pub mod policy;
```

## Usage Example

```rust
use depguard_domain_checks::{run_all, model::WorkspaceModel, policy::PolicyConfig};

// Build workspace model
let model: WorkspaceModel = /* ... */;

// Configure policy
let policy: PolicyConfig = PolicyConfig::default();

// Run all checks
let findings = run_all(&model, &policy, None);

for finding in findings {
    println!("{}: {} - {}", finding.check_id, finding.code, finding.message);
}
```

## Check Implementation Pattern

Each check follows a consistent pattern:

```rust
pub(crate) fn check(
    model: &WorkspaceModel,
    policy: &CheckPolicy,
) -> Vec<Finding> {
    let mut findings = Vec::new();
    
    for manifest in &model.manifests {
        for dep in &manifest.dependencies {
            if let Some(finding) = evaluate_dependency(dep, policy) {
                findings.push(finding);
            }
        }
    }
    
    findings
}
```

## Dependencies

| Crate | Purpose |
|-------|---------|
| `depguard-domain-core` | Core model and policy types |
| `depguard-types` | Finding types, IDs, severity |
| `depguard-check-catalog` | Feature gating and metadata |
| `serde_json` | JSON serialization for fingerprints |
| `sha2` | SHA-256 hashing for fingerprints |
| `hex` | Hex encoding for fingerprints |
| `globset` | Glob pattern matching |

## Feature Flags

All checks are enabled by default. Disable specific checks at compile time:

```toml
[dependencies]
depguard-domain-checks = { version = "0.1.0", default-features = false, features = [
    "check-no-wildcards",
    "check-path-safety",
] }
```

## Test Support

The crate includes a `test_support` module (compiled only in test mode) that provides utilities for writing check tests:

```rust
#[cfg(test)]
mod test_support;

// Usage in tests
use test_support::{build_test_manifest, make_dependency};
```

## Related Crates

- [`depguard-domain`](../depguard-domain/) - Main domain entry point
- [`depguard-domain-core`](../depguard-domain-core/) - Core model types
- [`depguard-check-catalog`](../depguard-check-catalog/) - Check metadata
