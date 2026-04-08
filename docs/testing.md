# Testing

## Problem
Policy tooling regresses quietly when fixtures, ordering, and edge cases are not continuously covered.

## Test strategy

- Golden fixtures (`tests/fixtures/`) with deterministic receipts.
- Integration tests for command paths and scope behavior.
- Unit tests for parsing, config resolution, and check algorithms.
- Property tests for shape and ordering invariants.
- Conformance checks against schema contracts.
- Optional fuzzing for manifest parser resilience.

## Recommended commands

```bash
cargo test
cargo test --lib
cargo test --test '*'
cargo +nightly fuzz run fuzz_toml_parser
```

## Determinism requirements
- Outputs must be byte-stable under identical input.
- Non-deterministic fields should be normalized in fixture assertions.

## CI coverage expectation
- New checks require fixture updates and explainability assertions.
- Renderer changes should include all output formats.
- Schema-emitting changes require fixture re-generation.

## Reviewing failures
- Re-run the specific suite first (e.g., domain, then app, then integration).
- Confirm no output contract was unintentionally altered.
- Keep fixture diffs minimal and explain why they exist in PR notes.

## Helpful rule
If a test update changes ordering without policy change, treat it as a release-impacting risk.
