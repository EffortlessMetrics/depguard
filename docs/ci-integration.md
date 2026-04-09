# CI Integration

## Problem
Policy checks often behave differently across pipelines due to path and scope differences.

## Default GitHub Actions pattern

```yaml
name: depguard
on: [pull_request, push]

jobs:
  depguard:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v6
        with:
          fetch-depth: 0
      - name: Install depguard
        run: cargo install depguard-cli --version 0.1.1 --bin depguard --locked
      - name: Run checks
        run: depguard --scope diff check --base origin/${{ github.base_ref }} --head HEAD
      - name: Post markdown report
        run: depguard md --report artifacts/depguard/report.json
```

## Scope strategy
- PR jobs: `--scope diff`.
- Scheduled/integration jobs: `--scope repo`.
- Restricted environments without git history: use `--diff-file`.

## Exit code handling
- `0` pass
- `1` tool/runtime failure
- `2` policy failure

Use step-level error handling when you want custom annotation of status.

## Artifact strategy
- keep `artifacts/depguard/report.json` for troubleshooting.
- optionally produce `markdown`, `junit`, `sarif`, `jsonl` for downstream systems.

## Alternative CI systems
The same commands work for GitLab/CircleCI/Jenkins as long as working directory and checkout depth are consistent.
