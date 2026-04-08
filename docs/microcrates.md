# depguard Microcrates

## Problem
A large Rust workspace gets fragile when crate responsibilities are unclear.

## Layer map

- `depguard-types`
  - Public contracts: schema IDs, report types, stable IDs, explanations.
- `depguard-domain-core`
  - Policy primitives and shared domain types.
- `depguard-domain-checks`
  - Individual check logic.
- `depguard-check-catalog`
  - Check metadata and profile defaults.
- `depguard`
  - Public facade over the domain evaluation surface.
- `depguard-domain`
  - Internal domain engine and orchestration.
- `depguard-repo-parser`
  - Pure manifest parsing and location extraction.
- `depguard-repo`
  - Workspace discovery, path handling, diff selection.
- `depguard-settings`
  - Config parsing and effective config resolution.
- `depguard-render`
  - Report rendering adapters.
- `depguard-app`
  - Use-case orchestration.
- `depguard-cli`
  - Command parsing and process interface.
- `depguard-yanked`
  - Offline exact yanked-version lookup.
- `depguard-inline-suppressions`
  - Inline manifest suppression directives.
- `depguard-test-util`
  - Deterministic test helpers for workspace tooling.
- `xtask`
  - Developer automation (schemas, fixtures, release prep).

## Dependency invariants
- Foundation crates have the smallest dependency surfaces.
- Domain crates should remain side-effect free.
- Outer layers can depend on inner layers, not the other way around.

## Use this index when
- You need to decide where to place a new feature.
- You are updating dependencies between crates.
- You are debugging test failures due to contract changes.
