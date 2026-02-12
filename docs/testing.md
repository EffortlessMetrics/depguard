# Testing

> **Navigation**: [Architecture](architecture.md) | [Design](design.md) | [Microcrates](microcrates.md) | Testing | [Contributing](../CONTRIBUTING.md)

The testing strategy is layered with different "optics" for different concerns.

## Test commands

```bash
# All tests
cargo test

# By scope
cargo test --lib                    # Unit tests only
cargo test --test '*'               # Integration tests only
cargo test -p depguard-domain       # Single crate

# Linting
cargo fmt --check
cargo clippy --all-targets --all-features

# Mutation testing (domain crate)
cargo mutants --package depguard-domain

# Fuzzing (requires nightly)
cargo +nightly fuzz run fuzz_toml_parser
```

## Unit tests (fast)

Live next to code in each crate:

| Crate | Focus |
|-------|-------|
| `depguard-types` | Serde roundtrip, explanation coverage |
| `depguard-domain` | Check behavior under small, explicit inputs |
| `depguard-settings` | Config parse, merge precedence, validation |
| `depguard-repo` | Manifest parsing, workspace discovery |
| `depguard-render` | Renderer formatting |

Example locations:
- `crates/depguard-domain/src/checks/no_wildcards.rs` → `#[cfg(test)] mod tests`
- `crates/depguard-settings/src/resolve.rs` → `#[cfg(test)] mod tests`

## Property tests (broad)

Use `proptest` for invariants in `depguard-domain`:

- Domain evaluation is deterministic (same input → same output)
- No panics on arbitrary manifest structures
- Findings ordering is stable under re-evaluation
- Truncation preserves ordering invariants

Location: `crates/depguard-domain/src/engine.rs` tests module

## Golden fixtures (byte-stable)

Canonical fixtures live in `tests/fixtures/`:

| File | Purpose |
|------|---------|
| `expected.report.json` | Expected JSON report output |
| `expected.comment.md` | Expected Markdown comment |
| `expected.annotations.txt` | Expected GitHub Actions annotations |

Regenerate by running depguard on each fixture and updating the `expected.*` files.

These tests catch unintentional output drift.

## BDD scenarios (integration)

Feature files in `tests/features/`:

```gherkin
Feature: Wildcard detection
  Scenario: Detects * in version
    Given a workspace with a dependency version "*"
    When depguard runs
    Then the report contains a finding with code "wildcard_version"
```

Wiring: Either `cucumber` crate or explicit table-driven scenario tests in `tests/`.

Location: `tests/features/depguard.feature`

## Integration tests

End-to-end tests using real CLI invocations:

Location: `crates/depguard-cli/tests/`

These tests:
- Run the actual binary
- Use fixture workspaces
- Verify exit codes, JSON output, Markdown output

## Fuzzing (parser hardening)

TOML parsing is the highest risk surface. Fuzz targets ensure no panics.

Location: `fuzz/` directory (when present)

Targets:
- `fuzz_toml_parser` — arbitrary TOML input to manifest parser
- `fuzz_glob_expansion` — arbitrary glob patterns

The `depguard-repo` crate exposes `fuzz` module APIs that return `Option` instead of `Result` and are guaranteed not to panic.

## Mutation testing (test quality)

Use `cargo-mutants` to ensure tests fail when behavior changes.

```bash
cargo mutants --package depguard-domain
```

Discipline:
- Run mutants on `depguard-domain` first (core logic)
- Exclude renderers if they produce noisy diffs until stabilized
- Target: <5% surviving mutants in domain crate

## Schema validation

CI validates that emitted reports conform to JSON schemas:

The CLI integration tests validate reports against:
- `schemas/depguard.report.v1.json`
- `schemas/depguard.report.v2.json`

## Test data locations

| Path | Contents |
|------|----------|
| `tests/fixtures/` | Golden output files |
| `tests/features/` | BDD feature files |
| `crates/*/tests/` | Per-crate integration tests |
| `examples/` | Example config files |

## See also

- [Contributing](../CONTRIBUTING.md) — How to contribute and run tests
- [Design Notes](design.md) — Pure domain and determinism
- [Architecture](architecture.md) — Crate responsibilities
- [Implementation Plan](implementation-plan.md) — Test requirements by phase
