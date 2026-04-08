# Task list (by crate)

This is a backlog tracking progress from scaffold to production.

## `depguard-types`

- [x] Add explicit `schema_id` constants and pin them in one place
- [x] Receipt/envelope DTOs with serde + schemars
- [x] Stable check IDs and codes in `ids` module
- [x] Explanation registry with `lookup_explanation()`
- [x] `RepoPath` canonical path type
- [x] Implement stable `fingerprint` hashing (SHA-256)
- [x] Add schema conformance tests for emitted JSON
- [x] Baseline suppression types

## `depguard-settings`

- [x] Config model (`DepguardConfigV1`, `CheckConfig`)
- [x] Profile presets (`strict`, `warn`, `compat`)
- [x] `parse_config_toml()` function
- [x] `resolve_config()` with override precedence
- [x] Support for `fail_on` in config + CLI override
- [x] Add config schema generation via `schemars`
- [ ] Add validation errors that point to config keys

## `depguard-domain-core`

- [x] Core domain types (`WorkspaceModel`, `ManifestModel`, `DependencyDecl`)
- [x] Policy types (`EffectiveConfig`, `CheckPolicy`, `Scope`, `FailOn`)
- [x] Verdict computation based on severities and fail_on
- [x] Target-specific dependency handling (target cfg)

## `depguard-domain-checks`

- [x] Check: `deps.no_wildcards`
- [x] Check: `deps.path_requires_version`
- [x] Check: `deps.path_safety`
- [x] Check: `deps.workspace_inheritance`
- [x] Check: `deps.git_requires_version`
- [x] Check: `deps.dev_only_in_normal`
- [x] Check: `deps.default_features_explicit`
- [x] Check: `deps.no_multiple_versions`
- [x] Check: `deps.optional_unused`
- [x] Check: `deps.yanked_versions`
- [x] Evaluation engine with deterministic ordering
- [x] Allowlist semantics (glob matching, per-kind)
- [ ] Property tests: no panics; determinism; truncation invariants
- [ ] Mutation testing loop on domain crate

## `depguard-check-catalog`

- [x] Check metadata and explanation registry
- [x] `lookup_explanation()` function for all check/code pairs

## `depguard-inline-suppressions`

- [x] Inline suppression comment parser
- [x] Fuzz-safe APIs

## `depguard-repo-parser`

- [x] TOML parsing with toml_edit
- [x] Line number tracking via spans
- [x] Fuzz-safe parsing APIs

## `depguard-repo`

- [x] Workspace discovery via `discover_manifests()`
- [x] Root manifest parsing with `[workspace.dependencies]`
- [x] Member manifest parsing
- [x] `build_workspace_model()` orchestration
- [x] Diff-scope with `--base`/`--head` git integration
- [x] Add fixtures for edge cases (virtual workspace, nested workspaces)
- [ ] Improve workspace member glob semantics (align with Cargo edge cases)

## `depguard-yanked`

- [x] Offline yanked-index parsing
- [x] Exact version lookup
- [x] Live index query support (`--yanked-live`)

## `depguard-render`

- [x] Markdown renderer (`render_markdown()`)
- [x] GitHub Actions annotations (`render_github_annotations()`)
- [x] Character escaping for GHA format
- [x] SARIF output
- [x] JUnit XML output
- [x] JSON Lines output
- [x] Snapshot tests for renderers

## `depguard-app`

- [x] `run_check()` use case
- [x] `run_markdown()` use case
- [x] `run_annotations()` use case
- [x] `run_explain()` use case
- [x] `run_baseline()` use case
- [x] `run_fix()` use case (buildfix plan generation)
- [x] `serialize_report()` JSON output
- [x] `verdict_exit_code()` mapping
- [ ] Better error context messages

## `depguard-cli`

- [x] `check` subcommand with report output
- [x] `md` subcommand (read JSON, render markdown)
- [x] `annotations` subcommand
- [x] `sarif` subcommand
- [x] `junit` subcommand
- [x] `jsonl` subcommand
- [x] `explain` subcommand
- [x] `baseline` subcommand
- [x] `fix` subcommand
- [x] Global options: `--repo-root`, `--config`, `--profile`, `--scope`, `--max-findings`
- [x] Diff scope with `--base`/`--head` git integration
- [x] Exit code semantics (0=pass, 1=error, 2=fail)
- [x] Artifact layout controls (`--out-dir`, etc.)
- [x] End-to-end integration tests with assert_cmd

## `xtask`

- [x] Schema generation from Rust types
- [x] Fixture update automation
- [ ] CI smoke script generation
- [ ] Release packaging automation

## Documentation

- [x] `docs/architecture.md` — Hexagonal architecture design
- [x] `docs/design.md` — Design decisions and patterns
- [x] `docs/microcrates.md` — Crate contracts and APIs
- [x] `docs/testing.md` — Test strategy
- [x] `docs/checks.md` — Check catalog with remediation (all 10 checks)
- [x] `docs/config.md` — Configuration reference
- [x] `docs/quickstart.md` — Getting started guide
- [x] `docs/ci-integration.md` — CI/CD pipeline setup
- [x] `docs/troubleshooting.md` — FAQ and common issues
- [x] `docs/implementation-plan.md` — Development roadmap
- [x] Per-crate `CLAUDE.md` files
- [x] Per-crate `README.md` files
