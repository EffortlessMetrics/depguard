# CLAUDE.md — xtask

## Purpose

Developer automation tasks. Kept separate from the main CLI to avoid bloating the user-facing binary.

## Available Tasks

```bash
cargo xtask <COMMAND>

Commands:
  schemas     Generate JSON schemas to schemas/ directory
  fixtures    Regenerate test fixtures
  release     Prepare release artifacts
```

## Schema Generation

Generates schemas from Rust types using `schemars`:

| Schema | Source |
|--------|--------|
| `depguard.report.v1.json` | `DepguardReport` from depguard-types |
| `depguard.config.v1.json` | `DepguardConfigV1` from depguard-settings |

Note: `receipt.envelope.v1.json` is **vendored** (external contract, not generated).

## Fixture Generation

Regenerates golden test files in `tests/fixtures/`:
- `report.json` — Expected JSON output
- `comment.md` — Expected Markdown output

Run after changing output format to update expectations.

## Design Constraints

- **No user-facing functionality**: xtask is for developers only
- **Deterministic output**: Generated schemas must be byte-stable
- **Schema versioning**: Never modify existing schema files; create new versions

## Dependencies

- `depguard-types` — Report schema source
- `depguard-settings` — Config schema source
- `schemars` — JSON schema generation
- `serde_json` — Pretty-printing schemas
- `anyhow` — Error handling

## Usage

```bash
# Generate all schemas
cargo xtask schemas

# Regenerate test fixtures
cargo xtask fixtures

# Prepare release
cargo xtask release
```
