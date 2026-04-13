# CI Integration

## Problem
Policy checks often behave differently across pipelines due to path and scope differences.

## Default GitHub Actions pattern

Follow the reference pattern in [docs/org-rollout.md](org-rollout.md) for a stable PR-vs-mainline workflow:

1. Always create `artifacts/depguard/report.json`.
2. Always emit review artifacts (Markdown/annotations).
3. Report results to the correct scope per event.
4. Enforce failures only after diagnostics are published.

```yaml
name: depguard

on:
  pull_request:
    branches: [main]
  push:
    branches: [main]
  schedule:
    - cron: "0 3 * * 1"

jobs:
  depguard:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v6
        with:
          fetch-depth: 0
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - name: Install depguard
        run: cargo install depguard-cli --version 0.1.1 --bin depguard --locked
      - name: Prepare artifacts
        run: mkdir -p artifacts/depguard
      - name: Run depguard
        id: depguard
        shell: bash
        run: |
          set +e
          if [ "${{ github.event_name }}" = "pull_request" ]; then
            depguard --scope diff check \
              --base origin/${{ github.base_ref }} \
              --head HEAD \
              --report-out artifacts/depguard/report.json \
              --write-markdown \
              --markdown-out artifacts/depguard/comment.md
          else
            depguard --scope repo check \
              --report-out artifacts/depguard/report.json \
              --write-markdown \
              --markdown-out artifacts/depguard/comment.md
          fi
          echo "exit_code=$?" >> "$GITHUB_OUTPUT"
          exit 0
      - name: Emit GitHub annotations
        if: always()
        shell: bash
        run: |
          if [ -f artifacts/depguard/report.json ]; then
            depguard annotations --report artifacts/depguard/report.json
          fi
      - name: Upload depguard artifacts
        if: always()
        uses: actions/upload-artifact@v7
        with:
          name: depguard-report
          path: artifacts/depguard/
      - name: Enforce result
        if: always()
        shell: bash
        run: |
          code="${{ steps.depguard.outputs.exit_code }}"
          if [ "$code" = "0" ]; then
            exit 0
          elif [ "$code" = "2" ]; then
            exit 2
          else
            exit 1
          fi
```

## Scope strategy
- PR jobs: `--scope diff`.
- Scheduled/integration jobs: `--scope repo`.
- Restricted environments without git history: use `--diff-file` with `--scope diff`.

## Exit code handling
- `0` pass
- `1` tool/runtime failure
- `2` policy failure

Use step-level enforcement after report generation when you need custom diagnostics.

## Artifact strategy
- Keep `artifacts/depguard/report.json` as the source of truth.
- Produce `markdown` and optional `junit/sarif/jsonl` from that file.
- Upload outputs as CI artifacts so failed runs remain reviewable.

## Alternative CI systems
The same commands work for GitLab/CircleCI/Jenkins as long as working directory and checkout depth are consistent.
