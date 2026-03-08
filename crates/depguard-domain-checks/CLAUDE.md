# CLAUDE.md — depguard-domain-checks

## Purpose

Pure check implementations and helpers for depguard domain-level rules. Contains all 10 check implementations and the `run_all()` orchestrator.

## Critical Constraint

**This crate must remain pure.** No filesystem access, no stdout/stderr, no network. All inputs come via function parameters; all outputs via return values.

## Key Modules

| Module | Contents |
|--------|----------|
| [`lib.rs`] | Re-exports, `run_all()` entry point |
| [`checks/mod.rs`] | Check runner registry, `run_all()` implementation |
| [`checks/no_wildcards.rs`] | Wildcard version detection (`*`) |
| [`checks/path_requires_version.rs`] | Path deps must have version |
| [`checks/path_safety.rs`] | No absolute paths or parent escapes (`../`) |
| [`checks/workspace_inheritance.rs`] | Workspace deps must use `workspace = true` |
| [`checks/git_requires_version.rs`] | Git deps must have version/tag/rev |
| [`checks/dev_only_in_normal.rs`] | Dev-only crates shouldn't be in normal deps |
| [`checks/default_features_explicit.rs`] | `default-features` should be explicit |
| [`checks/no_multiple_versions.rs`] | Detect duplicate crate versions |
| [`checks/optional_unused.rs`] | Optional deps should be used in features |
| [`checks/yanked_versions.rs`] | Check against yanked version index |
| [`checks/utils.rs`] | Shared check utilities |
| [`fingerprint.rs`] | Content fingerprinting for caching |
| [`model.rs`] | Re-exports from `depguard-domain-core` |
| [`policy.rs`] | Re-exports from `depguard-domain-core` |

## Public API

```rust
// Run all enabled checks against a workspace model
pub fn run_all(model: &WorkspaceModel, cfg: &EffectiveConfig, out: &mut Vec<Finding>);
```

## Check Runner Pattern

Each check follows the same signature:

```rust
fn run(model: &WorkspaceModel, cfg: &EffectiveConfig, out: &mut Vec<Finding>) {
    let Some(policy) = cfg.check_policy(CHECK_ID) else { return };
    for manifest in &model.manifests {
        for dep in &manifest.dependencies {
            // Check logic, emit findings
        }
    }
}
```

## Feature Gates

Each check is gated by a cargo feature that propagates to `depguard-check-catalog`:

```toml
check-no-wildcards = ["depguard-check-catalog/check-no-wildcards"]
```

This ensures unavailable checks are skipped at runtime via `is_check_available()`.

## Deterministic Output

Findings are sorted by: `path → line → check_id → code → message`

The `run_all()` function delegates ordering to the engine in `depguard-domain`.

## Dependencies

- `depguard-domain-core` — `WorkspaceModel`, `ManifestModel`, `DependencyDecl`, `EffectiveConfig`
- `depguard-types` — `Finding`, `Severity`, check IDs
- `depguard-check-catalog` — Feature gating
- `sha2`, `hex` — Fingerprinting
- `globset` — Pattern matching for allow lists
- `serde_json` — JSON serialization for fingerprints

Dev dependencies:
- `depguard-yanked` — For yanked_versions check tests

## Testing

```bash
cargo test -p depguard-domain-checks           # Unit tests
cargo mutants --package depguard-domain-checks # Mutation testing
```

Each check has unit tests in its module. The `checks/tests.rs` file contains shared test utilities.

## Adding a New Check

1. Add check ID and codes to `depguard-types/src/ids.rs`
2. Add explanation to `depguard-types/src/explain.rs`
3. Add catalog entry to `depguard-check-catalog/src/lib.rs`
4. Create feature gate in all crate `Cargo.toml` files
5. Implement check in `checks/<name>.rs`
6. Register runner in `checks/mod.rs` `RUNNERS` array
7. Add BDD feature file referenced in catalog
