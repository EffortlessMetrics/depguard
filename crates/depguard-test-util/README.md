# depguard-test-util

## Problem
Golden fixture tests are unreliable when outputs contain timestamps, versions, and durations that differ per run.

## What this crate does
`depguard-test-util` provides deterministic normalization helpers for repository tests and `xtask` fixture pipelines.

## Key helpers
- Replace non-deterministic fields with stable placeholders
- Normalize nested JSON values used by report contracts
- Optional deterministic crypto fixture utilities behind a feature flag

## Usage intent
- Use primarily in workspace tests/build tooling, not as shipped runtime API.
- Publish it only as support for crates.io packaging and verification of workspace crates that use it in dev-dependencies.
- Normalize before string/JSON snapshot comparison.

## Design constraints
- Minimal dependency surface (`serde_json` base)
- Feature-gated crypto fixture support (`crypto-fixtures`) to keep defaults light
- Stable placeholders across versions

## Why not in another crate
Keeping normalization separate keeps production crates minimal while allowing `xtask` and test modules to share the same deterministic baseline utilities.

## Related
- `xtask` fixture generation
- `depguard-types` report normalization targets
