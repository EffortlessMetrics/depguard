# xtask

## Problem
Contributor workflows (schema generation, fixture maintenance, conformance checks, packaging) are repetitive and error-prone when run manually.

## What this crate does
`xtask` is the workspace automation crate for depguard maintainers.

## Task families
### Schema and contracts
- Emit and validate JSON schemas
- Assert canonical schema IDs are stable
- Regenerate fixtures from current behavior

### Validation
- Ensure fixture and runtime outputs conform to schema contracts
- Verify explain registry and check-code coverage

### Release and CI automation
- Produce smoke scripts
- Prepare release metadata and artifacts
- Validate pre-release checks, including packaging publishable crates

## How to use
```bash
cargo run -p xtask -- emit-schemas
cargo run -p xtask -- fixtures
cargo run -p xtask -- conform-full
cargo run -p xtask -- release-prepare --dry-run
```

## Scope boundary
- This crate is internal (`publish = false`).
- It orchestrates developer tooling, not production checks.

## Related
- `schemas/` (generated and checked artifacts)
- `tests/fixtures/` (golden baseline files)
- `depguard-types`, `depguard-test-util`
