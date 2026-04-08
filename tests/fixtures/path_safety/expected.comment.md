# Depguard report

- Verdict: **FAIL**
- Findings: 5 (emitted) / 5 (total)

## Summary

5 errors

## Findings

### ERROR

#### `deps.path_safety`

- `deps.path_safety` / `absolute_path` — dependency 'abs-unix' uses an absolute path: /opt/libs/abs-unix ([`Cargo.toml`:11](Cargo.toml:L11))
  - help: Use repo-relative paths. Absolute paths are not portable and may leak host layout.
- `deps.path_safety` / `absolute_path` — dependency 'abs-windows-backslash' uses an absolute path: C:\libs\abs-windows ([`Cargo.toml`:14](Cargo.toml:L14))
  - help: Use repo-relative paths. Absolute paths are not portable and may leak host layout.
- `deps.path_safety` / `absolute_path` — dependency 'abs-windows-forward' uses an absolute path: D:/projects/shared-lib ([`Cargo.toml`:18](Cargo.toml:L18))
  - help: Use repo-relative paths. Absolute paths are not portable and may leak host layout.
- `deps.path_safety` / `parent_escape` — dependency 'escaping-deep' uses a path that escapes the repo root: ../../../../../../../../outside-repo ([`Cargo.toml`:22](Cargo.toml:L22))
  - help: Avoid `..` segments that escape the repository root.
- `deps.path_safety` / `parent_escape` — dependency 'escaping-minimal' uses a path that escapes the repo root: ../outside ([`Cargo.toml`:25](Cargo.toml:L25))
  - help: Avoid `..` segments that escape the repository root.
