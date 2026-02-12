# Fuzz Targets

This directory contains `cargo-fuzz` targets for testing parsing robustness in `depguard-repo`.

## Prerequisites

Install cargo-fuzz (requires nightly Rust):

```bash
rustup install nightly
cargo install cargo-fuzz
```

## Available Targets

### `fuzz_toml_parser`

Fuzzes the Cargo.toml manifest parser with arbitrary byte sequences.
Tests both root manifest and member manifest parsing paths.

**Goal**: Parser should never panic on any input (errors are acceptable).

```bash
cargo +nightly fuzz run fuzz_toml_parser
```

### `fuzz_glob_expansion`

Fuzzes the workspace member glob pattern expansion with arbitrary patterns and paths.
Uses structured fuzzing via `Arbitrary` for more effective test generation.

**Goal**: Glob expansion should never panic on any input (errors are acceptable).

```bash
cargo +nightly fuzz run fuzz_glob_expansion
```

## Usage

Run a fuzz target (runs indefinitely until interrupted or crash found):

```bash
cd fuzz
cargo +nightly fuzz run fuzz_toml_parser
```

Run for a limited time:

```bash
cargo +nightly fuzz run fuzz_toml_parser -- -max_total_time=60
```

List available targets:

```bash
cargo +nightly fuzz list
```

View coverage (after fuzzing):

```bash
cargo +nightly fuzz coverage fuzz_toml_parser
```

## Corpus

Fuzz corpus is stored in `fuzz/corpus/<target_name>/`. Initial seed inputs can be added there to guide the fuzzer toward interesting cases.

Consider seeding with:
- Valid Cargo.toml files from real projects
- Edge cases (empty files, deeply nested tables, unicode)
- Previously found crash inputs

## CI Integration

These fuzz targets are designed for local development and periodic security audits.
For CI, consider running with time limits:

```bash
cargo +nightly fuzz run fuzz_toml_parser -- -max_total_time=300
cargo +nightly fuzz run fuzz_glob_expansion -- -max_total_time=300
```
