# Determinism and Truncation

Rules for reproducible output and bounded report sizes.

## Sort order

Findings MUST be sorted deterministically:

1. Severity: `error` → `warn` → `info`
2. Path (lexicographic, forward slashes)
3. Line number (ascending)
4. check_id (lexicographic)
5. code (lexicographic)
6. message (lexicographic)

Missing locations sort last (path = `~`, line = `u32::MAX`).

## Truncation

When findings exceed `max_findings`:

- `data.findings_total` reflects the true count before truncation
- `data.findings_emitted` reflects the count actually included
- `data.truncated_reason` contains a human-readable explanation
- Truncation preserves the deterministic sort order (highest severity first)

## Byte stability

Same inputs MUST produce identical JSON output (modulo timestamps and duration_ms).

- Field order is fixed by serde derive order
- Arrays maintain deterministic sort
- No random or non-deterministic data in output

## Reference

- Sort implementation: `crates/depguard-domain/src/engine.rs` `compare_findings()`
