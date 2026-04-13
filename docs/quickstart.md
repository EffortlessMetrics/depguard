# Quickstart

## Problem
Getting started with dependency policy checks usually requires multiple examples and configuration choices.

## What this gets you
A working local run of `depguard` plus a first baseline-ready configuration.

## Install
```bash
cargo install depguard-cli --version 0.1.1 --bin depguard --locked
# Optional: as Cargo subcommand
cargo install depguard-cli --version 0.1.1 --bin cargo-depguard --locked
```

## First run
```bash
depguard check
```

If your project is not at repo root, run with:

```bash
depguard --repo-root /path/to/repo check
```

## Recommended onboarding flow
1. Run `depguard check` once and inspect the report.
2. Create a config file:
```toml
profile = "warn"
scope = "repo"
fail_on = "error"
max_findings = 100
```
3. Re-run with overrides as needed.
4. Add baseline once initial violations are intentional:
```bash
depguard baseline --output .depguard-baseline.json
```

## CLI you will use often
- `depguard check` for enforcement.
- `depguard explain <check_id|code>` for remediation.
- `depguard md --report artifacts/depguard/report.json` for review.
- `depguard annotations --report artifacts/depguard/report.json` for CI annotations.
- `depguard sarif --report artifacts/depguard/report.json` for third-party code scanning.
- `depguard junit --report artifacts/depguard/report.json` for test dashboards.
- `depguard jsonl --report artifacts/depguard/report.json` for log ingestion.
- `depguard fix --report artifacts/depguard/report.json [--apply]` for safe remediations.

## CI default pattern
```bash
depguard --scope diff check --base origin/main --head HEAD
```

## Next steps
- Review [`docs/config.md`](config.md) for full policy options.
- If running on large repos, start with `depguard --scope diff check`.
- For output parsing, use [`docs/output-contract.md`](output-contract.md).
