# Roadmap Task Board

## Purpose
This page tracks near-term roadmap items and maintenance work.

Last reviewed: 2026-04-13

## Active initiatives

| ID | Area | State | Owner | Target | Notes |
|---|---|---|---|---|---|
| DOC-01 | Documentation | In progress | Unassigned | 2026-Q2 | Publish standardized multi-repo rollout guidance and align docs |
| DOC-02 | Documentation | Planned | Unassigned | 2026-Q3 | Define clearer onboarding and command-surface documentation |
| DOC-03 | Documentation | Planned | Unassigned | 2026-Q4 | Draft simplified command model and phased adoption guidance |
| DOC-04 | Configuration | Planned | Unassigned | 2026-Q4 | Draft config composition and policy bundle documentation |
| DOC-05 | Governance | Planned | Unassigned | 2027-Q1 | Draft ratchet baseline model and owner/expiry process |
| DOC-06 | CI | In progress | Unassigned | 2027-Q1 | Finalize provider-aware `ci` mode and reusable workflow; evaluate dedicated setup action as optional install optimization |
| DOC-07 | Reporting | Planned | Unassigned | 2027-Q1 | Draft findings metadata extensions for bots/editors |
| PERF-01 | Performance | Planned | Unassigned | 2026-Q2 | Add measurable benchmarks/budgets for incremental and diff scans |
| OPS-01 | Release/process | Planned | Unassigned | 2026-Q2 | Publish release process and changelog policy |
| QA-01 | Governance | Completed | Unassigned | 2026-Q2 | Keep roadmap and task list refreshed per behavior change |

## Completion definition
- Behavior changes include tests and fixture updates.
- Contract changes include schema and fixture verification.
- Documentation includes behavior references and examples.
- No unchecked regressions in output shape.

## Recurring maintenance
- Validate schema and fixture sync.
- Refresh explain coverage.
- Review and prune stale allow-lists.
- Reconfirm CI defaults and diff-scope assumptions against current workflows.
