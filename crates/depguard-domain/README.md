# depguard-domain

## Problem
Policy evaluation logic is often mixed with command-line parsing, filesystem traversal, and rendering, which makes behavior hard to test and hard to prove deterministic.

## What this crate does
`depguard-domain` is depguard’s pure policy engine. Given an in-memory model and resolved configuration, it returns findings, verdicts, and summary data with no I/O.

## Responsibilities
- Evaluate active checks against dependency metadata
- Resolve check results into canonical findings
- Produce deterministic severity and ordering guarantees
- Keep policy behavior independent of transport format and CLI options

## How to use
- Build or inject parsed models and resolved settings in an adapter.
- Call domain use functions to obtain results.
- Forward findings into `depguard-render` or `depguard-app`.

## Quality gates
- No filesystem or network usage
- No process/environment side effects
- Explicitly unit-tested and suitable for property testing

## Related crates
- `depguard-domain-core` for primitives
- `depguard-domain-checks` for individual check implementations
- `depguard-app` for orchestration and commands
- `depguard-check-catalog` for check metadata
