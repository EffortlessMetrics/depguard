# depguard — Checks and Codes

This document is the source of truth for check IDs, codes, and remediation guidance.

## General conventions

- `check_id` identifies the producer check (stable)
- `code` classifies the specific condition (stable)
- Codes must never be renamed; deprecate via aliases only
- Each emitted code must have an explain entry

Severity defaults are profile-driven (oss|team|strict) and may be overridden by config.

---

## deps.no_wildcards

Flags dependency versions containing `*` (e.g., `"*"`, `"1.*"`).

**Codes**
- `wildcard_version`

**Default severity**
- oss: warn
- team: error
- strict: error

**Remediation**
- Pin a semver requirement (e.g., `"1"`, `"^1.2"`, `"~1.2.3"`).
- Prefer centralizing in `[workspace.dependencies]` and inheriting with `{ workspace = true }`.

---

## deps.path_requires_version

Flags `{ path = "..." }` dependencies missing `version`.

**Codes**
- `missing_version`

**Default severity**
- oss: warn (often), configurable
- team: error
- strict: error

**Notes**
- Config may ignore this when the *owning crate* has `package.publish = false`.

**Remediation**
- Add `version = "x.y.z"` matching the target crate.
- Or centralize in `[workspace.dependencies]` and inherit with `{ workspace = true }`.

---

## deps.path_safety

Flags path dependencies that are absolute or escape the repo root lexically.

**Codes**
- `absolute_path`
- `escapes_root`

**Default severity**
- oss: warn
- team: error
- strict: error

**Remediation**
- Use repo-relative paths that do not escape the workspace root.
- Avoid `..` segments that move outside the workspace boundary.
- Consider moving the dependency into the workspace if it’s truly internal.

---

## deps.workspace_inheritance

If the workspace root defines `[workspace.dependencies] <dep> = ...`, member crates should use `<dep> = { workspace = true }`.

**Codes**
- `not_inherited`

**Default severity**
- oss: skip (off by default)
- team: warn (often)
- strict: error (often)

**Remediation**
- Replace member entry with `{ workspace = true }`.
- Preserve member flags (`features`, `optional`, `default-features`, etc.).

---

## Shared / standardized codes

These should be standardized ecosystem-wide:

- `tool.runtime_error`
  - Tool-level failure (I/O, parse crash prevented, internal errors)
  - Emitted with `verdict.status="fail"` and `verdict.reasons=["tool_error"]`
