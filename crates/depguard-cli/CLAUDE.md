# CLAUDE.md — depguard-cli

## Purpose

Entry point binary. Handles argument parsing, filesystem I/O, git subprocess calls, and exit code mapping.

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
  md           Render Markdown from JSON report
  annotations  Render GitHub Actions annotations
  explain      Show remediation guidance for a check or code
```

## Subcommands

### check
```
depguard check [OPTIONS]

Options:
  --report-out <PATH>      Write JSON report to file
  --write-markdown         Also write Markdown output
  --markdown-out <PATH>    Markdown output path (default: stdout)
  --base <REF>             Git base ref for diff scope
  --head <REF>             Git head ref for diff scope
```

### md
```
depguard md --report <PATH> [--output <PATH>]
```

### annotations
```
depguard annotations --report <PATH> [--max <N>]
```

### explain
```
depguard explain <CHECK_ID|CODE>
```

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Pass (or Warn with fail_on=error) |
| 1 | Tool/runtime error |
| 2 | Policy failure |

## Git Integration

For diff scope, the CLI calls:
```bash
git diff --name-only <base>...<head>
```

This is the **only** external process call. Missing git is a runtime error.

## Design Constraints

- **Thin wrapper**: All business logic lives in `depguard-app`
- **Config loading**: Reads `depguard.toml`; missing file is OK (defaults apply)
- **Error messages**: Use anyhow context for user-friendly errors

## Dependencies

- `depguard-app` — Use cases
- `depguard-types`, `depguard-domain`, `depguard-settings` — Types
- `clap` — Argument parsing
- `anyhow` — Error handling
- `camino` — UTF-8 paths

## Testing

```bash
cargo test -p depguard-cli       # Unit tests
cargo test --test '*'            # Integration tests (in tests/)
```

Integration tests use fixtures in `tests/fixtures/` for golden file testing.
