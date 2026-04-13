# depguard Output Contract

## Problem
Without a clear output contract, consumers cannot reliably parse, compare, or persist depguard results.

## Canonical artifacts
- `depguard.report.v1.json` and `depguard.report.v2.json` in `schemas/`.
- `depguard.baseline.v1.json` for baseline files.
- Legacy envelope compatibility is retained where documented.

## Report shape (minimum)
- `schema` — schema identifier.
- `tool` — invoker metadata.
- `run` — execution metadata.
- `verdict` — status + counts.
- `findings` — ordered finding events.

## Finding fields (high-level)
- `severity`, `check_id`, `code`, `location`, `message`, optional `help/url`, optional `data`, optional `fingerprint`.
- `location` includes path/line for actionable edits.
- `data` carries check-specific details where available.

## Ordering contract
`severity -> path -> line -> check_id -> code -> message`

## Determinism requirements
- Canonical paths.
- Stable ordering.
- Explicit capping (`max_findings`) and truncation indicators.

## Consumption guidance
- Use `depguard report md` for human review.
- Use `depguard report sarif` for GitHub code scanning and third-party integrations.
- Use `depguard report jsonl` for log pipelines.
- Use `depguard report annotations` for inline GitHub annotations.
- Use `depguard report junit` for CI test result ingest.
- Legacy renderer commands (`depguard md`, etc.) remain supported.

## Related docs
- `docs/architecture.md`
- `docs/checks.md`
- `docs/quickstart.md`
