# Microcrates

> **Navigation**: [Architecture](architecture.md) | [Design](design.md) | Microcrates | [Testing](testing.md) | [Contributing](../CONTRIBUTING.md)

This document is the **contract map**: what each crate owns, what it is allowed to depend on, and what its public API looks like.

The goal is not microcrates for their own sake; the goal is to keep:
- the **policy engine** easy to test and reason about
- the **receipt format** stable
- the **IO surface** small and replaceable

---

## `depguard-types`

**Owns**
- Receipt/envelope DTOs (`ReportEnvelope`, `Finding`, `Verdict`, `Severity`, `Location`)
- Canonical repo path type (`RepoPath`)
- Stable check IDs and codes constants in `ids` module (string literals only; no policy defaults)
- Explanation registry (`Explanation`, `lookup_explanation()`)
- Depguard-specific data summary (`DepguardData`)

**Does not own**
- Policy logic
- Config merging
- Filesystem access

**Public API**
```rust
// Receipt types
pub struct ReportEnvelope<TData> { schema, tool, started_at, finished_at, verdict, findings, data }
pub type DepguardReport = ReportEnvelope<DepguardData>;
pub struct Finding { severity, check_id, code, message, location, help, url, fingerprint, data }
pub enum Severity { Info, Warning, Error }
pub enum Verdict { Pass, Warn, Fail }
pub struct Location { path, line, col }
pub struct DepguardData { scope, profile, manifests_scanned, dependencies_scanned, ... }

// Stable IDs
pub mod ids { pub const DEPS_NO_WILDCARDS: &str = "deps.no_wildcards"; ... }

// Explanations
pub fn lookup_explanation(identifier: &str) -> Option<&'static Explanation>
pub fn all_check_ids() -> impl Iterator<Item = &'static str>
pub fn all_codes() -> impl Iterator<Item = &'static str>

// Paths
pub struct RepoPath(Utf8PathBuf);
```

**Tests**
- Serde roundtrip + schema conformance tests
- Explanation coverage (all IDs and codes have entries)

---

## `depguard-settings`

**Owns**
- Config model (`DepguardConfigV1`, `CheckConfig`)
- Preset profiles (`strict`, `warn`, `compat`)
- Merge/override rules (file config + CLI overrides → `EffectiveConfig`)

**Does not own**
- Domain model types (imports from `depguard-domain`)
- Filesystem I/O (takes strings)

`depguard-settings` exposes the same `check-*` feature gates as `depguard-domain` so default preset generation tracks check availability consistently across crates.

**Public API**
```rust
// Parse TOML config (no I/O)
pub fn parse_config_toml(input: &str) -> Result<DepguardConfigV1>

// Resolve final config
pub fn resolve_config(cfg: Option<DepguardConfigV1>, overrides: Overrides) -> Result<ResolvedConfig>

// Presets
pub fn preset(profile: &str) -> EffectiveConfig  // "strict", "warn", "compat"
```

**Tests**
- Table-driven merge tests
- Profile precedence validation
- Validation error messages

---

## `depguard-yanked`

**Owns**
- Offline yanked-version index model (`YankedIndex`)
- Parsing yanked index text formats (JSON and line-based)
- Query API for exact crate/version lookup

**Does not own**
- File I/O (input is text)
- Policy evaluation

**Public API**
```rust
pub struct YankedIndex { ... }
pub fn parse_yanked_index(input: &str) -> Result<YankedIndex>
impl YankedIndex {
    pub fn is_yanked(&self, crate_name: &str, version: &str) -> bool
}
```

**Tests**
- Format parsing tests (JSON map, JSON array, line format)
- Validation/error message tests

---

## `depguard-check-catalog`

**Owns**
- Check catalog metadata and default policy matrix
- Check feature gates (`check-*`)
- Canonical profile-to-check mapping (`checks_for_profile`)
- API for feature-aware catalog lookups (`is_check_available`, `feature_name`)
- BDD feature coverage mapping (`bdd_feature_file`)

**Does not own**
- Rule implementations or check execution
- Runtime policy defaults in profiles

Feature availability and defaults for checks are intended to be the single source of truth for:
- `depguard-settings` preset generation
- `depguard-domain` runtime check gating
- `depguard-app` feature passthrough

**Public API**
```rust
pub struct CheckCatalogEntry {
    pub id: &'static str,
    pub codes: &'static [&'static str],
    pub strict_enabled: bool,
    pub strict_severity: Severity,
    pub warn_enabled: bool,
    pub warn_severity: Severity,
    pub feature: CheckFeature,
    pub bdd_feature_file: &'static str,
}

pub struct ProfileCheck {
    pub id: &'static str,
    pub enabled: bool,
    pub severity: Severity,
}

pub fn catalog() -> &'static [CheckCatalogEntry]
pub fn checks_for_profile(profile: &str) -> Vec<ProfileCheck>
pub fn is_check_available(check_id: &str) -> bool
pub fn feature_name(check_id: &str) -> Option<&'static str>
pub fn bdd_feature_file(check_id: &str) -> Option<&'static str>
pub fn all_check_ids() -> Vec<&'static str>
pub fn all_codes() -> Vec<&'static str>
```

**Tests**
- `catalog` integrity tests (all checks mapped to known IDs and explanations)

---

## `depguard-domain-core`

**Owns**
- Domain model (`WorkspaceModel`, `ManifestModel`, `DependencyDecl`, `DepSpec`, `WorkspaceDependency`, `PackageMeta`, `DepKind`)
- Policy types (`EffectiveConfig`, `CheckPolicy`, `Scope`, `FailOn`)

**Does not own**
- Check implementations
- Evaluation orchestration
- TOML parsing or filesystem access

**Public API**
```rust
// Model types
pub struct WorkspaceModel { repo_root, workspace_dependencies, manifests }
pub struct ManifestModel { path, package, dependencies, features }
pub struct DependencyDecl { kind, name, spec, location, target }
pub struct DepSpec { version, path, workspace }
pub enum DepKind { Normal, Dev, Build }

// Policy
pub struct EffectiveConfig { profile, scope, fail_on, max_findings, yanked_index, checks }
pub struct CheckPolicy { enabled, severity, allow, ignore_publish_false }
pub enum Scope { Repo, Diff }
pub enum FailOn { Error, Warning }
```

**Tests**
- Serde roundtrip tests for model types

---

## `depguard-domain-checks`

**Owns**
- All 10 check implementations (one module per check in `checks/`)
- `run_all()` orchestration: iterates enabled checks, collects findings
- Check utilities (`section_name()`, `spec_to_json()`)
- Finding fingerprinting

**Does not own**
- Domain model types (imports from `depguard-domain-core`)
- Policy evaluation / verdict computation
- TOML parsing or filesystem access

**Public API**
```rust
pub fn run_all(model: &WorkspaceModel, cfg: &EffectiveConfig) -> Vec<Finding>
```

**Tests**
- Unit tests per check (table-driven via `test_support` helpers)
- Mutation testing (`cargo mutants --package depguard-domain-checks`)

---

## `depguard-domain`

**Facade crate** — re-exports model/policy from `depguard-domain-core` and delegates check execution to `depguard-domain-checks`. Owns the evaluation engine that wraps check results into a `DomainReport`.

**Owns**
- `evaluate()` engine: calls `run_all()`, sorts findings, truncates, computes verdict
- `DomainReport` struct
- Deterministic ordering + truncation behavior
- Re-exports of model and policy types for downstream consumers

**Does not own**
- Individual check implementations (in `depguard-domain-checks`)
- Model/policy type definitions (in `depguard-domain-core`)
- TOML parsing, filesystem access, git operations

**Public API**
```rust
// Evaluation
pub fn evaluate(model: &WorkspaceModel, cfg: &EffectiveConfig) -> DomainReport

// Re-exported from depguard-domain-core:
pub mod model;   // WorkspaceModel, ManifestModel, DependencyDecl, DepSpec, ...
pub mod policy;  // EffectiveConfig, CheckPolicy, Scope, FailOn
```

**Critical constraint**: No IO; no TOML; no git. If a check needs file context beyond what is in the model, the model must be extended explicitly.

**Tests**
- Property tests: determinism, stable sorting, no panics on arbitrary inputs
- Engine integration tests

---

## `depguard-repo`

**Owns**
- Workspace discovery (`Cargo.toml` workspace members/excludes, glob expansion)
- Reading files from disk
- Parsing manifests (delegated to `depguard-repo-parser`)
- Diff scoping (changed file list → manifest set)
- Fuzz-safe parsing APIs

**Does not own**
- Git subprocess calls (that's CLI's job)
- Policy evaluation

**Public API**
```rust
// Discovery
pub fn discover_manifests(repo_root: &Utf8Path) -> Result<Vec<RepoPath>>

// Model building
pub fn build_workspace_model(repo_root: &Utf8Path, scope: ScopeInput) -> Result<WorkspaceModel>
pub fn build_workspace_model_with_cache(repo_root: &Utf8Path, scope: ScopeInput, cache_dir: Option<&Utf8Path>) -> Result<WorkspaceModel>

// Fuzz-safe APIs (never panic)
pub mod fuzz {
    pub fn parse_root_manifest(text: &str) -> anyhow::Result<()>
    pub fn parse_member_manifest(text: &str) -> anyhow::Result<()>
}
```

**Tests**
- Fixture-driven tests with tiny workspaces
- Fuzzing targets for TOML parsing and workspace member expansion

---

## `depguard-repo-parser`

**Owns**
- Pure TOML manifest parsing for root and member `Cargo.toml` files.
- Dependency model extraction (`DepSpec`, `DependencyDecl`, `ManifestModel`).
- Inline suppression parsing and feature extraction.

**Does not own**
- Filesystem access
- Workspace discovery
- Diff filtering

**Public API**
```rust
pub fn parse_root_manifest(path: &RepoPath, text: &str) -> Result<(BTreeMap<String, WorkspaceDependency>, ManifestModel)>
pub fn parse_member_manifest(path: &RepoPath, text: &str) -> Result<ManifestModel>
```

**Tests**
- Unit tests for parser coverage
- Fuzz-style property checks via `depguard-repo::fuzz` entry points

---

## `depguard-render`

**Owns**
- Renderers from report → text formats:
  - Markdown (PR comment)
  - GitHub Actions annotations
  - SARIF (`sarif v2.1.0`)
  - JUnit XML
  - JSON Lines (streaming)

**Does not own**
- Report serialization (that's app layer)
- File I/O

**Public API**
```rust
pub fn render_markdown(report: &DepguardReport) -> String
pub fn render_github_annotations(report: &DepguardReport) -> Vec<String>
pub fn render_sarif(report: &DepguardReport) -> String
pub fn render_junit(report: &DepguardReport) -> String
pub fn render_jsonl(report: &DepguardReport) -> String
```

**Tests**
- Golden snapshot tests for Markdown
- Property tests: output is stable under re-rendering

---

## `depguard-app`

**Owns**
- Use case orchestration (check, md, annotations, explain)
- Report serialization to JSON
- Baseline parsing/generation/suppression transforms
- Verdict → exit code mapping

Uses `depguard-settings` and `depguard-domain` and threads the shared `check-*` feature gates through both so config defaults and runtime execution stay aligned.

**Does not own**
- CLI argument parsing (that's `depguard-cli`)
- Direct filesystem I/O

**Public API**
```rust
// Use cases
pub fn run_check(input: CheckInput) -> Result<CheckOutput>
pub fn render_markdown(report: &DepguardReport) -> String
pub fn render_annotations(report: &DepguardReport) -> Vec<String>
pub fn render_junit(report: &DepguardReport) -> String
pub fn render_jsonl(report: &DepguardReport) -> String
pub fn run_explain(identifier: &str) -> Option<Explanation>
pub fn generate_baseline(report: &ReportVariant) -> DepguardBaselineV1
pub fn apply_baseline(report: &mut ReportVariant, baseline: &DepguardBaselineV1, fail_on: FailOn)

// Serialization
pub fn serialize_report(report: &DepguardReport) -> Result<String>
pub fn serialize_baseline(baseline: &DepguardBaselineV1) -> Result<String>

// Exit codes
pub fn verdict_exit_code(verdict: Verdict) -> i32  // 0=pass, 2=fail
```

**Tests**
- Integration tests for use case workflows

---

## `depguard-cli`

**Owns**
- clap CLI definitions
- Wiring: settings + repo + domain + render
- Artifact write layout + exit codes
- Git subprocess calls for diff scope

Feature flags for checks are passed through `depguard-app` so CLI and dependency graph share a single check metadata source from `depguard-check-catalog`.

**Does not own**
- Business logic (delegates to app/domain/settings/repo)

**Commands**
```
depguard check [--out-dir PATH] [--report-out PATH] [--write-markdown] [--write-junit] [--write-jsonl] [--base REF] [--head REF] [--diff-file PATH]
depguard check [--yanked-index PATH]
depguard baseline [--output PATH] [--base REF] [--head REF] [--diff-file PATH]
depguard baseline [--yanked-index PATH]
depguard md --report PATH [--output PATH]
depguard annotations --report PATH [--max N]
depguard sarif --report PATH [--output PATH]
depguard junit --report PATH [--output PATH]
depguard jsonl --report PATH [--output PATH]
depguard explain <CHECK_ID|CODE>
cargo depguard <ARGS...>
```

**Exit codes**: 0 = pass/warn, 1 = tool error, 2 = policy failure

**Tests**
- `assert_cmd` integration tests
- End-to-end fixtures in `tests/fixtures/`

---

## `depguard-test-util`

**Owns**
- Shared test-only helpers reused across workspace crates
- Normalization of non-deterministic report fields for golden-file comparisons

**Does not own**
- Production policy logic
- CLI/runtime behavior

**Public API**
```rust
pub fn normalize_nondeterministic(value: serde_json::Value) -> serde_json::Value
```

**Notes**
- `publish = false`
- Used by test suites and `xtask` fixture tooling

---

## `xtask`

**Owns**
- Schema generation (via `schemars`)
- Fixture updates
- Release packaging
- Developer loops that should not be in the CLI

**Commands**
```bash
cargo xtask emit-schemas   # Generate JSON schemas
cargo xtask fixtures       # Regenerate test fixtures
cargo xtask conform-full   # Full protocol conformance checks
```

## See also

- [Architecture](architecture.md) — Data flow and crate relationships
- [Design Notes](design.md) — Why microcrates, domain purity
- [Testing](testing.md) — Per-crate test strategies
- [Contributing](../CONTRIBUTING.md) — How to add new crates or checks
