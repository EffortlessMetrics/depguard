# Troubleshooting

## Problem
Users need fast, practical recovery paths for local and CI failures.

## Common failures and fixes

### `No Cargo.toml found`
- Ensure you are in or pointed to a Rust workspace root.
- Use `depguard --repo-root <path> check`.

### `Invalid configuration`
- Validate TOML syntax with `taplo`.
- Verify check IDs and config keys/values are valid in `depguard.toml`.
- Confirm `profile` and `fail_on` are valid values.

### `Git ref not found` during diff scope
- Ensure `--scope diff` is active (via CLI or config) and fetch depth includes the base branch:
```yaml
fetch-depth: 0
```
- Or use `--diff-file` with a precomputed manifest list.
- When using `--diff-file`, keep `scope` set to `diff` (CLI `--scope diff` or config `scope = "diff"`).

### `Permission denied` writing output
- Ensure output directory exists and is writable.
- Override output paths with explicit per-command options:
  - `--report-out` for `check` JSON reports.
  - `--markdown-out` for Markdown output.
  - `--junit-out` for JUnit.
  - `--jsonl-out` for JSONL.

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
