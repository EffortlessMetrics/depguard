# depguard

## Problem
Maintaining dependency hygiene in Rust workspaces is often solved by ad-hoc scripts and hand-rolled checks that are hard to version, hard to audit, and hard to reuse in CI.

depguard centralizes this work as a deterministic, offline policy engine with stable, machine-readable receipts.

## When to use depguard
- You need the same dependency rules in local dev, CI, and audit pipelines.
- You want deterministic outputs for golden-file tests or enforcement gates.
- You want policy behavior that is easy to explain, trace, and upgrade without surprises.

## How it works (system summary)
1. `depguard-cli` collects manifests and runtime inputs.
2. `depguard-repo` and `depguard-repo-parser` build an in-memory workspace model.
3. `depguard-settings` resolves configuration and effective policy.
4. `depguard-domain` evaluates checks and produces findings.
5. `depguard-app` orchestrates outputs.
6. `depguard-render` emits Markdown, annotations, SARIF, JUnit, JSONL, and report JSON.

This architecture keeps the domain model pure and deterministic, while adapters own I/O.

## Quick start

### Install
```bash
cargo install --path crates/depguard-cli
# Optional: as Cargo subcommand
cargo install --path crates/depguard-cli --bin cargo-depguard
```

### Run a first scan
```bash
depguard check
```

### Common CI pattern
```bash
depguard check --scope diff --base origin/main --head HEAD
```

### Render existing reports
```bash
depguard md --report artifacts/depguard/report.json

depguard annotations --report artifacts/depguard/report.json

depguard sarif --report artifacts/depguard/report.json
```

## Reference (commands by intent)

### Policy execution
- `depguard check` — analyze manifests and write a receipt
- `depguard baseline` — generate baseline suppressions
- `depguard explain <check_id|code>` — show remediation guidance

### Output conversion
- `depguard md --report <path>` — Markdown comment block
- `depguard annotations --report <path>` — GitHub annotations
- `depguard sarif --report <path>` — SARIF JSON
- `depguard junit --report <path>` — JUnit XML
- `depguard jsonl --report <path>` — JSONL stream

### Fixing
- `depguard fix --report <path>` — generate conservative fix plan
- `depguard fix --report <path> --apply` — apply safe fixes

### Runner options
- `cargo depguard` — Cargo subcommand wrapper
- `--scope repo|diff` — scan all manifests or changed scope only
- `--diff-file <path>` — avoid requiring Git in restricted runners
- `--out-dir`, `--report-version`, `--baseline` for output and policy behavior

## Inputs and outputs
By default, `check` writes:
- `artifacts/depguard/report.json`

Optional outputs can be enabled in the same invocation (`--write-markdown`, `--write-junit`, `--write-jsonl`, etc.).

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
- Performing crate resolution or network-dependent checks.
- Replacing `cargo` build tooling.
- Enforcing one-size-fits-all policy defaults.

## Documentation map
- [docs/quickstart.md](docs/quickstart.md) — practical onboarding
- [docs/config.md](docs/config.md) — configuration contract
- [docs/checks.md](docs/checks.md) — check behavior and remediation
- [docs/architecture.md](docs/architecture.md) — deeper design
- [docs/testing.md](docs/testing.md) — test strategy
- [CONTRIBUTING.md](CONTRIBUTING.md) — contribution flow

## Workspace design constraints
- Domain crates have no filesystem/network side effects
- Output is byte-stable for same inputs
- Check IDs and finding codes are stable contracts
- Schema evolution occurs via explicit versioned schema IDs

## License
[MIT](LICENSE-MIT) OR [Apache-2.0](LICENSE-APACHE)
