# Depguard report

- Verdict: **FAIL**
- Findings: 1 (emitted) / 1 (total)

## Findings

- [ERROR] `deps.workspace_inheritance` / `missing_workspace_true` — dependency 'serde' exists in [workspace.dependencies] but is not declared with `workspace = true` (`member-crate/Cargo.toml`:11 )
  - help: Prefer `workspace = true` to inherit the workspace dependency version and features.
