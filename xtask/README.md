# xtask

Developer automation commands for the depguard workspace.

This binary crate is for contributor workflows and CI validation, not end-user policy checks.

## Common Commands

### Schema and Fixture Management

- `cargo xtask emit-schemas` — Generate JSON schemas from Rust types
- `cargo xtask validate-schemas` — Validate schemas match generated output
- `cargo xtask fixtures` — Regenerate test fixture golden files
- `cargo xtask print-schema-ids` — Print known schema IDs

### Conformance and Coverage

- `cargo xtask conform` — Validate contract fixtures against sensor.report.v1 schema
- `cargo xtask conform-full` — Full conformance: fixtures + depguard output validation
- `cargo xtask explain-coverage` — Validate all check IDs and codes have explanations

### CI Smoke Script Generation

- `cargo xtask generate-smoke` — Generate CI smoke test scripts (bash and PowerShell)
- `cargo xtask generate-smoke --format=github` — Generate GitHub Actions workflow snippet

Generated scripts verify:
- Binary exists and is executable
- `--help` runs successfully
- `--version` outputs version info
- `check` command runs on a minimal fixture
- `explain` command works

### Release Automation

- `cargo xtask release-prepare` — Prepare release: validate state, update changelog, bump version
- `cargo xtask release-prepare --dry-run` — Preview release changes without making them
- `cargo xtask release-prepare 1.2.3` — Prepare specific version
- `cargo xtask release-artifacts` — Build release artifacts for current target
- `cargo xtask release-check` — Run pre-release validation checks

Release prepare options:
- `--dry-run` — Show what would be done without making changes
- `--skip-changelog` — Skip changelog updates
- `--build` — Build artifacts after preparation
- `--version=X.Y.Z` or positional `X.Y.Z` — Target version

## Scope

- Schema generation from Rust types
- Fixture regeneration/validation workflows
- Contract and conformance checks
- CI smoke test script generation
- Release packaging and automation

This crate is `publish = false`.
