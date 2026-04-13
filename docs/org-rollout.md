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

Use `--scope diff` for PR gating and `--scope repo` for regular drift/full-repo scans:

- PRs: lightweight, fast feedback on the change set.
- Mainline/scheduled jobs: periodic full scan to catch drift, stale baseline, or policy gaps.

Because depguard is manifest-only and offline-by-default, these jobs fit early in most pipelines.

## Brownfield onboarding sequence

1. Run a full repo scan locally first:

```bash
cargo install depguard-cli --version 0.1.1 --bin depguard --locked
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

      - name: Emit annotations
        if: always()
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

Use the same model in other CI platforms:

- PR/MR runs: `depguard --scope diff check --base ... --head ... --report-out artifacts/depguard/report.json`
- Mainline/scheduled runs: `depguard --scope repo check --report-out artifacts/depguard/report.json`
- Restricted runners: replace base/head with `--diff-file changed-manifests.txt`

After rendering the JSON receipt:

- `depguard md --report artifacts/depguard/report.json`
- `depguard annotations --report artifacts/depguard/report.json`
- `depguard sarif --report artifacts/depguard/report.json` (optional)
- `depguard junit --report artifacts/depguard/report.json` (optional)

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
