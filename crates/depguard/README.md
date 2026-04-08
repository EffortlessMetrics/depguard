# depguard

## Problem
Downstream Rust integrations should not need to know how depguard splits its internal engine across `depguard-domain`, `depguard-domain-core`, and `depguard-domain-checks`.

## What this crate does
`depguard` is the public Rust facade crate for depguard’s pure evaluation API.

## Surface
- Re-export domain model and policy modules
- Re-export `evaluate()` and `checks::run_all()`
- Preserve ergonomic root imports for common types

## Intended use
- Depend on `depguard` when embedding depguard policy evaluation in another Rust crate
- Treat `depguard-domain` as the engine implementation detail behind this facade

## Related crates
- `depguard-domain`
- `depguard-domain-core`
- `depguard-domain-checks`
