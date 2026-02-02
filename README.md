# depguard — Architecture + Plan Package

This folder is a **copy-ready doc set** for **depguard**, written to match the ecosystem shape:

- receipts-first (`artifacts/<sensor>/report.json`)
- strict, versioned schemas
- deterministic outputs
- hexagonal / clean architecture
- microcrate workspace layout
- test-heavy: BDD + fixtures + proptest + fuzz + mutation testing

## Contents

- `docs/` — requirements, design, architecture, implementation plan, config, checks, testing
- `schemas/` — `receipt.envelope.v1.json` and `depguard.report.v1.json`
- `examples/` — example `depguard.toml`, example `cockpit.toml`, and a GitHub Actions snippet
- `tests/` — starter BDD feature file and fixture skeleton (structure + a tiny sample)

## Notes

- This package is intentionally **schema-first**: the envelope is treated as a stable ABI.
- Tool-specific payload is confined to `data` (report-level) and `finding.data` (finding-level).
- Codes and check IDs are treated as API: never rename; only deprecate with aliases.

Generated: 2026-02-02
