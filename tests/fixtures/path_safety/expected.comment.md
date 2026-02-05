# Depguard report

- Verdict: **FAIL**
- Findings: 5 (emitted) / 5 (total)

## Findings

- [ERROR] `deps.path_safety` / `absolute_path` — dependency 'abs-unix' uses an absolute path: /opt/libs/abs-unix (`Cargo.toml`:9 )
  - help: Use repo-relative paths. Absolute paths are not portable and may leak host layout.
- [ERROR] `deps.path_safety` / `absolute_path` — dependency 'abs-windows-backslash' uses an absolute path: C:\libs\abs-windows (`Cargo.toml`:12 )
  - help: Use repo-relative paths. Absolute paths are not portable and may leak host layout.
- [ERROR] `deps.path_safety` / `absolute_path` — dependency 'abs-windows-forward' uses an absolute path: D:/projects/shared-lib (`Cargo.toml`:16 )
  - help: Use repo-relative paths. Absolute paths are not portable and may leak host layout.
- [ERROR] `deps.path_safety` / `parent_escape` — dependency 'escaping-deep' uses a path that escapes the repo root: ../../../../../../../../outside-repo (`Cargo.toml`:20 )
  - help: Avoid `..` segments that escape the repository root.
- [ERROR] `deps.path_safety` / `parent_escape` — dependency 'escaping-minimal' uses a path that escapes the repo root: ../outside (`Cargo.toml`:23 )
  - help: Avoid `..` segments that escape the repository root.
