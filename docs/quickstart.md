# Quick Start

Get depguard running on your Rust workspace in 5 minutes.

## Installation

### From source

```bash
# Install directly
cargo install --path crates/depguard-cli

# Or build and use locally
cargo build --release
./target/release/depguard --help

# Cargo subcommand also works
cargo depguard --help
```

## First run

Navigate to your Rust workspace and run:

```bash
depguard check
```

This analyzes all `Cargo.toml` files and outputs a JSON report to `artifacts/depguard/report.json`.
Use `--write-markdown` to also write `artifacts/depguard/comment.md`.

### Example output

```
$ depguard check
Scanning workspace: /path/to/my-project
Found 5 manifests

✗ 2 findings (1 error, 1 warning)

Report written to artifacts/depguard/report.json
```

## Understanding the output

### Report structure

The JSON report contains:

```json
{
  "schema": "depguard.report.v2",
  "tool": { "name": "depguard", "version": "0.1.0" },
  "run": { "started_at": "...", "ended_at": "...", "duration_ms": 12 },
  "verdict": { "status": "fail", "counts": { "info": 0, "warn": 0, "error": 1 }, "reasons": [] },
  "findings": [...],
  "data": {
    "scope": "repo",
    "profile": "strict",
    "manifests_scanned": 5,
    "dependencies_scanned": 42
  }
}
```

Need the legacy schema? Run `depguard check --report-version v1`.

### Verdicts

| Verdict | Meaning |
|---------|---------|
| `pass` | No issues found |
| `warn` | Warnings only, below `fail_on` threshold |
| `fail` | Errors found, above `fail_on` threshold |

### Findings

Each finding includes:

- **severity**: `info`, `warn`, or `error`
- **check_id**: Which check triggered (e.g., `deps.no_wildcards`)
- **code**: Specific condition (e.g., `wildcard_version`)
- **message**: Human-readable description
- **location**: File path and line number

## Get help on findings

Use `depguard explain` to understand what a finding means and how to fix it:

```bash
# By check ID
depguard explain deps.no_wildcards

# By code
depguard explain wildcard_version
```

Output:

```
deps.no_wildcards — Wildcard Version Detection

Detects wildcard version specifiers that allow any version.

Example (bad):
  serde = "*"
  tokio = "1.*"

Example (good):
  serde = "1.0"
  tokio = "1.35"

Remediation:
  Pin to a specific version or version range.
  Use `cargo update` to find the latest compatible version.
```

## Generate readable output

### Markdown report

```bash
depguard md --report artifacts/depguard/report.json
```

Outputs a Markdown summary suitable for PR comments or documentation.

### GitHub annotations

```bash
depguard annotations --report artifacts/depguard/report.json
```

Outputs GitHub Actions workflow commands that create inline annotations on your PR.

### SARIF output

```bash
depguard sarif --report artifacts/depguard/report.json
```

Outputs SARIF v2.1.0 JSON for GitHub Advanced Security and other SARIF consumers.

### JUnit XML output

```bash
depguard junit --report artifacts/depguard/report.json
```

Outputs JUnit XML suitable for legacy CI systems and test report dashboards.

### JSON Lines output

```bash
depguard jsonl --report artifacts/depguard/report.json
```

Outputs newline-delimited JSON (`.jsonl`) with one finding event per line plus a summary line.

### Buildfix plan and safe auto-fix

```bash
# Generate buildfix.plan.v1 from report findings
depguard fix --report artifacts/depguard/report.json

# Apply only conservative safe fixes in-place
depguard fix --report artifacts/depguard/report.json --apply
```

By default this writes `artifacts/buildfix/plan.json`. The current safe auto-fix scope is intentionally narrow.

## Basic configuration

Create `depguard.toml` in your repo root:

```toml
# Use a lenient profile for gradual adoption
profile = "warn"

# Only fail on errors, not warnings
fail_on = "error"

# Enable workspace inheritance enforcement
[checks."deps.workspace_inheritance"]
enabled = true
```

### Profiles

| Profile | Description |
|---------|-------------|
| `strict` | All checks at `error` severity (default) |
| `warn` | All checks at `warning` severity |
| `compat` | Lenient defaults for gradual adoption |

## Diff-scope mode

Only analyze manifests changed in a PR:

```bash
depguard check --scope diff --base origin/main --head HEAD
```

Or use a precomputed changed-files list (for CI environments without git history):

```bash
depguard check --scope diff --diff-file changed-files.txt
```

This is useful for:
- Faster CI runs
- Gradual adoption (only new code must comply)
- Reducing noise on large existing codebases

## Yanked-version checks

Enable the check in config:

```toml
[checks."deps.yanked_versions"]
enabled = true
severity = "error"
```

Run with an offline yanked index:

```bash
depguard check --yanked-index yanked-index.txt
```

Or query crates.io live:

```bash
depguard check --yanked-live
```

Supported index formats include JSON maps and simple line format (`crate version` or `crate@version`).

## Baseline mode

Generate a baseline for existing violations, then fail only on new findings:

```bash
depguard baseline --output .depguard-baseline.json
depguard check --baseline .depguard-baseline.json
```

You can also set `baseline = ".depguard-baseline.json"` in `depguard.toml`.
When baseline suppression is active, suppressed findings are counted in `verdict.counts.suppressed`.

## Incremental mode

Cache parsed manifests between runs:

```bash
depguard check --incremental
```

By default depguard writes cache data to `.depguard-cache/` (override with `--cache-dir`).

## Inline suppression comments

For one-off dependency exceptions, add an inline suppression comment next to the dependency:

```toml
[dependencies]
serde = "*" # depguard: allow(no_wildcards)
```

You can suppress by check ID or by code:
- `# depguard: allow(deps.no_wildcards)`
- `# depguard: allow(wildcard_version)`

## Exit codes

| Code | Meaning | CI behavior |
|------|---------|-------------|
| `0` | Pass | Success |
| `1` | Tool error | Fail (fix config/setup) |
| `2` | Policy failure | Fail (fix code) |

## Optional pre-commit hook

Install the included hook:

```bash
git config core.hooksPath .githooks
chmod +x .githooks/pre-commit
```

The hook runs depguard against staged files (diff scope) before commit and writes `artifacts/depguard/report.json`.

## Next steps

- [Configuration Reference](config.md) — Full config options
- [Checks Catalog](checks.md) — All checks with examples
- [CI Integration](ci-integration.md) — GitHub Actions setup
- [Troubleshooting](troubleshooting.md) — Common issues

## Common workflows

### Initial adoption

1. Start with `profile = "compat"` to see current state
2. Review findings and fix easy wins
3. Add allowlists for intentional exceptions
4. Gradually move to `profile = "strict"`

### CI gating

1. Add `depguard check` to your CI pipeline
2. Use `--scope diff` to only check changed files
3. Set `fail_on = "error"` to block on serious issues
4. Use warnings for advisory issues

### Monorepo setup

Depguard automatically discovers workspace members:

```toml
# Root Cargo.toml
[workspace]
members = ["crates/*", "tools/*"]
exclude = ["experiments/*"]
```

All discovered manifests are analyzed unless scoped by diff.

