# BDD scenarios

This folder is intended for `.feature` files (Gherkin).

Wiring options:
- `cucumber` crate runner (Rust-native Gherkin)
- plain Rust integration tests that interpret a minimal subset of Gherkin

The point is to keep “policy semantics” readable by humans who don't want to read Rust to understand behavior.
