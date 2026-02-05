# depguard

**Repo-truth dependency manifest hygiene sensor for Rust workspaces**

Depguard inspects `Cargo.toml` manifests and evaluates them against explicit policies, emitting versioned reports suitable for CI pipelines, PR comments, and audit trails.

## Features

- **Offline & fast** — No network access, no cargo builds, no metadata resolution
- **Deterministic** — Same inputs produce byte-identical outputs; CI diffs stay clean
- **Schema-first** — Versioned JSON schemas define the contract; tooling can rely on stable output
- **CI-native** — GitHub Actions annotations, Markdown PR comments, configurable exit codes
- **Gradual adoption** — Profiles (`strict`/`warn`/`compat`) and diff-scope let you roll out incrementally

## Installation

```bash
# From source
cargo install --path crates/depguard-cli

# Or build locally
cargo build --release
```

## Quick start

```bash
# Analyze all manifests in the workspace
depguard check

# Analyze only manifests changed since main
depguard check --scope diff --base origin/main

# Generate a Markdown report from existing JSON
depguard md --report artifacts/depguard/report.json

# Get help for a specific check or code
depguard explain deps.no_wildcards
depguard explain wildcard_version
```

By default, reports are written to `artifacts/depguard/report.json`. If you pass `--write-markdown`, Markdown is written to `artifacts/depguard/comment.md`.

## Example output

```json
{
  "schema": "depguard.report.v2",
  "tool": { "name": "depguard", "version": "0.1.0" },
  "run": { "started_at": "...", "ended_at": "...", "duration_ms": 12 },
  "verdict": { "status": "fail", "counts": { "info": 0, "warn": 0, "error": 1 }, "reasons": [] },
  "findings": [
    {
      "severity": "error",
      "check_id": "deps.no_wildcards",
      "code": "wildcard_version",
      "message": "Wildcard version '*' is not allowed",
      "location": { "path": "crates/foo/Cargo.toml", "line": 12 }
    }
  ]
}
```

To emit the legacy v1 schema, use `depguard check --report-version v1`.

## Configuration

Create `depguard.toml` in your repo root (optional—defaults to `strict` profile):

```toml
profile = "strict"        # strict | warn | compat
scope = "repo"            # repo | diff
fail_on = "error"         # error | warning
max_findings = 100

[checks."deps.no_wildcards"]
enabled = true
severity = "error"

[checks."deps.path_requires_version"]
enabled = true
allow = ["internal-*"]  # Glob patterns; case-sensitive
ignore_publish_false = true
```

See [docs/config.md](docs/config.md) for the full configuration reference.

## Checks

| Check ID | Description |
|----------|-------------|
| `deps.no_wildcards` | Detect wildcard versions (`*`, `1.*`) |
| `deps.path_requires_version` | Require version alongside path dependencies |
| `deps.path_safety` | Prevent absolute paths and workspace escapes |
| `deps.workspace_inheritance` | Enforce `workspace = true` for shared deps (disabled by default) |

See [docs/checks.md](docs/checks.md) for detailed documentation, examples, and remediation guidance.

## CI integration

### GitHub Actions

```yaml
- name: Run depguard
  run: |
    depguard check --scope diff --base origin/${{ github.base_ref }}

- name: Comment on PR
  if: failure()
  run: |
    depguard md --report artifacts/depguard/report.json >> $GITHUB_STEP_SUMMARY
```

See [docs/ci-integration.md](docs/ci-integration.md) for complete CI setup examples.

## Exit codes

| Code | Meaning |
|------|---------|
| `0` | Pass — no policy violations |
| `1` | Tool error — invalid config, missing files, git issues |
| `2` | Policy failure — findings exceed `fail_on` threshold |

## Architecture

Depguard uses hexagonal (ports & adapters) architecture with a pure evaluation core:

```
crates/
  depguard-types     # Stable DTOs, schema IDs, finding codes
  depguard-settings  # Config parsing, profile presets
  depguard-domain    # Pure policy evaluation (no I/O)
  depguard-repo      # Workspace discovery, TOML parsing
  depguard-render    # Markdown and annotation renderers
  depguard-app       # Use case orchestration
  depguard-cli       # CLI binary
xtask/               # Dev tooling
schemas/             # Versioned JSON schemas
```

See [docs/architecture.md](docs/architecture.md) for the full design.

## Documentation

| Document | Description |
|----------|-------------|
| [Quick Start](docs/quickstart.md) | Get up and running in 5 minutes |
| [Configuration](docs/config.md) | Full config file reference |
| [Checks Catalog](docs/checks.md) | All checks with examples and remediation |
| [CI Integration](docs/ci-integration.md) | GitHub Actions, GitLab CI setup |
| [Architecture](docs/architecture.md) | System design and data flow |
| [Testing](docs/testing.md) | Test strategy and commands |
| [Contributing](CONTRIBUTING.md) | How to contribute |

## Design principles

- **Domain has no I/O** — `depguard-domain` takes an in-memory model and returns findings
- **Adapters are swappable** — Filesystem/git operations isolated in `depguard-repo`
- **DTOs are stable** — Receipt types versioned with schema IDs
- **Deterministic output** — Sorting and capping rules are explicit

## License

[MIT](LICENSE) OR [Apache-2.0](LICENSE-APACHE)

