# CLAUDE.md — depguard-domain

## Purpose

Internal engine crate for the domain layer. Re-exports model/policy types from `depguard-domain-core`, delegates check execution to `depguard-domain-checks`, and owns the evaluation engine that wraps findings into a `DomainReport`.

## Critical Constraint

**This crate must remain pure.** No filesystem access, no stdout/stderr, no network. All inputs come via function parameters; all outputs via return values.

## Key Modules

| Module | Contents |
|--------|----------|
| [`model.rs`] | Re-exports from `depguard-domain-core` |
| [`policy.rs`] | Re-exports from `depguard-domain-core` |
| [`engine.rs`] | `evaluate()` orchestrator, verdict computation |
| [`fingerprint.rs`] | Content fingerprinting for caching |
| [`report.rs`] | `DomainReport` struct |
| [`proptest.rs`] | Property-based test strategies |

## Domain Crate Split

| Crate | Responsibility |
|-------|----------------|
| `depguard` | Public facade and stable import surface |
| `depguard-domain-core` | `WorkspaceModel`, `ManifestModel`, `DependencyDecl`, `DepSpec`, `CheckPolicy`, `EffectiveConfig`, `Scope`, `FailOn` |
| `depguard-domain-checks` | All 10 check implementations, `run_all()`, check utilities |
| `depguard-check-catalog` | Check metadata, feature gates, profile defaults |
| `depguard-domain` (this crate) | Engine + orchestration + re-exports used by `depguard` |

## Public API

```rust
// Re-exports from domain-core
pub use depguard_domain_core::model::*;
pub use depguard_domain_core::policy::*;

// Evaluation engine
pub fn evaluate(model: &WorkspaceModel, cfg: &EffectiveConfig) -> DomainReport;

// Fingerprinting
pub fn fingerprint_model(model: &WorkspaceModel) -> String;
```

## Evaluation Flow

```
WorkspaceModel + EffectiveConfig
    → depguard-domain-checks::run_all()
    → sort findings deterministically
    → truncate to max_findings
    → compute verdict
    → DomainReport
```

## Deterministic Ordering

Findings are sorted by: `severity → path → line → check_id → code → message`

## Feature Gates

This crate propagates check features to `depguard-domain-checks`:

```toml
check-no-wildcards = ["depguard-domain-checks/check-no-wildcards"]
```

All 10 checks have corresponding features.

## Dependencies

| Dependency | Purpose |
|------------|---------|
| `depguard-domain-core` | Model and policy types |
| `depguard-domain-checks` | Check execution |
| `depguard-types` | DTOs and IDs |
| `serde` | Serialization |
| `serde_json` | JSON handling |

Dev dependencies:
- `proptest` — Property-based testing
- `rand` — Test utilities

## Testing

```bash
cargo test -p depguard-domain            # Engine + property tests
cargo test -p depguard-domain-checks     # Check unit tests
cargo mutants --package depguard-domain  # Mutation testing
```

## Architecture Notes

The domain layer is split into four crates to:
1. Allow `depguard-settings` to depend only on core types (not checks)
2. Allow check implementations to be feature-gated independently
3. Keep `depguard-domain` as the simple engine/orchestrator
4. Keep `depguard` as the public facade crate

```
depguard-settings → depguard-domain-core (model/policy only)
depguard-domain-checks → depguard-domain-core + depguard-check-catalog
depguard-domain → depguard-domain-core + depguard-domain-checks
depguard → depguard-domain
```
