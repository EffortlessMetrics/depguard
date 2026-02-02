# Microcrates

This document is the **contract map**: what each crate owns, what it is allowed to depend on, and what
its public API is supposed to look like.

The goal is not microcrates for their own sake; the goal is to keep:
- the **policy engine** easy to test and reason about
- the **receipt format** stable
- the **IO surface** small and replaceable

## `depguard-types`

**Owns**
- Receipt/envelope DTOs (`ReportEnvelope`, `Finding`, `Verdict`, …)
- Canonical repo path type (`RepoPath`)
- Stable check IDs and codes registry (string constants)
- Schema IDs and versions

**Does not own**
- policy logic
- config merging
- filesystem access

**Public API**
- `receipt::*` structs
- `RepoPath`
- `ids::*` constants
- helper ordering functions

**Tests**
- serde roundtrip + schema conformance tests (golden JSON)

## `depguard-settings`

**Owns**
- config model (`DepguardConfigV1`)
- preset profiles (`strict`, `warn`, `compat`, etc.)
- merge/override rules (repo file + CLI overrides)

**Public API**
- `parse_config_toml(&str) -> DepguardConfigV1`
- `effective_config(config: DepguardConfigV1, overrides: Overrides) -> EffectiveConfig`

**Tests**
- table-driven merge tests
- property tests: “merging is associative for disjoint keys” (where intended)

## `depguard-domain`

**Owns**
- domain model (`WorkspaceModel`, `ManifestModel`, `DependencyDecl`)
- check registry and evaluation engine
- deterministic ordering + truncation behavior

**Public API**
- `evaluate(model: &WorkspaceModel, cfg: &EffectiveConfig) -> DomainReport`

**Notes**
- No IO; no TOML; no git.
- If a check needs file context beyond what is in the model, the model must be extended explicitly.

**Tests**
- unit tests per check
- property tests: determinism, stable sorting, no panics on arbitrary inputs

## `depguard-repo`

**Owns**
- workspace discovery (`Cargo.toml` workspace members/excludes)
- reading files from disk
- parsing manifests (TOML -> domain model)
- diff scoping (git changed files -> manifest set)

**Public API**
- `build_workspace_model(root: &Utf8Path, scope: ScopeInput) -> WorkspaceModel`

**Notes**
- This crate is where parsing gets messy; keep complexity here, not in the domain.

**Tests**
- fixture-driven tests with tiny workspaces
- fuzzing targets for TOML parsing and workspace member expansion

## `depguard-render`

**Owns**
- renderers from report -> text formats:
  - Markdown (PR comment)
  - GitHub Actions annotations
  - (optional) SARIF

**Public API**
- `render_markdown(report: &DepguardReport, opts: MdOpts) -> String`
- `render_annotations(report: &DepguardReport) -> Vec<String>`

**Tests**
- golden snapshot tests for Markdown
- property tests: output is stable under re-ordering of already-sorted findings (should be no-op)

## `depguard-cli`

**Owns**
- clap CLI definitions
- wiring: settings + repo + domain + render
- artifact write layout + exit codes

**Notes**
- keep the CLI mostly glue; any “business logic” belongs in domain/settings/repo.

**Tests**
- `assert_cmd` integration tests
- end-to-end fixtures

## `xtask`

**Owns**
- schema generation (if deriving via `schemars`)
- fixture updates
- release packaging
- “developer loops” that should not be in the CLI
