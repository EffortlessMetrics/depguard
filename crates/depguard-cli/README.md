# depguard-cli

CLI binaries for depguard.

This crate provides the user-facing executable entry points and handles process-level concerns.

## Binaries

- `depguard`
- `cargo-depguard` (Cargo subcommand wrapper)

## Owns

- `clap` argument parsing
- Filesystem reads/writes for config, reports, and artifacts
- Optional git subprocess usage for diff scope (when `--diff-file` is not used)
- Exit code mapping and runtime error handling

## Commands

- `check`
- `baseline`
- `md`
- `annotations`
- `sarif`
- `junit`
- `jsonl`
- `fix`
- `explain`

Business logic is delegated to `depguard-app`.
