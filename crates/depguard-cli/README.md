# depguard-cli

CLI binaries for depguard.

This crate provides the user-facing executable entry points and handles process-level concerns. It is the primary interface for running depguard from the command line or CI/CD pipelines.

## Purpose

The CLI crate is responsible for:
- Command-line argument parsing with `clap`
- Filesystem I/O for config, reports, and artifacts
- Process exit code mapping
- Delegating business logic to `depguard-app`

## Binaries

### `depguard`

The main executable:

```bash
depguard check                    # Analyze manifests, emit receipt
depguard baseline                 # Generate baseline suppressions
depguard md --report <path>       # Render Markdown from receipt
depguard annotations --report <path>  # Render GitHub annotations
depguard sarif --report <path>    # Render SARIF from receipt
depguard junit --report <path>    # Render JUnit XML from receipt
depguard jsonl --report <path>    # Render JSON Lines from receipt
depguard fix --report <path>      # Apply safe fixes to manifests
depguard explain <check_id|code>  # Show remediation guidance
```

### `cargo-depguard`

A Cargo subcommand wrapper for convenient usage:

```bash
cargo depguard check
cargo depguard baseline
# ... same commands as above
```

## Commands

| Command | Description |
|---------|-------------|
| `check` | Analyze workspace manifests and emit a report |
| `baseline` | Generate baseline suppressions from current findings |
| `md` | Render report as Markdown |
| `annotations` | Render report as GitHub Actions annotations |
| `sarif` | Render report as SARIF for security tools |
| `junit` | Render report as JUnit XML for test runners |
| `jsonl` | Render report as JSON Lines |
| `fix` | Apply safe automated fixes to manifests |
| `explain` | Show remediation guidance for a check or code |

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Pass - no policy violations |
| 2 | Policy failure - errors or warnings exceeded threshold |
| 1 | Tool/runtime error - configuration or execution error |

## Usage Examples

### Basic Check

```bash
# Run with default settings
depguard check

# Run with a specific config file
depguard check --config depguard.toml

# Run with diff scope (only changed files)
depguard check --scope diff --diff-file changed.txt
```

### Generate and Use Baseline

```bash
# Generate baseline from current findings
depguard baseline --output .depguard-baseline.json

# Run with baseline suppressions
depguard check --baseline .depguard-baseline.json
```

### Output Rendering

```bash
# Generate Markdown report
depguard check --output report.json
depguard md --report report.json --output README.md

# Generate GitHub Actions annotations
depguard annotations --report report.json

# Generate SARIF for GitHub code scanning
depguard sarif --report report.json --output results.sarif.json
```

### Explain Checks

```bash
# Get help for a specific check
depguard explain deps.no_wildcards

# Get help for a specific code
depguard explain wildcard_version
```

## Design Constraints

- CLI parsing belongs here; business logic stays in `depguard-app`
- Exit codes must be stable for CI integration
- All I/O (file reads/writes) happens at this layer
- Git subprocess usage for diff scope (when `--diff-file` is not used)

## Dependencies

| Crate | Purpose |
|-------|---------|
| `depguard-app` | Use case orchestration |
| `depguard-domain` | Domain types |
| `depguard-repo` | Repository access |
| `depguard-render` | Output rendering |
| `depguard-settings` | Configuration |
| `depguard-types` | Shared types |
| `depguard-yanked` | Yanked version index |
| `clap` | Argument parsing |
| `anyhow` | Error handling |
| `camino` | UTF-8 paths |

## Feature Flags

All check features are propagated through from app and settings:
- `check-no-wildcards`
- `check-path-requires-version`
- `check-path-safety`
- `check-workspace-inheritance`
- `check-git-requires-version`
- `check-dev-only-in-normal`
- `check-default-features-explicit`
- `check-no-multiple-versions`
- `check-optional-unused`
- `check-yanked-versions`

## Related Crates

- [`depguard-app`](../depguard-app/) - Use case orchestration
- [`depguard-settings`](../depguard-settings/) - Configuration parsing
- [`depguard-render`](../depguard-render/) - Output format rendering
