# depguard-settings

## Problem
Policy defaults, profiles, and check-level overrides become inconsistent when configuration precedence is implicit or duplicated.

## What this crate does
`depguard-settings` parses `depguard.toml`, applies profile defaults, validates values, and produces an effective config used by the domain.

## Resolution order (high to low)
1. CLI overrides
2. File config (`depguard.toml`)
3. Profile defaults (`strict`, `warn`, `compat`)

## Responsibilities
- Parse and validate user config
- Normalize severity/enablement and check-level settings
- Resolve baseline path and scope settings
- Return actionable validation errors for operators

## How to use
- Use this crate before invoking policy execution.
- Keep config errors as user-facing diagnostics, not domain errors.
- Depend on profiles for predictable migration behavior.

## Design constraints
- No filesystem reads here; callers provide text/bytes.
- No process state mutation.
- Deterministic resolution semantics.

## Related crates
- `depguard-app`
- `depguard-check-catalog`
- `depguard-domain`
