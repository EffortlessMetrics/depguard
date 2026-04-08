# CLAUDE.md — depguard-cli

## Purpose

Entry point binary. Handles argument parsing, filesystem I/O, git subprocess calls, and exit code mapping.

## Binaries

| Binary | Entry Point | Purpose |
|--------|-------------|---------|
| `depguard` | `src/main.rs` | Main CLI |
| `cargo-depguard` | `src/cargo_depguard.rs` | Cargo subcommand wrapper |

## CLI Structure

```
depguard [OPTIONS] <COMMAND>

Options:
  --repo-root <PATH>     Repository root (default: current directory)
  --config <PATH>        Config file path (default: depguard.toml)
  --profile <NAME>       Override profile (strict|warn|compat)
  --scope <SCOPE>        Override scope (repo|diff)
  --max-findings <N>     Maximum findings to report

Commands:
  check        Analyze manifests and emit receipt
  baseline     Generate baseline suppressions from current findings
  md           Render Markdown from JSON report
  annotations  Render GitHub Actions annotations
  sarif        Render SARIF from JSON report
  junit        Render JUnit XML from JSON report
  jsonl        Render JSON Lines from JSON report
  fix          Generate buildfix plan and optionally apply safe fixes
  explain      Show remediation guidance for a check or code
```

## Subcommands

### check
```
depguard check [OPTIONS]

Options:
  --out-dir <PATH>         Base output directory for artifacts (default: artifacts/depguard)
  --report-out <PATH>      Write JSON report to file
  --write-markdown         Also write Markdown output
  --markdown-out <PATH>    Markdown output path (default: <out-dir>/comment.md)
  --write-junit            Also write JUnit XML output
  --junit-out <PATH>       JUnit output path (default: <out-dir>/report.junit.xml)
  --write-jsonl            Also write JSON Lines output
  --jsonl-out <PATH>       JSON Lines output path (default: <out-dir>/report.jsonl)
  --base <REF>             Git base ref for diff scope
  --head <REF>             Git head ref for diff scope
  --diff-file <PATH>       Precomputed changed-files list for diff scope
  --yanked-index <PATH>    Offline yanked-version index for deps.yanked_versions
```

### baseline
```
depguard baseline [OPTIONS]

Options:
  --report <PATH>         Input report path
  --output <PATH>         Output baseline file path
```

### md
```
depguard md --report <PATH> [--output <PATH>]
```

### annotations
```
depguard annotations --report <PATH> [--max <N>]
```

### sarif
```
depguard sarif --report <PATH> [--output <PATH>]
```

### junit
```
depguard junit --report <PATH> [--output <PATH>]
```

### jsonl
```
depguard jsonl --report <PATH> [--output <PATH>]
```

### fix
```
depguard fix --report <PATH> [--plan-out <PATH>] [--apply]
```

### explain
```
depguard explain <CHECK_ID|CODE>
```

### cargo subcommand
```
cargo depguard [ARGS...]
```

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Pass (or Warn with fail_on=error) |
| 1 | Tool/runtime error |
| 2 | Policy failure |

## Git Integration

For diff scope, the CLI can either call:
```bash
git diff --name-only <base>..<head>
```

or read changed files from `--diff-file` (including GitHub Actions output formats) without invoking git.

`git` remains the only external process call when `--diff-file` is not used.

## Feature Gates

This crate propagates check feature gates to app and settings:

```toml
check-no-wildcards = [
    "depguard-app/check-no-wildcards",
    "depguard-settings/check-no-wildcards",
]
```

All 10 checks have corresponding features.

## Design Constraints

- **Thin wrapper**: All business logic lives in `depguard-app`
- **Config loading**: Reads `depguard.toml`; missing file is OK (defaults apply)
- **Error messages**: Use anyhow context for user-friendly errors

## Dependencies

| Dependency | Purpose |
|------------|---------|
| `depguard-app` | Use cases |
| `depguard` | Public domain facade and types |
| `depguard-repo` | Repository adapters |
| `depguard-render` | Output formatters |
| `depguard-settings` | Configuration |
| `depguard-types` | DTOs |
| `depguard-yanked` | Yanked version parsing |
| `clap` | Argument parsing |
| `anyhow` | Error handling |
| `camino` | UTF-8 paths |
| `serde_json` | JSON handling |
| `reqwest` | HTTP client (for future features) |

## Testing

```bash
cargo test -p depguard-cli       # Unit tests
cargo test --test conformance    # Conformance tests
cargo test --test fixtures       # Golden file tests
cargo test --test bdd            # BDD scenario tests
cargo test --test help           # Help output tests
```

Integration tests use fixtures in `tests/fixtures/` for golden file testing.
