# Fuzz targets (placeholder)

This directory is scaffolded for `cargo-fuzz`.

Suggested targets:
- `fuzz_manifest_parse.rs` — random TOML into manifest parser (should never panic)
- `fuzz_workspace_discovery.rs` — random workspace tables + globs (should never panic)

Create with:
```bash
cargo install cargo-fuzz
cargo fuzz init
```
