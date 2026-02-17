# CLAUDE.md — depguard-yanked

## Purpose

Parses and queries offline yanked-version indexes used by `deps.yanked_versions`.

This crate is IO-free and deterministic:
- Input: index text
- Output: `YankedIndex` model with exact crate/version lookups

## Supported formats

- JSON map:
  ```json
  { "serde": ["1.0.188", "1.0.189"] }
  ```
- JSON array:
  ```json
  [{ "crate": "serde", "version": "1.0.188" }]
  ```
- Line format:
  ```text
  serde 1.0.188
  tokio@1.37.0
  ```

## Public API

```rust
pub struct YankedIndex;
pub fn parse_yanked_index(input: &str) -> anyhow::Result<YankedIndex>;
impl YankedIndex {
    pub fn is_yanked(&self, crate_name: &str, version: &str) -> bool;
}
```

## Design constraints

- No filesystem access
- No network access
- Preserve deterministic behavior and stable parsing errors
