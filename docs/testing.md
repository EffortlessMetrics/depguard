# depguard — Testing Strategy

depguard is a gatekeeper. Trust is the product, and trust comes from:
- determinism
- resilience to weird TOML
- clear failure classification
- stable outputs over time

This doc describes the test stack and what each layer is protecting.

## 1) Golden fixtures (contract tests)

Every check should have fixtures that produce:

- exact `report.json` (byte-stable)
- exact `comment.md` (byte-stable, capped)

Fixtures should cover:
- workspace discovery edge cases (members/exclude globs, ordering, duplicates)
- dependency table shapes (string vs inline table vs workspace = true)
- target-specific dependency tables
- formatting oddities (comments, weird whitespace, ordering)
- Windows path separators and CRLF

Golden tests should run on all platforms, or at minimum validate normalization yields identical canonical paths.

## 2) BDD (behavior-as-spec)

Use a small set of `.feature` files describing expected behavior in human language.

The BDD harness should:
- set up a fixture repo directory
- run depguard with known flags
- compare output artifacts to expected snapshots

BDD is valuable here because you will add checks over time and you want a stable story:
- “what happens when I do X” remains readable and enforceable.

## 3) Property tests (proptest)

Use proptest for:
- generating randomized dependency spec shapes (string/table/workspace flags)
- ensuring normalization doesn’t crash and yields stable internal representation
- ordering invariants (randomly permuted input order → same output ordering)

## 4) Fuzzing (cargo-fuzz)

Fuzz targets should include:
- TOML parser inputs (arbitrary bytes) → must not panic
- workspace member glob expansion inputs → must not panic
- (optional) any diff-related parsing helpers if introduced

The key assertion: malformed input is classified as a tool/runtime error, not a crash.

## 5) Mutation testing (cargo-mutants)

Run mutation testing on:
- `depguard-domain` crate
- critical check logic and policy mapping
- ordering comparator and “ignore_publish_false” logic

Mutation testing is ideal here because the logic is small but critical; it catches “tests that don’t actually assert the rule.”

Recommended posture:
- run in scheduled CI initially (nightly)
- move to required CI if runtime is acceptable

## 6) Schema validation

In CI:
- validate sample receipts against:
  - `schemas/receipt.envelope.v1.json`
  - `schemas/depguard.report.v1.json`

If you generate receipts in tests, validate those artifacts too.

## 7) Explain coverage tests

Depguard must not emit undocumented codes.

Enforce in CI:
- every `(check_id, code)` in fixtures exists in explain registry
- every explain entry has a remediation sentence and (optional) a docs link
