# depguard Roadmap

## Current status

Last reviewed: 2026-04-12

- Foundation contracts are stable: schema versions, IDs, and report formats.
- Check catalog and check execution paths are implemented.
- Rendering is available for Markdown, annotations, SARIF, JUnit, and JSONL.
- Operational workflows for fixtures/conformance/release tasks exist via `xtask`.
- Focus has shifted from feature discovery to hardening and governance.

## Active roadmap

1. DOC-01: Publish and enforce a standardized multi-repo rollout playbook.
2. PERF-01: Introduce explicit performance budgets for incremental and diff scope runs.
3. DOC-02: Improve command surface documentation and onboarding model for easier adoption.

## Completed

- QA-01: Keep roadmap and task tracking updated with status and ownership.

## Planned direction

- DOC-03: Simplify the public command model around `depguard check`, grouped `depguard report`, `depguard ci`, `depguard init`, and `depguard doctor`.
- DOC-04: Add config composition and policy bundles for org-level policy inheritance.
- DOC-05: Add ratcheted baseline suppressions with ownership, expiry, and evidence.
- DOC-06: Add first-class CI provider adapters as a replacement for hand-written YAML snippets.
- DOC-07: Enrich findings and explain/report payloads for better bot and editor integrations.

## Near-term guardrails

- Keep all CLI behavior changes paired with updated docs and fixture coverage.
- Treat output-contract changes as release-impacting unless tested and justified.
- Refresh this roadmap whenever `docs/implementation-plan.md` or `docs/tasks.md` changes.

## How this page is maintained

- Quarterly: review completed items, adjust priorities.
- Monthly: validate open items against open issues and CI needs.
- Ongoing: update links when commands, output modes, or release workflows change.
