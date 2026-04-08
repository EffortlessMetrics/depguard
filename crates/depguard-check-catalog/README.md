# depguard-check-catalog

## Problem
Check metadata (docs, severity defaults, feature availability, remediations) can drift from implementation, creating mismatched CLI behavior and broken explain output.

## What this crate does
`depguard-check-catalog` is the source of truth for check metadata, feature gates, and explainability content.

## Responsibilities
- Register all check IDs and aliases
- Define default severities and feature gating
- Maintain a one-to-one explanation registry (check/code -> guidance)
- Support checks coverage validation in CI tooling

## How to use
- Import metadata when rendering explanations.
- Validate that every `(check_id, code)` pair has documentation and guidance.
- Keep this crate in sync with changes in `depguard-domain-checks`.

## Design constraints
- No external side effects.
- No direct dependency on command parsing or persistence.

## Related crates
- `depguard-domain-checks`
- `depguard-types`
- `depguard-app`
