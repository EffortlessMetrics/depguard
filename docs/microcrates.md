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
- Stable check IDs and codes registry (string constants in `ids` module)
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

## `depguard-domain`

**Owns**
- Domain model (`WorkspaceModel`, `ManifestModel`, `DependencyDecl`, `DepSpec`)
- Policy types (`EffectiveConfig`, `CheckPolicy`, `Scope`, `FailOn`)
- Check registry and evaluation engine
- Deterministic ordering + truncation behavior

**Does not own**
- TOML parsing
- Filesystem access
- Git operations

**Public API**
```rust
// Evaluation
pub fn evaluate(model: &WorkspaceModel, cfg: &EffectiveConfig) -> DomainReport

// Model types
pub struct WorkspaceModel { repo_root, workspace_dependencies, manifests }
pub struct ManifestModel { path, package, dependencies }
pub struct DependencyDecl { kind, name, spec, location }
pub struct DepSpec { version, path, workspace }

// Policy
pub struct EffectiveConfig { profile, scope, fail_on, max_findings, checks }
pub enum Scope { Repo, Diff }
pub enum FailOn { Error, Warning }
```

**Critical constraint**: No IO; no TOML; no git. If a check needs file context beyond what is in the model, the model must be extended explicitly.

**Tests**
- Unit tests per check
- Property tests: determinism, stable sorting, no panics on arbitrary inputs
- Mutation testing (`cargo mutants`)

---

## `depguard-repo`

**Owns**
- Workspace discovery (`Cargo.toml` workspace members/excludes, glob expansion)
- Reading files from disk
- Parsing manifests (TOML → domain model with line numbers)
- Diff scoping (changed file list → manifest set)
- Fuzz-safe parsing APIs

**Does not own**
- Git subprocess calls (that's CLI's job)
- Policy evaluation

**Public API**
```rust
// Discovery
pub fn discover_manifests(repo_root: &Utf8Path) -> Result<Vec<RepoPath>>

// Parsing
pub fn parse_root_manifest(path: &RepoPath, text: &str) -> Result<(HashMap<String, DepSpec>, ManifestModel)>
pub fn parse_member_manifest(path: &RepoPath, text: &str) -> Result<ManifestModel>

// Model building
pub fn build_workspace_model(repo_root: &Utf8Path, scope: ScopeInput) -> Result<WorkspaceModel>

// Fuzz-safe APIs (never panic)
pub mod fuzz {
    pub fn parse_root_manifest(text: &str) -> Option<...>
    pub fn parse_member_manifest(text: &str) -> Option<...>
}
```

**Tests**
- Fixture-driven tests with tiny workspaces
- Fuzzing targets for TOML parsing and workspace member expansion

---

## `depguard-render`

**Owns**
- Renderers from report → text formats:
  - Markdown (PR comment)
  - GitHub Actions annotations

**Does not own**
- Report serialization (that's app layer)
- File I/O

**Public API**
```rust
pub fn render_markdown(report: &DepguardReport) -> String
pub fn render_github_annotations(report: &DepguardReport) -> Vec<String>
```

**Tests**
- Golden snapshot tests for Markdown
- Property tests: output is stable under re-rendering

---

## `depguard-app`

**Owns**
- Use case orchestration (check, md, annotations, explain)
- Report serialization to JSON
- Verdict → exit code mapping

**Does not own**
- CLI argument parsing (that's `depguard-cli`)
- Direct filesystem I/O

**Public API**
```rust
// Use cases
pub fn run_check(input: CheckInput) -> Result<CheckOutput>
pub fn run_markdown(report: &DepguardReport) -> String
pub fn run_annotations(report: &DepguardReport) -> Vec<String>
pub fn run_explain(identifier: &str) -> Option<Explanation>

// Serialization
pub fn serialize_report(report: &DepguardReport) -> Result<String>

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

**Does not own**
- Business logic (delegates to app/domain/settings/repo)

**Commands**
```
depguard check [--report-out PATH] [--write-markdown] [--base REF] [--head REF]
depguard md --report PATH [--output PATH]
depguard annotations --report PATH [--max N]
depguard explain <CHECK_ID|CODE>
```

**Exit codes**: 0 = pass/warn, 1 = tool error, 2 = policy failure

**Tests**
- `assert_cmd` integration tests
- End-to-end fixtures in `tests/fixtures/`

---

## `xtask`

**Owns**
- Schema generation (via `schemars`)
- Fixture updates
- Release packaging
- Developer loops that should not be in the CLI

**Commands**
```bash
cargo xtask schemas    # Generate JSON schemas
cargo xtask fixtures   # Regenerate test fixtures
cargo xtask release    # Prepare release artifacts
```

## See also

- [Architecture](architecture.md) — Data flow and crate relationships
- [Design Notes](design.md) — Why microcrates, domain purity
- [Testing](testing.md) — Per-crate test strategies
- [Contributing](../CONTRIBUTING.md) — How to add new crates or checks
