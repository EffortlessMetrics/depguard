# depguard-repo-parser

## Problem
Parsing `Cargo.toml` manually and safely at scale is easy to get wrong, especially around comments, table shapes, and error surfaces.

## What this crate does
`depguard-repo-parser` provides deterministic, pure parsing of Cargo manifest structures into depguard domain models.

## Responsibilities
- Parse manifest content into shared in-memory representations
- Preserve enough location info for actionable findings
- Surface parse errors deterministically (no ad-hoc panics)

## How to use
- Feed manifest text plus path metadata into parser entry points.
- Handle parse diagnostics as first-class structured errors.
- Treat parser as a pure transformer before adapter-level policy handling.

## Design constraints
- No filesystem access and no command execution.
- Deterministic behavior for identical input text.

## Related crates
- `depguard-repo`
- `depguard-domain-core`
- `depguard-domain` (consumer layer)
