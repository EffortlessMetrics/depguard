# depguard-test-util

Shared test utilities for the depguard workspace.

This internal crate currently provides deterministic normalization helpers used by tests and `xtask`.

## Public API

- `normalize_nondeterministic(value: serde_json::Value) -> serde_json::Value`

## What It Normalizes

- Root report `tool.version` placeholder replacement for envelope-shaped JSON
- Timestamp fields (`started_at`, `finished_at`, `ended_at`) at any depth
- `duration_ms` at any depth

This crate is `publish = false` and intended for workspace-internal tooling/tests.
