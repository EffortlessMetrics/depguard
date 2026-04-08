# Depguard report

- Verdict: **FAIL**
- Findings: 1 (emitted) / 1 (total)

## Summary

1 error

## Findings

### ERROR

#### `deps.workspace_inheritance`

- `deps.workspace_inheritance` / `missing_workspace_true` — dependency 'serde' exists in [workspace.dependencies] but is not declared with `workspace = true` ([`member-crate/Cargo.toml`:11](member-crate/Cargo.toml:L11))
  - help: Prefer `workspace = true` to inherit the workspace dependency version and features.
