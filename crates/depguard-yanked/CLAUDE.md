# CLAUDE.md — depguard-yanked

## Purpose

Parses and queries offline yanked-version indexes used by `deps.yanked_versions` check.

This crate is IO-free and deterministic:
- Input: index text
- Output: `YankedIndex` model with exact crate/version lookups

## Supported Formats

### JSON map
```json
{ "serde": ["1.0.188", "1.0.189"], "tokio": ["1.37.0"] }
```

### JSON array
```json
[
  { "crate": "serde", "version": "1.0.188" },
  { "crate": "serde", "version": "1.0.189" }
]
```

### Line format
```text
serde 1.0.188
serde@1.0.189
tokio 1.37.0
```

## Public API

```rust
/// Parse yanked version index from text
pub fn parse_yanked_index(input: &str) -> anyhow::Result<YankedIndex>;

/// Parsed yanked version index
pub struct YankedIndex {
    // Internal map of crate → versions
}

impl YankedIndex {
    /// Check if a specific crate version is yanked
    pub fn is_yanked(&self, crate_name: &str, version: &str) -> bool;
    
    /// Get all yanked versions for a crate
    pub fn yanked_versions(&self, crate_name: &str) -> Option<&[String]>;
}
```

## Design Constraints

- **No filesystem access**: Takes string input
- **No network access**: Offline only
- **Deterministic**: Same input → same output
- **Stable errors**: Parsing errors are consistent and actionable

## Usage in depguard

The `deps.yanked_versions` check uses this crate:

1. User provides `--yanked-index <path>` CLI flag
2. CLI reads file and passes contents to `parse_yanked_index()`
3. Index is stored in `EffectiveConfig.yanked_index`
4. Check calls `is_yanked()` for each dependency version

## Dependencies

| Dependency | Purpose |
|------------|---------|
| `anyhow` | Error handling |
| `serde` | JSON deserialization |
| `serde_json` | JSON parsing |

## Testing

```bash
cargo test -p depguard-yanked
```

Tests cover:
- All input formats
- Malformed input handling
- Empty input
- Case sensitivity
- Version matching semantics

## Generating Yanked Indexes

Yanked indexes are typically generated externally (e.g., by querying crates.io). The format is simple enough to generate with a script:

```bash
# Example: generate from crates.io API
curl -s 'https://crates.io/api/v1/crates/serde/versions' | \
  jq -r '.versions[] | select(.yanked) | "serde \(.num)"' >> yanked.txt
```
