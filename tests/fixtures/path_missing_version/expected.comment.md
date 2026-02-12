# Depguard report

- Verdict: **FAIL**
- Findings: 1 (emitted) / 1 (total)

## Findings

- [ERROR] `deps.path_requires_version` / `path_without_version` — dependency 'my-local' uses a path dependency without an explicit version (`Cargo.toml`:11 )
  - help: Add an explicit version alongside `path = ...`, or use `workspace = true` with a workspace dependency.
