# depguard Output Contract

## Problem
Without a clear output contract, consumers cannot reliably parse, compare, or persist depguard results.

## Canonical artifacts
- `depguard.report.v1.json` and `depguard.report.v2.json` in `schemas/`.
- Legacy envelope compatibility is retained where documented.

## Report shape (minimum)
- `schema` — schema identifier.
- `tool` — invoker metadata.
- `run` — execution metadata.
- `verdict` — status + counts.
- `findings` — ordered finding events.

## Finding fields (high-level)
- `severity`, `check_id`, `code`, `location`, `message`, optional `help/url`, optional `data`.
- `location` includes path/line for actionable edits.

## Ordering contract
`severity -> path -> line -> check_id -> code -> message`

## Determinism requirements
- Canonical paths.
- Stable ordering.
- Explicit capping (`max_findings`) and truncation indicators.

## Consumption guidance
- Use `depguard md` for human review.
- Use `depguard sarif` for GitHub code scanning and third-party integrations.
- Use `depguard jsonl` for log pipelines.

## Related docs
- `docs/architecture.md`
- `docs/checks.md`
- `docs/quickstart.md`
