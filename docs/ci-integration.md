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
      - name: Install depguard
        run: cargo install depguard-cli --version 0.1.1 --bin depguard --locked
      - name: Prepare artifacts
        run: mkdir -p artifacts/depguard
      - name: Run depguard
        id: depguard
        shell: bash
        run: |
          set +e
          depguard ci github \
            --event "${{ github.event_name }}" \
            --out-dir artifacts/depguard \
            --report-out artifacts/depguard/report.json \
            --write-markdown \
            --emit-annotations
          echo "exit_code=$?" >> "$GITHUB_OUTPUT"
          exit 0
      - name: Emit GitHub annotations
        if: always()
        shell: bash
        run: |
          if [ -f artifacts/depguard/comment.md ]; then
            cat artifacts/depguard/comment.md
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
- PR jobs: `depguard ci github --event pull_request`.
- Push/schedule jobs: `depguard ci github --event push` (or `--event schedule`).
- Restricted environments without git history: use `--diff-file` with `depguard ci github`.

## Exit code handling
- `0` pass
- `1` tool/runtime failure
- `2` policy failure

Use step-level enforcement after report generation when you need custom diagnostics.

## Artifact strategy
- Keep `artifacts/depguard/report.json` as the source of truth.
- Produce `markdown` and optional `junit/sarif/jsonl` from that file.
- Upload outputs as CI artifacts so failed runs remain reviewable.

## Reusable workflow

For org-wide rollout, consume the standardized reusable workflow directly:

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
    uses: EffortlessMetrics/depguard/.github/workflows/depguard-reusable.yml@v0.1.1
    with:
      depguard-version: 0.1.1
      event-name: ${{ github.event_name }}
      repo-root: "."
      write-markdown: true
      write-annotations: true
      write-junit: true
      write-jsonl: true
      write-sarif: true
      # Optional for monorepos:
      # repo-root: crates/my-workspace
      # Optional when restricting CI runners without full Git history:
      # diff-file: changed-manifests.txt
      # max-annotations: 25
```

The reusable workflow uses `depguard ci github` and preserves the same event behavior (`pull_request` uses diff scope; other event modes use repo scope), always writes `artifacts/depguard/report.json`, and emits markdown/annotations/optional renderer outputs before exit-code enforcement.

## Alternative CI systems
The same commands work for GitLab/CircleCI/Jenkins as long as working directory and checkout depth are consistent.

## Install options

For CI today, pin `depguard-cli` in the workflow with `cargo install`.
If you prefer a dedicated installation action later, you can keep that outside the default
path and keep the same `depguard ci github` command after it.
