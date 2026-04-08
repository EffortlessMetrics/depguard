# depguard-app

## Problem
Domain checks, configuration, rendering, and reporting all need consistent orchestration, but duplicating this wiring creates inconsistent behavior between entrypoints.

## What this crate does
`depguard-app` implements use-case orchestration for depguard: check, explain, baseline, rendering, and fix workflows.

## Use cases
- Execute an end-to-end policy run from model + config + inputs
- Produce report receipts and verdict summaries
- Generate baseline suppressions
- Resolve and emit explanation payloads
- Route artifacts to renderers

## How to use
- Treat this crate as the application layer boundary.
- Keep I/O and CLI concerns at outer layers (`depguard-cli`).
- Keep domain/policy logic inside `depguard-domain`.

## Design constraints
- Stable behavior: same inputs -> same outputs
- Explicit error types for business-level failures
- Feature-gated checks are respected via resolved settings

## Related crates
- `depguard-domain`
- `depguard-settings`
- `depguard-repo`
- `depguard-render`
