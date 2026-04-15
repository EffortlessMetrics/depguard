# CI Integration

## Problem
Policy checks often behave differently across pipelines due to path and scope differences.

## Default GitHub Actions pattern

Use the reusable workflow pattern in this page as the recommended baseline.

1. Always create `artifacts/depguard/report.json`.
2. Always emit review artifacts (`markdown`, `annotations`, and optional `sarif`/`junit`/`jsonl`).
3. Use event-aware `depguard ci github` behavior for scope.
4. Enforce failures only after diagnostics are written.

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
    uses: EffortlessMetrics/depguard/.github/workflows/depguard-reusable.yml@v0.1.2
    with:
      depguard-version: 0.1.2
      event-name: ${{ github.event_name }}
      repo-root: "."
      write-markdown: true
      write-annotations: true
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
    uses: EffortlessMetrics/depguard/.github/workflows/depguard-reusable.yml@v0.1.2
    with:
      depguard-version: 0.1.2
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

Compatibility options:
- Keep using `cargo install depguard-cli` inline only if your org can tolerate rebuild time.
- A dedicated installation action is deferred and not required for the canonical
  `depguard ci github` workflow.
