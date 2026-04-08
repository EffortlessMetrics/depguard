# depguard Implementation Plan

## Problem
Large policy systems degrade over time unless implementation milestones are explicit and reversible.

## Plan goals
- Keep domain behavior stable while adding checks.
- Preserve byte-stable output for existing contracts.
- Minimize risky migrations.

## Phased approach

1. **Foundation cleanup**
   - Stabilize shared DTOs and schema IDs.
   - Tighten parse error handling in `depguard-repo-parser`.
2. **Core policy consolidation**
   - Add/maintain checks in `depguard-domain-checks`.
   - Keep catalog and metadata synchronized in `depguard-check-catalog`.
3. **Execution hardening**
   - Improve workspace discovery and diff-scope determinism.
   - Expand conformance coverage.
4. **Renderer expansion**
   - Maintain rendering consistency across markdown, annotations, SARIF, JUnit, JSONL.
5. **Operational maturation**
   - Add automation in `xtask` for schema emission and release checks.

## Success criteria
- Golden fixtures update minimally for each phase.
- New behavior has explainability coverage.
- Exit code semantics remain unchanged unless explicitly planned.

## Risks
- Expanding checks without corresponding fixture and explain coverage.
- Introducing implicit config precedence changes.
- Output format drift without schema verification.

## Current workflow recommendation
Use feature flags per check for incremental rollout and keep each PR scoped to one or two adjacent phases.
