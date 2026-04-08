# depguard-types

## Problem
Every layer in a dependency policy pipeline needs the same check IDs, schema versions, and finding contracts. Without one shared layer, payloads drift and consumers break in subtle ways.

## What this crate does
`depguard-types` is the stable protocol boundary for depguard. It owns IDs, schemas, and shared DTOs used by all adapters and renderers.

## Reference model
- Report envelopes for legacy and current versions
- Finding payloads and severities
- Baseline suppression schema
- Buildfix schema
- Explanation registry entries
- Stable crate/path helpers

## How to use
- Use this crate whenever you serialize, deserialize, compare, or render depguard outputs.
- Treat `check_id` and `code` values as append-only contracts.
- Use the schema constants for compatibility checks and tests.

## Design constraints
- No I/O and no side effects; pure data definitions.
- Public IDs and codes are stable: deprecate only, never rename.
- JSON and schema serialization must remain deterministic.

## Typical dependency graph
- Consumed by: `depguard-app`, `depguard-render`, `depguard-check-catalog`, `depguard-cli`
- Produced against by: all renderers and workspace adapters

## Related docs
- [docs/architecture.md](../../docs/architecture.md)
- [docs/microcrates.md](../../docs/microcrates.md)
