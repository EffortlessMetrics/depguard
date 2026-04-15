# depguard Microcrates

## Problem
A large Rust workspace gets fragile when crate responsibilities are unclear.

## Layer map

- `depguard-cli`
  - Public product entrypoint and process boundary.
- `depguard-types`
  - Public contracts: schema IDs, report types, stable IDs, explanations.
- `depguard-domain-core`
  - Policy primitives and shared domain types (internal).
- `depguard-domain-checks`
  - Individual check logic (internal implementation).
- `depguard-check-catalog`
  - Check metadata and profile defaults (internal metadata).
- `depguard`
  - Public facade over evaluation for Rust embedding.
- `depguard-domain`
  - Internal domain engine and orchestration.
- `depguard-repo`
  - Pure manifest parsing, inline suppression extraction, and location tracking (now internal to parser module).
- `depguard-repo`
  - Workspace discovery, path handling, diff selection.
- `depguard-settings`
  - Config parsing and effective config resolution.
- `depguard-render`
  - Report rendering adapters.
- `depguard-app`
  - Use-case orchestration.
- `depguard-yanked`
  - Offline exact yanked-version lookup.
- `depguard-test-util`
  - Deterministic test helpers for workspace tooling (not intended for external publishing).
- `xtask`
  - Developer automation (schemas, fixtures, release prep).

## Public vs internal publishing intent

- Supported public surfaces for external consumers: `depguard-cli`, `depguard`, and `depguard-types`.
- Published internals (supporting crates): `depguard-domain`, `depguard-domain-*`, `depguard-repo`,
  `depguard-settings`, `depguard-render`, `depguard-yanked`, `depguard-app`.
- Internal-only: `depguard-test-util` (set `publish = false`) and `xtask`.

## Collapse target for alpha

- `depguard-inline-suppressions` is now implemented in `depguard-repo::parser` as an internal module.
- `depguard-repo-parser` was merged into `depguard-repo::parser` in this PR line.
- Next cleanup target is `depguard-domain-core` + `depguard-domain-checks` + `depguard-check-catalog` into `depguard-domain`.

## Dependency invariants
- Foundation crates have the smallest dependency surfaces.
- Domain crates should remain side-effect free.
- Outer layers can depend on inner layers, not the other way around.

## Use this index when
- You need to decide where to place a new feature.
- You are updating dependencies between crates.
- You are debugging test failures due to contract changes.
