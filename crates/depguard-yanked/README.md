# depguard-yanked

## Problem
Policies that check yank status must work in offline or restricted environments, but online crates index queries are brittle and non-deterministic.

## What this crate does
`depguard-yanked` parses yanked-version data from plain input text and performs exact version lookups (`crate + version`) for dependency checks.

## Supported inputs
- JSON object maps: `{"serde": ["1.2.3"]}`
- JSON array entries: `[{"crate": "serde", "version": "1.2.3"}]`
- Line-based formats: `serde 1.2.3` and `serde@1.2.3`

## Reference behavior
- Matching is exact; no semver range interpretation.
- Input is provided as a string; network and filesystem are caller responsibilities.
- Duplicate rows are merged deterministically.

## How to use
1. Materialize an index file in CI or from a prefetch step.
2. Parse with `parse_yanked_index`.
3. Pass the model into the domain evaluation path.

## Why this exists in its own crate
It keeps the `yanked_versions` check offline and deterministic while staying small and highly reusable for other callers.

## Related
- [depguard-domain-checks](../depguard-domain-checks/README.md)
- [depguard-app](../depguard-app/README.md)
