# CLAUDE.md — depguard-domain

## Purpose

**Facade crate** for the domain layer. Re-exports model/policy types from `depguard-domain-core` and delegates check execution to `depguard-domain-checks`. Owns the evaluation engine that wraps findings into a `DomainReport`.

## Critical Constraint

**This crate must remain pure.** No filesystem access, no stdout/stderr, no network. All inputs come via function parameters; all outputs via return values.

## Key Modules

| Module | Contents |
|--------|----------|
| `model.rs` | Re-exports from `depguard-domain-core` |
| `policy.rs` | Re-exports from `depguard-domain-core` |
| `engine.rs` | `evaluate()` orchestrator, verdict computation |
| `checks/mod.rs` | Delegates to `depguard-domain-checks::run_all()` |
| `report.rs` | `DomainReport` struct |

## Internal Domain Split

Individual check files have moved to `depguard-domain-checks`:

| Crate | Responsibility |
|-------|----------------|
| `depguard-domain-core` | `WorkspaceModel`, `ManifestModel`, `DependencyDecl`, `DepSpec`, `CheckPolicy`, `EffectiveConfig`, `Scope`, `FailOn` |
| `depguard-domain-checks` | All 10 check implementations, `run_all()`, fingerprinting, check utilities |
| `depguard-check-catalog` | Check metadata, feature gates, profile defaults |
| `depguard-domain` (this crate) | Engine + orchestration + re-exports |

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

Findings are sorted by: `path → line → check_id → code → message`

## Dependencies

- `depguard-domain-core` — Model and policy types
- `depguard-domain-checks` — Check execution
- `depguard-types` — DTOs and IDs
- `thiserror` — Error types

Dev dependencies: `proptest`, `rand` for property testing

## Testing

```bash
cargo test -p depguard-domain       # Engine + property tests
cargo test -p depguard-domain-checks # Check unit tests
cargo mutants --package depguard-domain-checks  # Mutation testing
```
