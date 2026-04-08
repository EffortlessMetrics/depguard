# Troubleshooting

## Problem
Users need fast, practical recovery paths for local and CI failures.

## Common failures and fixes

### `No Cargo.toml found`
- Ensure you are in or pointed to a Rust workspace root.
- Use `depguard check --repo-root <path>`.

### `Invalid configuration`
- Validate TOML syntax with `taplo`.
- Verify check IDs are in `depguard.types` registry.
- Confirm `profile` and `fail_on` are valid values.

### `Git ref not found` during diff scope
- Ensure fetch depth includes base branch:
```yaml
fetch-depth: 0
```
- Or use `--diff-file` with a precomputed manifest list.

### `Permission denied` writing output
- Ensure output directory exists and is writable.
- Override output paths with `--out-dir`/`--report` equivalents.

### Unexpected exit code `2`
- Inspect findings in `artifacts/depguard/report.json`.
- Run `depguard explain <code>`.

## CI-specific checks
- Keep checkout working directory consistent.
- Capture command output for logs.
- Verify relative paths in annotations.

## When to escalate
If a reproducible inconsistency remains after config cleanup, capture:
- Inputs (`Cargo.toml` set)
- `depguard.toml`
- Command + args
- Full JSON report
- Version string

Then open an issue with minimal repro artifacts.
