# depguard-cli

## Problem
Users need a stable command interface for analysis, rendering, and fixes, while keeping policy evaluation testable and independent.

## What this crate does
`depguard-cli` is the public command surface for depguard, mapping arguments and runtime options into application use cases.

## Commands
- `check` — run policy scans
- `baseline` — create suppressions from current findings
- `md` / `annotations` / `sarif` / `junit` / `jsonl` — render existing report
- `fix` — generate/apply conservative fixes
- `explain` — show remediation guidance

## Operational model
1. Parse CLI arguments (`clap`)
2. Resolve runtime config and paths
3. Delegate to `depguard-app`
4. Write outputs and emit deterministic exit code

## Exit behavior
- `0` pass
- `1` runtime/tooling error
- `2` policy failure threshold met

## Why this layer exists
It keeps command parsing and process behavior in one place, instead of leaking into domain logic.

## Related crates
- `depguard-app`
- `depguard-repo`
- `depguard-render`
- `depguard-settings`
- `depguard-types`
