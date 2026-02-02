# depguard — Implementation Plan

This plan is sequenced to ship value early while freezing the protocol and preventing drift.

## Guiding sequencing rule

**Freeze contracts early; build features behind them.**

- The receipt schema, codes, ordering, and artifact paths should stabilize before you add “nice to haves.”
- Adoption valves (profiles, diff-scope) are part of v0, not v1.

## Phase 0 — Protocol + scaffolding (P0)

### Deliverables
- Workspace structure with microcrates:
  - `depguard-types`, `depguard-domain`, `depguard-repo`, `depguard-render`, `depguard-app`, `depguard-cli`, `xtask`
- Schemas:
  - `schemas/receipt.envelope.v1.json` (vendored)
  - `schemas/depguard.report.v1.json`
- DTOs aligned to schema (types crate)
- Explain registry skeleton + `depguard explain`
- CLI skeleton with `depguard check` producing a minimal receipt (even if empty)

### CI requirements
- `cargo test` (all)
- `cargo fmt --check`
- `cargo clippy --all-targets --all-features`
- schema validation job (validate a known sample report against schema)

### Tests
- “no manifest” fixture: root without `Cargo.toml` → `skip` receipt
- golden snapshot for `report.json` bytes

## Phase 1 — Workspace discovery + parsing (P1)

### Deliverables
- Root manifest reader and parser (`toml_edit`)
- Workspace discovery:
  - parse `[workspace].members` and `exclude`
  - glob expansion relative to root
  - find member `Cargo.toml`
  - stable ordering + dedupe
- Dependency table walker:
  - dependencies/dev/build
  - target.* variants
- Normalized dependency entry model

### Tests
- fixture: workspace with members + exclude + nested target deps
- proptest: normalization of spec shapes (string vs table vs workspace=true)
- golden snapshot for discovered manifest ordering

## Phase 2 — Checks MVP (P2)

Implement one check at a time; each adds fixtures and explain entries.

### 2.1 deps.no_wildcards
- detect `*` in versions (string + table)
- fixture cases include target deps

### 2.2 deps.path_requires_version
- detect `path` with missing `version`
- implement `ignore_publish_false` option
- fixture cases include publish=false crate

### 2.3 deps.path_safety
- detect absolute paths
- detect lexical escape from workspace root
- add allowlist globs (config)
- fixtures include Windows-style paths and `..` chains

### 2.4 deps.workspace_inheritance
- read `[workspace.dependencies]` keys
- detect member override without `{ workspace = true }`
- profile gate defaults (off in oss)
- allowlist for exceptional deps

### Tests
- BDD scenarios for each rule
- mutation testing (cargo-mutants) enabled for domain crate (scheduled if slow)
- determinism tests for stable findings ordering

## Phase 3 — Renderers and UX (P3)

### Deliverables
- Receipt writer to canonical artifact path:
  - default `artifacts/depguard/report.json`
- Markdown renderer:
  - summary + top N findings
  - clear remediation hints
  - link to report artifact
- GitHub annotations renderer:
  - location-bearing findings only
  - cap to `max_annotations`
  - stable ordering

### Tests
- golden comment.md snapshots
- golden annotation stream snapshots

## Phase 4 — Profiles + diff scope adoption valve (P4)

### Deliverables
- Effective config builder:
  - profile defaults → overridden by depguard.toml
  - applied once, passed to domain
- `--scope diff`:
  - changed-file list between base/head (shell-out to git with fixed args)
  - scan only changed manifests (still read root for workspace deps)
- Missing/partial inputs policy:
  - treat shallow clone missing base as tool error with remediation message
  - optionally support `--diff-file` later to avoid git dependency

### Tests
- BDD: diff scope analyzes only modified manifests
- fixture: base/head selection works with a known git repo fixture (or mocked DiffProvider)

## Phase 5 — Hardening (P5)

### Deliverables
- Fuzz targets (`cargo-fuzz`):
  - TOML parser inputs (never panic)
  - workspace member discovery inputs (never panic)
- Expanded property tests:
  - ordering invariants under randomized iteration order
  - path normalization invariants
- Conformance harness integration:
  - validate receipt against schemas in CI
  - enforce explain coverage for every emitted code

### Release polish
- prebuilt binaries (Linux/macOS/Windows) via GitHub Releases
- README quickstart + CI snippet
- `cargo publish --dry-run` gating (if publishing to crates.io)

## Definition of Done (v0.1)

- emits `artifacts/depguard/report.json` conforming to schema
- stable codes and explain entries for all checks
- deterministic ordering and golden snapshot tests
- diff scope mode works
- renderers exist (md + annotations) and are capped
- fuzz targets exist and run at least in scheduled CI
- mutation testing runs (scheduled or required, depending on time budget)
