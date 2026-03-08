# CLAUDE.md — depguard-test-util

## Purpose

Shared test utilities for the depguard workspace. Provides JSON normalization for golden-file comparison, ensuring deterministic test output despite non-deterministic fields like timestamps and version numbers.

## Why This Crate Exists

This crate exists because `xtask` needs `normalize_nondeterministic` at runtime (not behind `#[cfg(test)]`), so a `#[cfg(test)]` module inside `depguard-types` would not suffice.

## Public API

```rust
/// Normalize non-deterministic JSON fields for golden-file comparison.
///
/// Two concerns are handled separately:
///
/// 1. **Root-only** — `tool.version` is replaced with `"__VERSION__"` only
///    when the *root* object looks like a report envelope.
///
/// 2. **Recursive** — timestamp keys and `duration_ms` are normalized at any depth.
pub fn normalize_nondeterministic(value: Value) -> Value;
```

## Normalization Rules

| Field | Replacement | Scope |
|-------|-------------|-------|
| `tool.version` | `"__VERSION__"` | Root envelope only |
| `started_at` | `"__TIMESTAMP__"` | Recursive |
| `finished_at` | `"__TIMESTAMP__"` | Recursive |
| `ended_at` | `"__TIMESTAMP__"` | Recursive |
| `duration_ms` | `0` | Recursive |

## Envelope Detection

The function only normalizes `tool.version` when the root object has all five envelope keys:
- `schema`
- `tool`
- `run`
- `verdict`
- `findings`

This prevents false normalization of nested objects that happen to share the same shape.

## Design Constraints

- **Publishable**: `publish = false` — only for internal testing
- **No I/O**: Pure JSON transformation
- **Deterministic**: Same input → same output

## Dependencies

- `serde_json` — JSON value manipulation

## Usage

```rust
use depguard_test_util::normalize_nondeterministic;
use serde_json::from_str;

let report_json = fs::read_to_string("report.json")?;
let mut value: Value = from_str(&report_json)?;
value = normalize_nondeterministic(value);
// Now safe for golden comparison
```

## Testing

```bash
cargo test -p depguard-test-util
```

Tests verify:
- Envelope detection logic
- Recursive timestamp normalization
- Nested data preservation
