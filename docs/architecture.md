# depguard Architecture

## Problem
Dependency checking was previously easy to evolve in behavior and hard to keep side-effect-free. The result is policy decisions that are difficult to test, reason about, and migrate safely.

## What this solves
`depguard` now uses a strict layered architecture so policy evaluation is isolated from CLI, filesystem, and rendering concerns.

## Architecture in one line
- **Domain layer** is pure and deterministic.
- **Adapters** own I/O and conversions.
- **Application layer** orchestrates use cases.
- **CLI layer** parses process arguments and maps exit codes.

## Data flow
1. `depguard-cli` parses command arguments, paths, and scope.
2. `depguard-settings` resolves effective configuration.
3. `depguard-repo` discovers and reads manifests.
4. `depguard-repo-parser` parses `Cargo.toml` into in-memory models.
5. `depguard-domain` evaluates checks and produces findings.
6. `depguard-app` wraps results in report envelopes.
7. `depguard-render` formats output for Markdown/JSONL/SARIF/JUnit/annotations.

## Core boundaries
- `depguard-domain*` never reads files and never invokes subprocesses.
- `depguard-cli` is the only stable boundary that performs process I/O and exit-code mapping.
- `depguard-render` is output-format specific only.

## Crate layers

- Foundation
  - `depguard-types` (schema IDs, DTOs, IDs)
  - `depguard-yanked` (offline yanked lookup model)
  - `depguard-test-util` (test helpers)
- Domain
  - `depguard-domain-core` (primitives)
  - `depguard-domain-checks` (pure check implementations)
  - `depguard-check-catalog` (check metadata)
  - `depguard-domain` (policy orchestration)
- Adapters
  - `depguard-repo-parser` (pure TOML parsing)
  - `depguard-repo` (discovery, scope resolution)
  - `depguard-settings` (config resolution)
  - `depguard-render` (format adapters)
- Application / UI
  - `depguard-app` (use cases)
  - `depguard-cli` / `cargo-depguard` (process entry points)
- Tooling
  - `xtask` (maintenance and schema/fixture tasks)

## Why this is intentional
- Swaps of adapters (or renderers) don’t change policy behavior.
- Deterministic domain makes golden fixtures and conformance tests reliable.
- CI integrations depend on stable contracts, not implementation details.

## Implementation map
- `docs/microcrates.md` for ownership and dependencies.
- `docs/design.md` for component interactions.
- `docs/output-contract.md` for result shape.
