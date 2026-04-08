# depguard-domain-checks

## Problem
Dependency hygiene rules (wildcards, yanked versions, path safety, etc.) tend to accrete together and become difficult to reason about as one giant module.

## What this crate does
`depguard-domain-checks` contains the individual, pure check implementations used by `depguard-domain`.

## How checks are organized
- Each check converts domain model input into one or more findings.
- Findings are deterministic and only depend on inputs.
- Feature flags control optional rule availability.

## Design constraints
- No I/O, no CLI concerns, no rendering behavior.
- Stable ordering of findings.
- Check IDs and code values remain aligned with `depguard-check-catalog` and `depguard-types`.

## How to use
- Add a new rule by implementing the expected check trait/shape in this crate.
- Register check availability and metadata in check catalog.
- Keep logic focused: input model in, finding stream out.

## Related crates
- `depguard-domain`
- `depguard-check-catalog`
- `depguard-settings`
