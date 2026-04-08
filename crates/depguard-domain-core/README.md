# depguard-domain-core

## Problem
Domain-level checks share core models (dependency graph nodes, severities, and location primitives), but ad-hoc duplicates create incompatibilities.

## What this crate does
`depguard-domain-core` defines the pure primitives used by domain checks and adapters. Think of it as the structural backbone behind policy evaluation.

## Core responsibilities
- Define domain-level entities and contracts shared across policy layers
- Keep type semantics stable across check boundaries
- Provide canonical comparisons and ordering helpers

## How to use
- Depend on this crate when you need primitive domain types without pulling full check implementations.
- Use it as the baseline for domain feature-gated behavior and shared models.

## Why not put this elsewhere
This crate isolates frequently reused, low-level domain logic so check and application crates can stay composable and test-friendly.

## Related crates
- `depguard-domain` (policy orchestration)
- `depguard-domain-checks` (concrete checks)
- `depguard-types` (report payloads and public schema)
