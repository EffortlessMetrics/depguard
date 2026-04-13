# Multi-Repo Rollout Guide for depguard

## Standard adoption pattern

For most Rust repositories, a stable rollout should include the same four elements:

1. A committed `depguard.toml` at repo root, which defines the policy contract.
2. A committed `.depguard-baseline.json` for brownfield repos to stage existing policy debt.
3. A CI run that always writes `artifacts/depguard/report.json`.
4. At least one human review format (for example Markdown and/or GitHub annotations).

These map to the project contracts: config lives in `depguard.toml`, baseline generation is an onboarding action, check writes JSON receipt, and renderer commands consume that report.

## Classify repo topology before rollout

- **Single Rust workspace:** one check run at the workspace root is usually enough.
- **Monorepo with multiple workspaces:** use a matrix and pass `--repo-root` per workspace.
- **Restricted CI environment (no git history):** run diff checks with `--diff-file` rather than `--base/--head`.

## Rollout command shape

Use `depguard ci github` for CI-native lane control:

- PRs: lightweight, fast feedback on the change set (`depguard ci github --event pull_request`).
- Mainline/scheduled jobs: periodic full scan with repo scope (`depguard ci github --event push|schedule`).

Because depguard is manifest-only and offline-by-default, these jobs fit early in most pipelines.

## Brownfield onboarding sequence

1. Run a full repo scan locally first:

```bash
depguard check
```

2. Review findings in the resulting receipt (`artifacts/depguard/report.json`), then generate a baseline:

```bash
depguard baseline --output .depguard-baseline.json
```

3. Add baseline and policy config to version control, then add CI gating for diff scope.

4. Tighten required checks over time rather than gating all checks immediately.

## Suggested starter config for brownfield repos

```toml
profile = "warn"
scope = "repo"
fail_on = "error"
baseline = ".depguard-baseline.json"
max_findings = 100

[checks."deps.no_wildcards"]
enabled = true
severity = "error"

[checks."deps.path_requires_version"]
enabled = true
severity = "error"

[checks."deps.path_safety"]
enabled = true
severity = "error"

[checks."deps.workspace_inheritance"]
enabled = true
severity = "warning"

[checks."deps.dev_only_in_normal"]
enabled = true
severity = "error"
```

Keep the most disruptive checks for later as each repo matures.

## GitHub Actions reference (diff + repo lanes)

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
          depguard ci github \
            --event "${{ github.event_name }}" \
            --out-dir artifacts/depguard \
            --report-out artifacts/depguard/report.json \
            --write-markdown \
            --emit-annotations
          echo "exit_code=$?" >> "$GITHUB_OUTPUT"
          exit 0

      - name: Emit annotations
        if: always()
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

## Reusable workflow option

After you standardize one local baseline in your repo, consume this directly from org repos:

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
      write-markdown: true
      write-annotations: true
      write-junit: true
      write-jsonl: true
      write-sarif: true
      # For monorepos, include a matrix and pass workspace paths:
      # repo-root: ${{ matrix.workspace }}
      # Optional for restricted checkouts:
      # base-ref: origin/main
      # head-ref: ${{ github.sha }}
      # diff-file: .github/changed-manifests.txt
      # max-annotations: 25
```

- For monorepos, run one scan per workspace via caller matrix:
  ```yaml
  jobs:
    depguard:
      strategy:
        matrix:
          workspace:
            - crates/ops
            - crates/api
      uses: EffortlessMetrics/depguard/.github/workflows/depguard-reusable.yml@v0.1.1
      with:
        depguard-version: 0.1.1
        event-name: ${{ github.event_name }}
        repo-root: ${{ matrix.workspace }}
  ```

This keeps scope detection, artifact publishing, and post-run diagnostics in one centrally maintained place.

Use the same model in other CI platforms:

- PR/MR runs: `depguard ci github --event pull_request --report-out artifacts/depguard/report.json`
- Mainline/scheduled runs: `depguard ci github --event push --report-out artifacts/depguard/report.json`
- Restricted runners: `depguard ci github --event pull_request --diff-file changed-manifests.txt --report-out artifacts/depguard/report.json`

After rendering the JSON receipt:

- `depguard report md --report artifacts/depguard/report.json`
- `depguard report annotations --report artifacts/depguard/report.json`
- `depguard report sarif --report artifacts/depguard/report.json` (optional)
- `depguard report junit --report artifacts/depguard/report.json` (optional)

## CI install guidance

Keep CI install explicit and minimal for now:

- `cargo install depguard-cli --version 0.1.1 --bin depguard --locked`
- A dedicated installation action can be introduced later if CI time becomes an issue.

## Common mistakes

- Using `--scope diff` without full git history.
- Using PR-only flow in push/scheduled jobs.
- Failing the job before report and annotations are emitted.
- Starting with strict diff gating before baseline creation.
- Parsing console output instead of `artifacts/depguard/report.json`.
- Pointing one scan at a multi-workspace monorepo without per-workspace scope.

## Rollout order suggestion

1. Pilot one clean repo and one brownfield repo.
2. Establish a shared profile/check set and baseline policy.
3. Add full-lane repository scan jobs (even when PR diff gating is the blocker).
4. Freeze the template and roll into remaining repos.
