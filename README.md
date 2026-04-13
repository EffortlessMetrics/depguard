# depguard

## Problem
Maintaining dependency hygiene in Rust workspaces is often solved by ad-hoc scripts and hand-rolled checks that are hard to version, hard to audit, and hard to reuse in CI.

depguard centralizes this work as a deterministic, offline-first policy engine with stable, machine-readable receipts.

## When to use depguard
- You need the same dependency rules in local dev, CI, and audit pipelines.
- You want deterministic outputs for golden-file tests or enforcement gates.
- You want policy behavior that is easy to explain, trace, and upgrade without surprises.

## How it works (system summary)
1. `depguard-cli` collects manifests and runtime inputs.
2. `depguard-repo` and `depguard-repo-parser` build an in-memory workspace model.
3. `depguard-settings` resolves configuration and effective policy.
4. `depguard` exposes the public Rust evaluation facade backed by `depguard-domain`.
5. `depguard-app` orchestrates outputs.
6. `depguard-render` emits Markdown, annotations, SARIF, JUnit, JSONL, and report JSON.

This architecture keeps the domain model pure and deterministic, while adapters own I/O.

## Quick start

### Install
```bash
cargo install depguard-cli --version 0.1.1 --bin depguard --locked
# Optional: as Cargo subcommand
cargo install depguard-cli --version 0.1.1 --bin cargo-depguard --locked
```
For CI, pin a version with the same command and run it in the workflow.

### Run a first scan
```bash
depguard check
```

### Common CI pattern
```bash
depguard ci github \
  --event pull_request \
  --report-out artifacts/depguard/report.json
```

For cross-repo rollout guidance (PR diff lane + scheduled full-repo lane, baseline policy, and reusable workflow snippets), see [docs/org-rollout.md](docs/org-rollout.md).

### Render existing reports
```bash
depguard report md --report artifacts/depguard/report.json

depguard report annotations --report artifacts/depguard/report.json

depguard report sarif --report artifacts/depguard/report.json
```

## Reference (commands by intent)

### Policy execution
- `depguard check` — analyze manifests and write a receipt
- `depguard baseline` — generate baseline suppressions
- `depguard explain <check_id|code>` — show remediation guidance
- `depguard ci github` — CI-native mode with lane handling (`pull_request`, `push`, `schedule`, `workflow_call`, `auto`)

### Output conversion
- `depguard report md|annotations|sarif|junit|jsonl --report <path>` — grouped report output renderer
- `depguard md|annotations|sarif|junit|jsonl --report <path>` — legacy aliases

### Fixing
- `depguard fix --report <path>` — generate conservative fix plan
- `depguard fix --report <path> --apply` — apply safe fixes

### Runner options
- `cargo depguard` — Cargo subcommand wrapper
- `depguard ci github --event <pull_request|push|schedule|workflow_call|auto>` — CI-native scope strategy
- `--scope repo|diff` — scan all manifests or changed scope only
- `--repo-root`, `--config`, `--profile`, `--max-findings` control context and overrides
- Check and baseline scoped commands accept `--diff-file <path>` (requires `--scope diff` or `scope = "diff"`).
- For monorepos, set `--repo-root` to each workspace when using matrixed CI jobs.

### `check` command options
- `--out-dir` and `--report-out` — control report destination
- `--baseline`, `--report-version` — baseline and schema selection
- `--incremental`, `--cache-dir` — incremental run performance
- `--yanked-index`, `--yanked-live`, `--yanked-api-base-url` — yanked-resolution behavior
- `--write-markdown`, `--write-junit`, `--write-jsonl` plus `--markdown-out` / `--junit-out` / `--jsonl-out`
- `--mode` — standard (`exit 2` on policy failure) or cockpit (`exit 0` after writing receipt)
- `--diff-file` requires `--scope diff` (or `scope = "diff"` in config)

### `baseline` command options
- Baseline command options for scoped runs mirror `check`: `--base`, `--head`, and `--diff-file` for diff scope.

### Renderer and fix command options
- `md|sarif|junit|jsonl`
  - `--report` (input report path)
  - `--output` (write output to a file; defaults to stdout)
- `annotations`
  - `--report` (input report path)
  - `--max` (annotation count limit)
- `fix`
  - `--report` (input report path)
  - `--plan-out` (buildfix plan destination, default: `artifacts/buildfix/plan.json`)
  - `--apply` (write safe fixes in place)

## Inputs and outputs
By default, `check` writes:
- `artifacts/depguard/report.json`

Optional outputs can be enabled in the same invocation with `--write-markdown`, `--write-junit`, `--write-jsonl`, and explicit destinations via `--markdown-out`, `--junit-out`, `--jsonl-out`, or `--out-dir`.

## Exit codes
- `0` — pass (no policy failure)
- `2` — policy failure (checks above threshold)
- `1` — tool/runtime error

## Configuration sketch
Create a `depguard.toml` in repo root:

```toml
profile = "strict"
scope = "repo"
fail_on = "error"
max_findings = 100

[checks."deps.no_wildcards"]
enabled = true
severity = "error"

[checks."deps.path_requires_version"]
enabled = true
```

See [docs/config.md](docs/config.md) for the full schema and all settings.

## Non-goals
- Performing crate resolution or requiring network in default execution.
- Replacing `cargo` build tooling.
- Enforcing one-size-fits-all policy defaults.

## Documentation map
- [docs/quickstart.md](docs/quickstart.md) — practical onboarding
- [docs/config.md](docs/config.md) — configuration contract
- [docs/checks.md](docs/checks.md) — check behavior and remediation
- [docs/architecture.md](docs/architecture.md) — deeper design
- [docs/testing.md](docs/testing.md) — test strategy
- [docs/implementation-plan.md](docs/implementation-plan.md) — implementation roadmap and risks
- [docs/tasks.md](docs/tasks.md) — roadmap initiatives and owners
- [CONTRIBUTING.md](CONTRIBUTING.md) — contribution flow

## Roadmap
- [docs/roadmap.md](docs/roadmap.md) — current status, active work, and upcoming milestones.
- [docs/org-rollout.md](docs/org-rollout.md) — rollout model for multi-repo adoption.

## Workspace design constraints
- Domain crates have no filesystem/network side effects
- Output is byte-stable for same inputs
- Check IDs and finding codes are stable contracts
- Schema evolution occurs via explicit versioned schema IDs

## License
[MIT](LICENSE-MIT) OR [Apache-2.0](LICENSE-APACHE)
