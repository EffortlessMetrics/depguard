# CLAUDE.md — depguard-domain

## Purpose

Pure policy evaluation engine. This is the **core business logic** layer—no I/O, no filesystem, no clap dependencies.

## Critical Constraint

**This crate must remain pure.** No filesystem access, no stdout/stderr, no network. All inputs come via function parameters; all outputs via return values.

## Key Modules

| Module | Contents |
|--------|----------|
| `model.rs` | `WorkspaceModel`, `ManifestModel`, `DependencyDecl`, `DepSpec` |
| `policy.rs` | `Scope`, `FailOn`, `CheckPolicy`, `EffectiveConfig` |
| `engine.rs` | `evaluate()` orchestrator, verdict computation |
| `checks/` | Rule implementations (one module per check) |
| `report.rs` | `DomainReport` struct |

## Checks

Each check lives in `checks/` and exports a `run()` function:

| Check | File | Detects |
|-------|------|---------|
| `no_wildcards` | `checks/no_wildcards.rs` | `*` or `1.*` in version specs |
| `path_requires_version` | `checks/path_requires_version.rs` | Path deps without version |
| `path_safety` | `checks/path_safety.rs` | Absolute paths, parent escapes (`../`) |
| `workspace_inheritance` | `checks/workspace_inheritance.rs` | Deps not using `workspace = true` |

## Evaluation Flow

```
WorkspaceModel + EffectiveConfig
    → checks::run_all()
    → sort findings deterministically
    → truncate to max_findings
    → compute verdict
    → DomainReport
```

## Deterministic Ordering

Findings are sorted by: `path → line → check_id → code → message`

## Dependencies

- `depguard-types` — DTOs and IDs
- `thiserror` — Error types
- `serde_json` — Finding data payloads

Dev dependencies: `proptest`, `rand` for property testing

## Testing

```bash
cargo test -p depguard-domain       # Unit tests
cargo mutants --package depguard-domain  # Mutation testing
```

Mutation testing is required on this crate to ensure rule logic is properly asserted.
