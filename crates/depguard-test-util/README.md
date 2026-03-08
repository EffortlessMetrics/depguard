# depguard-test-util

Shared test utilities for the depguard workspace.

This internal crate provides deterministic normalization helpers used by tests and `xtask`. It is not published and is intended for workspace-internal use only.

## Purpose

The test-util crate provides:
- JSON normalization for golden file comparison
- Timestamp and version placeholder replacement
- Cross-crate test support utilities

## Key Features

### Non-Deterministic Field Normalization

Golden file tests require deterministic output, but reports contain non-deterministic fields:
- `tool.version` - Changes with each release
- `started_at`, `finished_at`, `ended_at` - Timestamps
- `duration_ms` - Duration values

This crate normalizes these fields to stable placeholder values.

## Public API

```rust
/// Normalize non-deterministic JSON fields for golden-file comparison.
///
/// Handles two concerns separately:
///
/// 1. **Root-only** — `tool.version` is replaced with `"__VERSION__"` only
///    when the root object looks like a report envelope.
///
/// 2. **Recursive** — Timestamp keys and `duration_ms` are normalized at
///    any depth.
pub fn normalize_nondeterministic(value: Value) -> Value;
```

## Usage Example

```rust
use depguard_test_util::normalize_nondeterministic;
use serde_json::json;

// Raw report with non-deterministic fields
let report = json!({
    "schema": "urn:effortless:sensor.report.v1",
    "tool": { "name": "depguard", "version": "0.1.0" },
    "run": { "started_at": "2025-01-01T00:00:00Z", "ended_at": "2025-01-01T00:00:01Z" },
    "verdict": { "pass": true },
    "findings": []
});

// Normalize for golden file comparison
let normalized = normalize_nondeterministic(report);

// Now safe to compare with golden file
assert_eq!(
    normalized["tool"]["version"],
    "__VERSION__"
);
assert_eq!(
    normalized["run"]["started_at"],
    "__TIMESTAMP__"
);
```

## What It Normalizes

| Field | Replacement | Scope |
|-------|-------------|-------|
| `tool.version` | `"__VERSION__"` | Root envelope only |
| `started_at` | `"__TIMESTAMP__"` | Recursive |
| `finished_at` | `"__TIMESTAMP__"` | Recursive |
| `ended_at` | `"__TIMESTAMP__"` | Recursive |
| `duration_ms` | `0` | Recursive |

## Why This Crate Exists

This crate exists as a separate crate (rather than a `#[cfg(test)]` module in `depguard-types`) because `xtask` needs these utilities at runtime, not just during tests.

## Design Constraints

- **Minimal dependencies**: Only `serde_json`
- **Stable placeholder values**: Must not change between versions
- **Envelope-aware**: Only normalizes `tool.version` for actual report envelopes

## Dependencies

| Crate | Purpose |
|-------|---------|
| `serde_json` | JSON value manipulation |

## Related Crates

- [`depguard-types`](../depguard-types/) - Report types being normalized
- `xtask` - Uses normalization for fixture generation
