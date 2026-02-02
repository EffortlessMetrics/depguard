# Testing

The intent is layered tests with different “optics”:

## Unit tests (fast)

Live next to code:
- check behavior under small, explicit inputs
- config merge precedence
- renderer formatting

## Property tests (broad)

Use `proptest` for invariants:
- domain evaluation is deterministic
- no panics on arbitrary manifests
- ordering is stable

## BDD (integration semantics)

Keep scenarios readable:
- “given a workspace with …”
- “when depguard runs …”
- “then report contains …”

The scaffold includes a `tests/bdd/` folder for `.feature` files. Wiring to a runner is left to implementation
choice (either `cucumber` crate, or explicit table-driven scenario tests).

## Fuzzing (parser hardening)

TOML parsing is the highest risk surface.
The scaffold includes a `fuzz/` directory placeholder for `cargo-fuzz` harnesses:
- fuzz manifest parsing
- fuzz workspace discovery member expansion

## Mutation testing (test quality)

Use `cargo-mutants` (or similar) to ensure tests fail when behavior changes.
The discipline:
- run mutants on `depguard-domain` first
- exclude renderers if they produce noisy diffs until stabilized
