# depguard Configuration

## Problem
Unclear precedence between CLI flags, config files, and profiles creates surprising policy behavior.

## Resolution order
1. CLI overrides
2. `depguard.toml`
3. Profile defaults (`strict` / `warn` / `compat`)

## Key settings

- `profile`: `strict | warn | compat`
- `scope`: `repo | diff`
- `fail_on`: `error | warning`
- `baseline`: path to baseline JSON file
- `max_findings`: integer limit

## Per-check section
```toml
[checks."deps.no_wildcards"]
enabled = true
severity = "error"
allow = ["vendor-*"]
```

## Scopes and base refs
- Use `--scope diff` for PR-only checks.
- For restricted runners, avoid git and pass `--diff-file`.

## Why profiles exist
Profiles encode migration-safe defaults and make repository policy explicit while allowing local overrides.

## Valid values summary

### `fail_on`
- `error` (default)
- `warning`
- `never`

### `scope`
- `repo`: full workspace scan
- `diff`: changed manifests only

## Good defaults
- Start with `profile = "warn"` for adoption.
- Switch to `strict` after baseline and suppression cleanup.

## Validation behavior
- Bad config and unknown IDs are surfaced as explicit errors.
- Invalid values fail fast with actionable diagnostics.
