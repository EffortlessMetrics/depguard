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
  - `schemas/depguard.report.v2.json`
- [x] DTOs aligned to schema (types crate)
- [x] Explain registry skeleton + `depguard explain`
- [x] CLI skeleton with `depguard check` producing a minimal receipt

### CI requirements
- [x] `cargo test` (all)
- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets --all-features`
- [x] Schema validation job (validate a known sample report against schema)

### Tests
- [x] "no manifest" fixture: root without `Cargo.toml` → empty pass receipt
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
- [x] Fixture: workspace with members + exclude + nested target deps
- [x] Proptest: normalization of spec shapes (string vs table vs workspace=true)
- [x] Golden snapshot for discovered manifest ordering

---

## Phase 2 — Checks MVP ✅

Implement one check at a time; each adds fixtures and explain entries.

### 2.1 deps.no_wildcards ✅
- [x] Detect `*` in versions (string + table)
- [x] Fixture cases include target deps

### 2.2 deps.path_requires_version ✅
- [x] Detect `path` with missing `version`
- [x] Implement `ignore_publish_false` option
- [x] Fixture cases include publish=false crate

### 2.3 deps.path_safety ✅
- [x] Detect absolute paths
- [x] Detect lexical escape from workspace root
- [x] Add allowlist globs (config)
- [x] Fixtures include Windows-style paths and `..` chains

### 2.4 deps.workspace_inheritance ✅
- [x] Read `[workspace.dependencies]` keys
- [x] Detect member override without `{ workspace = true }`
- [x] Profile gate defaults (off by default)
- [x] Allowlist for exceptional deps

### Tests
- [x] BDD scenarios for each rule
- [ ] Mutation testing (cargo-mutants) enabled for domain crate
- [x] Determinism tests for stable findings ordering

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
- [x] Golden comment.md snapshots
- [x] Golden annotation stream snapshots

---

## Phase 4 — Profiles + diff scope adoption valve ✅

### Deliverables
- [x] Effective config builder:
  - [x] Profile defaults → overridden by depguard.toml
  - [x] Applied once, passed to domain
- [x] `--scope diff`:
  - [x] Changed-file list between base/head (shell-out to git with fixed args)
  - [x] Scan only changed manifests (still read root for workspace deps)
- [x] Missing/partial inputs policy:
  - [x] Treat shallow clone missing base as tool error with remediation message
  - [ ] Optionally support `--diff-file` later to avoid git dependency

### Tests
- [ ] BDD: diff scope analyzes only modified manifests
- [ ] Fixture: base/head selection works with a known git repo fixture

---

## Phase 5 — Hardening 🔄

### Deliverables
- [x] Fuzz targets (`cargo-fuzz`):
  - [x] TOML parser inputs (never panic)
  - [x] Workspace member discovery inputs (never panic)
- [x] Expanded property tests:
  - [x] Ordering invariants under randomized iteration order
  - [ ] Path normalization invariants
- [ ] Conformance harness integration:
  - [ ] Validate receipt against schemas in CI
  - [ ] Enforce explain coverage for every emitted code

### Release polish
- [ ] Prebuilt binaries (Linux/macOS/Windows) via GitHub Releases
- [x] README quickstart + CI snippet
- [ ] `cargo publish --dry-run` gating (if publishing to crates.io)

---

## Definition of Done (v0.1)

- [x] Emits JSON report conforming to schema
- [x] Stable codes and explain entries for all checks
- [x] Deterministic ordering
- [x] Diff scope mode works
- [x] Renderers exist (md + annotations)
- [x] Golden snapshot tests pass
- [x] Fuzz targets exist and run in scheduled CI
- [ ] Mutation testing runs on domain crate

---

## Phase 6 — CI/CD & Release Automation 🆕

### 6.1 Continuous Integration Workflow
- [ ] `.github/workflows/ci.yml`:
  - [ ] Unit tests: `cargo test --lib`
  - [ ] Integration tests: `cargo test --test '*'`
  - [ ] Format check: `cargo fmt --check`
  - [ ] Clippy: `cargo clippy --all-targets --all-features`
  - [ ] Schema validation: `cargo xtask validate-schemas`
- [ ] Mutation testing job (scheduled):
  - [ ] `cargo mutants --package depguard-domain`
  - [ ] Fail if mutation score drops below threshold

### 6.2 Self-Dogfooding Workflow
- [ ] `.github/workflows/depguard.yml`:
  - [ ] Full scan on push to main
  - [ ] Diff-scope scan on PRs (`--scope diff --base origin/${{ github.base_ref }}`)
  - [ ] Generate JSON report + Markdown summary
  - [ ] Create GitHub annotations for findings
  - [ ] Upload artifacts (90-day retention)
  - [ ] Fail on policy violations (exit code 2)

### 6.3 Conformance Workflow
- [ ] `.github/workflows/conformance.yml`:
  - [ ] Run `cargo xtask conform`
  - [ ] Validate receipts against all schema versions
  - [ ] Enforce explain coverage for every emitted code

### 6.4 Release Workflow
- [ ] `.github/workflows/release.yml`:
  - [ ] Trigger on git tag (`v*`) or manual dispatch
  - [ ] Build matrix: Linux (x86_64, aarch64), macOS (x86_64, aarch64), Windows (x86_64)
  - [ ] Create GitHub Release with prebuilt binaries
  - [ ] `cargo publish --dry-run` validation
  - [ ] Optional: publish to crates.io

### Tests
- [ ] Workflow syntax validation via `actionlint`
- [ ] Manual workflow dispatch test for release dry-run

---

## Phase 7 — Additional Checks 🆕

Expand coverage with high-value dependency hygiene checks.

### 7.1 deps.git_requires_version (High Priority)
- [ ] Detect `{ git = "..." }` without `version = "..."`
- [ ] Respect `ignore_publish_false` flag (like path_requires_version)
- [ ] Code: `git_without_version`
- [ ] Explain entry with before/after examples
- [ ] BDD scenarios and golden fixtures

**Justification**: Git deps without versions are non-reproducible and block crates.io publishing.

### 7.2 deps.default_features_explicit (Medium Priority)
- [ ] Flag dependencies with inline options but no explicit `default-features`
- [ ] Suggest adding `default-features = true` or `false`
- [ ] Configurable severity (default: warn)
- [ ] Code: `default_features_implicit`
- [ ] Explain entry and fixtures

**Justification**: Explicit intent improves maintainability and supply chain auditing.

### 7.3 deps.no_multiple_versions (Medium Priority)
- [ ] Track (crate_name, version) pairs across workspace
- [ ] Warn when same crate appears with different versions
- [ ] Allowlist for intentional version splits
- [ ] Code: `duplicate_different_versions`
- [ ] Explain entry and fixtures

**Justification**: Multiple versions bloat binaries and cause subtle interop bugs.

### 7.4 deps.optional_unused (Medium Priority)
- [ ] Parse `[features]` table from manifests
- [ ] Flag `optional = true` deps without corresponding feature
- [ ] Code: `optional_not_in_features`
- [ ] Allowlist for custom feature naming patterns
- [ ] Explain entry and fixtures

**Justification**: Orphaned optional deps are unreachable dead weight.

### 7.5 deps.dev_only_in_normal (Low Priority)
- [ ] Curated list of dev/test crate names (proptest, insta, criterion, etc.)
- [ ] Flag if they appear in `[dependencies]` section
- [ ] Code: `dev_dep_in_normal`
- [ ] Configurable via allowlist
- [ ] Explain entry and fixtures

**Justification**: Reduces transitive deps for consumers; catches copy-paste errors.

### 7.6 deps.yanked_versions (Future, Optional)
- [ ] Accept `--yanked-index` file (pre-computed offline list)
- [ ] Flag pinned versions that are yanked
- [ ] Code: `version_yanked`
- [ ] Optional network mode for live crates.io lookup

**Justification**: Yanked versions signal bugs/security issues; blocks publishing.

### Architecture notes
- All checks follow existing pattern: pure functions in `depguard-domain/src/checks/`
- IDs and codes in `depguard-types/src/ids.rs`
- Explanations in `depguard-types/src/explain.rs`
- Parser extensions needed for 7.3 (workspace tracking) and 7.4 (features table)

---

## Phase 8 — Future Enhancements 🆕

### 8.1 Enhanced Diff Scope
- [ ] `--diff-file` option to accept pre-computed file list
- [ ] Avoid git dependency for containerized/sandboxed environments
- [ ] Support GitHub Actions changed-files action output format

### 8.2 Suppression & Baseline
- [ ] Inline suppression comments: `# depguard: allow(no_wildcards)`
- [ ] Baseline file: ignore known violations during migration
- [ ] `depguard baseline` command to generate suppression file
- [ ] Gradual adoption: only fail on new violations

### 8.3 Fix Suggestions
- [ ] Machine-readable fix suggestions in report
- [ ] `depguard fix` command for auto-remediation
- [ ] Integration with buildfix.plan.v1 schema
- [ ] Conservative: only safe, unambiguous fixes

### 8.4 Extended Outputs
- [ ] SARIF output for GitHub Advanced Security
- [ ] JUnit XML for legacy CI systems
- [ ] JSON Lines streaming for large workspaces

### 8.5 Performance & Scale
- [ ] Parallel manifest parsing for large workspaces
- [ ] Incremental mode: cache parsed manifests
- [ ] Memory-efficient streaming for 1000+ crate workspaces

### 8.6 Ecosystem Integration
- [ ] VS Code extension for inline diagnostics
- [ ] Pre-commit hook integration
- [ ] Cargo subcommand: `cargo depguard`

---

## Milestone Summary

| Milestone | Target | Key Deliverables |
|-----------|--------|------------------|
| v0.1 | Phase 5 complete | Mutation testing, conformance harness |
| v0.2 | Phase 6 complete | CI/CD workflows, prebuilt binaries |
| v0.3 | Phase 7.1-7.2 | git_requires_version, default_features_explicit |
| v0.4 | Phase 7.3-7.5 | Workspace-level checks, features parsing |
| v1.0 | Phase 8.1-8.2 | Suppression, baseline, production-ready |
| v1.1+ | Phase 8.3-8.6 | Fix suggestions, ecosystem integration |

---

## See also

- [Architecture](architecture.md) — System design
- [Testing](testing.md) — Test strategy details
- [Microcrates](microcrates.md) — Crate boundaries
- [Design Notes](design.md) — Key decisions
