# Implementation Plan

> **Navigation**: [Architecture](architecture.md) | [Design](design.md) | [Microcrates](microcrates.md) | Implementation Plan | [Testing](testing.md)

This plan is sequenced to ship value early while freezing the protocol and preventing drift.

## Guiding sequencing rule

**Freeze contracts early; build features behind them.**

- The receipt schema, codes, ordering, and artifact paths should stabilize before you add "nice to haves."
- Adoption valves (profiles, diff-scope) are part of v0, not v1.

---

## Phase 0 — Protocol + scaffolding ✅

### Deliverables
- [x] Workspace structure with microcrates:
  - `depguard-types`, `depguard-domain`, `depguard-repo`, `depguard-render`, `depguard-app`, `depguard-cli`, `xtask`
- [x] Schemas:
  - `schemas/receipt.envelope.v1.json` (vendored)
  - `schemas/depguard.report.v1.json`
- [x] DTOs aligned to schema (types crate)
- [x] Explain registry skeleton + `depguard explain`
- [x] CLI skeleton with `depguard check` producing a minimal receipt

### CI requirements
- [x] `cargo test` (all)
- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets --all-features`
- [ ] Schema validation job (validate a known sample report against schema)

### Tests
- [ ] "no manifest" fixture: root without `Cargo.toml` → `skip` receipt
- [x] Golden snapshot for `report.json` bytes

---

## Phase 1 — Workspace discovery + parsing ✅

### Deliverables
- [x] Root manifest reader and parser (`toml_edit`)
- [x] Workspace discovery:
  - [x] Parse `[workspace].members` and `exclude`
  - [x] Glob expansion relative to root
  - [x] Find member `Cargo.toml`
  - [x] Stable ordering + dedupe
- [x] Dependency table walker:
  - [x] dependencies/dev/build
  - [x] target.* variants
- [x] Normalized dependency entry model

### Tests
- [ ] Fixture: workspace with members + exclude + nested target deps
- [ ] Proptest: normalization of spec shapes (string vs table vs workspace=true)
- [ ] Golden snapshot for discovered manifest ordering

---

## Phase 2 — Checks MVP ✅

Implement one check at a time; each adds fixtures and explain entries.

### 2.1 deps.no_wildcards ✅
- [x] Detect `*` in versions (string + table)
- [ ] Fixture cases include target deps

### 2.2 deps.path_requires_version ✅
- [x] Detect `path` with missing `version`
- [ ] Implement `ignore_publish_false` option
- [ ] Fixture cases include publish=false crate

### 2.3 deps.path_safety ✅
- [x] Detect absolute paths
- [x] Detect lexical escape from workspace root
- [ ] Add allowlist globs (config)
- [ ] Fixtures include Windows-style paths and `..` chains

### 2.4 deps.workspace_inheritance ✅
- [x] Read `[workspace.dependencies]` keys
- [x] Detect member override without `{ workspace = true }`
- [ ] Profile gate defaults (off in oss)
- [ ] Allowlist for exceptional deps

### Tests
- [ ] BDD scenarios for each rule
- [ ] Mutation testing (cargo-mutants) enabled for domain crate
- [ ] Determinism tests for stable findings ordering

---

## Phase 3 — Renderers and UX ✅

### Deliverables
- [x] Receipt writer to canonical artifact path
- [x] Markdown renderer:
  - [x] Summary + findings
  - [x] Remediation hints
- [x] GitHub annotations renderer:
  - [x] Location-bearing findings only
  - [x] Stable ordering

### Tests
- [ ] Golden comment.md snapshots
- [ ] Golden annotation stream snapshots

---

## Phase 4 — Profiles + diff scope adoption valve ✅

### Deliverables
- [x] Effective config builder:
  - [x] Profile defaults → overridden by depguard.toml
  - [x] Applied once, passed to domain
- [x] `--scope diff`:
  - [x] Changed-file list between base/head (shell-out to git with fixed args)
  - [x] Scan only changed manifests (still read root for workspace deps)
- [ ] Missing/partial inputs policy:
  - [ ] Treat shallow clone missing base as tool error with remediation message
  - [ ] Optionally support `--diff-file` later to avoid git dependency

### Tests
- [ ] BDD: diff scope analyzes only modified manifests
- [ ] Fixture: base/head selection works with a known git repo fixture

---

## Phase 5 — Hardening 🔄

### Deliverables
- [ ] Fuzz targets (`cargo-fuzz`):
  - [ ] TOML parser inputs (never panic)
  - [ ] Workspace member discovery inputs (never panic)
- [ ] Expanded property tests:
  - [ ] Ordering invariants under randomized iteration order
  - [ ] Path normalization invariants
- [ ] Conformance harness integration:
  - [ ] Validate receipt against schemas in CI
  - [ ] Enforce explain coverage for every emitted code

### Release polish
- [ ] Prebuilt binaries (Linux/macOS/Windows) via GitHub Releases
- [ ] README quickstart + CI snippet
- [ ] `cargo publish --dry-run` gating (if publishing to crates.io)

---

## Definition of Done (v0.1)

- [x] Emits JSON report conforming to schema
- [x] Stable codes and explain entries for all checks
- [x] Deterministic ordering
- [x] Diff scope mode works
- [x] Renderers exist (md + annotations)
- [ ] Golden snapshot tests pass
- [ ] Fuzz targets exist and run in scheduled CI
- [ ] Mutation testing runs on domain crate

## See also

- [Architecture](architecture.md) — System design
- [Testing](testing.md) — Test strategy details
- [Microcrates](microcrates.md) — Crate boundaries
- [Design Notes](design.md) — Key decisions
