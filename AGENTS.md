# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**depguard** is a repo-truth dependency manifest hygiene sensor for Rust workspaces. It scans `Cargo.toml` files for hygiene violations and emits versioned receipts in JSON format. Key characteristics:

- Deterministic: Same inputs → same outputs (byte-stable)
- Offline: No network access, no builds
- Schema-first: Versioned JSON schemas define the contract

## Build and Development Commands

```bash
# Build
cargo build                              # Debug build
cargo build --release                    # Release build

# Test
cargo test                               # All tests
cargo test --lib                         # Unit tests only
cargo test --test '*'                    # Integration tests only
cargo test -p depguard-domain            # Single crate tests

# Linting and formatting
cargo fmt --check                        # Check formatting
cargo fmt                                # Apply formatting
cargo clippy --all-targets --all-features

# Mutation testing (domain crate)
cargo mutants --package depguard-domain

# Fuzzing
cargo +nightly fuzz run fuzz_toml_parser
```

## Architecture

The project uses **hexagonal (ports & adapters)** architecture with a multi-crate workspace:

| Crate | Purpose |
|-------|---------|
| `depguard-types` | DTOs, config, report, findings; schema IDs; stable codes |
| `depguard-domain` | Rule implementations; policy evaluation (pure, no I/O) |
| `depguard-repo` | Workspace discovery; manifest loading; TOML parsing; diff-scope |
| `depguard-render` | Markdown and GitHub annotations renderers |
| `depguard-app` | Use cases: check, md, annotations, explain; error handling |
| `depguard-cli` | clap wiring; filesystem paths; exit code mapping |
| `xtask` | Schema emission; fixture generation; release tasks |

**Critical constraint**: The domain layer is pure—no filesystem, no stdout logging, no clap dependencies.

## Data Flow

```
CLI → App (use case) → Repo discovery → Manifest parsing → Dependency walk
    → Domain checks → Findings + Verdict → Receipt writer → Optional renderers
```

## Testing Strategy

1. **Golden fixtures**: Byte-stable `expected.report.json`, `expected.comment.md`, and `expected.annotations.txt` in `tests/fixtures/`
2. **BDD scenarios**: `.feature` files in `tests/features/`
3. **Property tests**: proptest for dependency spec shapes and ordering
4. **Fuzzing**: TOML parser and glob expansion must not panic
5. **Mutation testing**: Run on `depguard-domain` to validate assertions

## Key Schemas

Located in `schemas/`:
- `receipt.envelope.v1.json` — Vendored legacy envelope (external contract)
- `depguard.report.v1.json` — Legacy depguard report schema
- `depguard.report.v2.json` — Current depguard report schema
- `depguard.config.v1.json` — Configuration file schema

## CLI Commands

```bash
depguard check                           # Analyze manifests, emit receipt
depguard md --report <path>              # Render Markdown from receipt
depguard annotations --report <path>     # Render GitHub annotations
depguard explain <check_id|code>         # Show remediation guidance
```

**Exit codes**: 0 = pass, 2 = policy failure, 1 = tool/runtime error

## Protocol Discipline

- Stable codes: Never rename; deprecate via aliases only
- Extension via `data` object only in receipts
- Findings ordered deterministically: severity → path → line → check_id → code → message
- Every `(check_id, code)` pair must have an explain registry entry

## Documentation

**User-facing:**
- `docs/quickstart.md` — Getting started guide
- `docs/config.md` — Configuration file format
- `docs/checks.md` — Check IDs, codes, remediation guidance
- `docs/ci-integration.md` — CI/CD pipeline setup
- `docs/troubleshooting.md` — FAQ and common issues

**Architecture:**
- `docs/architecture.md` — Hexagonal architecture design
- `docs/design.md` — Data flow, parsing, rule evaluation
- `docs/microcrates.md` — Crate contracts and APIs
- `docs/implementation-plan.md` — 5-phase development roadmap
- `docs/testing.md` — Test strategy details

**Contributing:**
- `CONTRIBUTING.md` — How to contribute
