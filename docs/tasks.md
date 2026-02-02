# Task list (by crate)

This is a pragmatic backlog to get from scaffold -> production.

## `depguard-types`

- [ ] Add explicit `schema_id` constants and pin them in one place
- [ ] Implement stable `fingerprint` hashing (SHA-256 recommended)
- [ ] Add schema conformance tests for emitted JSON

## `depguard-settings`

- [ ] Add support for `fail_on` in config + CLI override
- [ ] Add validation errors that point to config keys
- [ ] Add config schema generation via `schemars` (optional)

## `depguard-domain`

- [ ] Implement target-specific dependency parsing in model (target cfg)
- [ ] Add allowlist semantics (glob matching, per-kind)
- [ ] Expand checks catalog + explain registry
- [ ] Property tests: no panics; determinism; truncation invariants
- [ ] Mutation testing loop on domain crate

## `depguard-repo`

- [ ] Capture best-effort line/col in locations (toml_edit span APIs)
- [ ] Improve workspace member glob semantics (align with Cargo)
- [ ] Add fixtures for edge cases (virtual workspace, nested workspaces)
- [ ] Add fuzz harnesses for parsing/discovery

## `depguard-render`

- [ ] Add SARIF output (optional)
- [ ] Improve Markdown formatting (grouping, counts, links)
- [ ] Snapshot tests for Markdown under multiple finding sets

## `depguard-cli`

- [ ] `md` subcommand: read JSON and render markdown
- [ ] `annotations` subcommand
- [ ] Better exit code semantics (pass/warn/fail vs fail_on)
- [ ] Artifact layout controls (`--out-dir`, etc.)
- [ ] End-to-end integration tests

## `xtask`

- [ ] Generate schemas from Rust types
- [ ] CI smoke script generation
- [ ] Release packaging automation
