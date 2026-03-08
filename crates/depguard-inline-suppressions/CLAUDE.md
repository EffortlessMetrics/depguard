# CLAUDE.md — depguard-inline-suppressions

## Purpose

Inline suppression parsing for depguard. Parses `# depguard: allow(...)` comments in Cargo.toml files to suppress specific findings.

## Supported Syntax

### Inline comment on same line as dependency
```toml
serde = "*" # depguard: allow(deps.no_wildcards)
```

### Standalone comment above dependency
```toml
# depguard: allow(no_wildcards, wildcard_version)
serde = "*"
```

### Multiple suppressions
```toml
# depguard: allow(deps.no_wildcards, deps.path_safety)
my-path-dep = { path = "../lib" }
```

## Public API

```rust
/// Parse inline suppression tokens for a dependency declaration line.
///
/// Returns a sorted, deduplicated list of suppression tokens.
/// Each token can be either:
/// - A check ID (e.g., `deps.no_wildcards`)
/// - A code (e.g., `wildcard_version`)
pub fn parse_inline_suppressions(source: &str, line: u32) -> Vec<String>;
```

## Token Normalization

Short tokens are normalized to full check IDs:

| Input | Normalized |
|-------|------------|
| `no_wildcards` | `deps.no_wildcards` |
| `wildcard_version` | `wildcard_version` (unchanged - it's a code) |
| `deps.path_safety` | `deps.path_safety` (unchanged - already full ID) |

Normalization looks up tokens in the explanation registry to verify valid check IDs.

## Design Constraints

- **No I/O**: Pure string parsing
- **No panics**: Malformed input returns empty vec
- **Deterministic**: Same input → same output
- **Line-based**: Uses 1-based line numbers

## Parsing Rules

1. Look for `# depguard: allow(...)` pattern
2. Split contents on `,` and trim whitespace
3. Normalize each token
4. Walk upward to find contiguous comment lines above the target line
5. Merge and deduplicate all suppression tokens

## Dependencies

- `depguard-types` — Explanation registry for token normalization

Dev dependencies:
- `proptest` — Property-based testing

## Testing

```bash
cargo test -p depguard-inline-suppressions
```

Tests cover:
- Inline comment parsing
- Above-line comment parsing
- Multi-line comment blocks
- Token normalization
- Property tests for fuzzing resilience

## Fuzzing

This crate is fuzzed via `fuzz/fuzz_targets/fuzz_inline_suppressions.rs` to ensure no panics on arbitrary input.
