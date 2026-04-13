# depguard Implementation & Roadmap

## Problem
Large policy systems degrade over time unless implementation milestones are explicit and reversible.

## Plan goals
- Keep domain behavior stable while adding checks.
- Preserve byte-stable output for existing contracts.
- Minimize risky migrations.

## Current status

Last reviewed: 2026-04-12

### Completed

- Foundation stabilization: DTO/schema contracts and profile defaults are implemented.
- Policy consolidation: check engine and catalog layers are in place.
- Execution hardening: diff scope, diff-file input, incremental manifest cache, baselines, and yanked checks are implemented.
- Renderer expansion: Markdown, annotations, SARIF, JUnit, JSONL, and report JSON are available.
- Operational maturity: schema emission and fixture/conformance workflows are available via `xtask`.

### In progress
- DOC-01 (`docs/roadmap.md`) — Publish standardized multi-repo rollout guidance and update docs for CI lane behavior.
- DOC-02 (`docs/roadmap.md`) — Improve command-surface documentation for easier onboarding and governance.
- PERF-01 (`docs/roadmap.md`) — Define and enforce baseline performance budgets for incremental and diff runs.

### Planned
- OPS-01 (`docs/roadmap.md`) — Publish an explicit release process and changelog policy.
- Extend task definitions in `docs/tasks.md` with ownership, measurable acceptance criteria, and completion evidence.
- DOC-03 (`docs/roadmap.md`) — Documenting a simplified public command model (`report`, `ci`, `init`, `doctor`) and rollout impact.
- DOC-04 (`docs/roadmap.md`) — Document and evaluate config composition (`extends`, policy bundles, effective config rendering).
- DOC-05 (`docs/roadmap.md`) — Design ratcheted baseline suppressions and review workflows.
- DOC-06 (`docs/roadmap.md`) — Evaluate CI provider adapters and official setup action.
- DOC-07 (`docs/roadmap.md`) — Explore richer reporting metadata and explain/report machine-readable outputs.

## Success criteria
- Golden fixtures updates are minimal and reviewable.
- Every new rule has explainability coverage and tests.
- Exit code semantics remain unchanged unless explicitly planned.

## Risks
- Expanding checks without corresponding fixture and explain coverage.
- Introducing implicit config precedence changes.
- Output format drift without schema verification.
