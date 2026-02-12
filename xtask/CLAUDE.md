# CLAUDE.md — xtask

## Purpose

Developer automation tasks. Kept separate from the main CLI to avoid bloating the user-facing binary.

## Available Tasks

```bash
cargo xtask <COMMAND>

Commands:
  emit-schemas      Generate JSON schemas to schemas/ directory
  validate-schemas  Check schemas/ against generated output
  print-schema-ids  Print known schema IDs
```

## Schema Generation

Generates schemas from Rust types using `schemars`:

| Schema | Source |
|--------|--------|
| `depguard.report.v1.json` | `DepguardReportV1` from depguard-types |
| `depguard.report.v2.json` | `DepguardReportV2` from depguard-types |
| `depguard.config.v1.json` | `DepguardConfigV1` from depguard-settings |

Note: `receipt.envelope.v1.json` is **vendored** (external contract, not generated).

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
cargo xtask emit-schemas

# Validate schema files
cargo xtask validate-schemas
```
