# depguard Design Notes

## Problem
Complexity in parsing and policy evaluation can hide tradeoffs and create hidden coupling.

## What this document is for
A short rationale for how `depguard` balances strictness, determinism, and practical CI behavior.

## Design decisions

- **Input-first model**: `depguard` evaluates `Cargo.toml` data directly and avoids cargo metadata.
- **Offline by default**: no network at runtime, so behavior is stable in CI, air-gapped runners, and local scripts.
- **Pure domain**: all rule evaluation in `depguard-domain` is free of I/O.
- **Stable contracts**: IDs, schema IDs, and finding codes are versioned and never renamed.
- **Deterministic output ordering**: predictable for snapshot tests and audit trails.
- **Explicit opt-in side effects**: fixes require explicit `--apply`.

## Non-functional constraints
- No build-time metadata resolution.
- No ad-hoc process-level state in domain code.
- No dynamic code paths that change ordering silently.

## Boundary rules
- **Domain** returns findings only.
- **Adapters** prepare and translate external inputs.
- **Renderers** do not alter policy outcomes.
- **CLI** handles `std::process::Command` and filesystem boundaries.

## Tradeoff notes
- Not resolving the Cargo dependency graph keeps runs fast and reproducible, but it limits checks that require full dependency graph context.
- Exact-match yanked checks improve determinism, not semver reasoning.

## When to revisit
Adjust these decisions if future requirements need dependency graph data, remote index lookups, or richer fix planning across multiple files.
