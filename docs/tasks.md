# Task list (by crate)

This is a backlog tracking progress from scaffold to production.

## `depguard-types`

- [x] Add explicit `schema_id` constants and pin them in one place
- [x] Receipt/envelope DTOs with serde + schemars
- [x] Stable check IDs and codes in `ids` module
- [x] Explanation registry with `lookup_explanation()`
- [x] `RepoPath` canonical path type
- [x] Implement stable `fingerprint` hashing (SHA-256 recommended)
- [x] Add schema conformance tests for emitted JSON

## `depguard-settings`

- [x] Config model (`DepguardConfigV1`, `CheckConfig`)
- [x] Profile presets (`strict`, `warn`, `compat`)
- [x] `parse_config_toml()` function
- [x] `resolve_config()` with override precedence
- [x] Support for `fail_on` in config + CLI override
- [ ] Add validation errors that point to config keys
- [x] Add config schema generation via `schemars`

## `depguard-domain`

- [x] Domain model (`WorkspaceModel`, `ManifestModel`, `DependencyDecl`)
- [x] Policy types (`EffectiveConfig`, `CheckPolicy`, `Scope`, `FailOn`)
- [x] Evaluation engine with deterministic ordering
- [x] Check: `deps.no_wildcards`
- [x] Check: `deps.path_requires_version`
- [x] Check: `deps.path_safety`
- [x] Check: `deps.workspace_inheritance`
- [x] Verdict computation based on severities and fail_on
- [x] Implement target-specific dependency handling (target cfg)
- [x] Add allowlist semantics (glob matching, per-kind)
- [ ] Property tests: no panics; determinism; truncation invariants
- [ ] Mutation testing loop on domain crate

## `depguard-repo`

- [x] Workspace discovery via `discover_manifests()`
- [x] Root manifest parsing with `[workspace.dependencies]`
- [x] Member manifest parsing
- [x] `build_workspace_model()` orchestration
- [x] Line number tracking via toml_edit spans
- [x] Fuzz-safe APIs in `fuzz` module
- [ ] Improve workspace member glob semantics (align with Cargo edge cases)
- [x] Add fixtures for edge cases (virtual workspace, nested workspaces)
- [ ] Add fuzz harnesses for parsing/discovery

## `depguard-render`

- [x] Markdown renderer (`render_markdown()`)
- [x] GitHub Actions annotations (`render_github_annotations()`)
- [x] Character escaping for GHA format
- [ ] Add SARIF output (optional)
- [ ] Improve Markdown formatting (grouping, counts, links)
- [x] Snapshot tests for Markdown under multiple finding sets

## `depguard-app`

- [x] `run_check()` use case
- [x] `run_markdown()` use case
- [x] `run_annotations()` use case
- [x] `run_explain()` use case
- [x] `serialize_report()` JSON output
- [x] `verdict_exit_code()` mapping
- [ ] Better error context messages

## `depguard-cli`

- [x] `check` subcommand with report output
- [x] `md` subcommand (read JSON, render markdown)
- [x] `annotations` subcommand
- [x] `explain` subcommand
- [x] Global options: `--repo-root`, `--config`, `--profile`, `--scope`, `--max-findings`
- [x] Diff scope with `--base`/`--head` git integration
- [x] Exit code semantics (0=pass, 1=error, 2=fail)
- [ ] Artifact layout controls (`--out-dir`, etc.)
- [x] End-to-end integration tests with assert_cmd

## `xtask`

- [x] Schema generation from Rust types
- [ ] Fixture update automation
- [ ] CI smoke script generation
- [ ] Release packaging automation

## Documentation

- [x] `docs/architecture.md` — Hexagonal architecture design
- [x] `docs/design.md` — Design decisions and patterns
- [x] `docs/microcrates.md` — Crate contracts and APIs
- [x] `docs/testing.md` — Test strategy
- [x] `docs/checks.md` — Check catalog with remediation
- [x] `docs/config.md` — Configuration reference
- [x] `docs/implementation-plan.md` — Development roadmap
- [x] Per-crate `CLAUDE.md` files
