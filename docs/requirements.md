# Requirements (operational)

## Must-haves

- Work on:
  - single-crate repos
  - Cargo workspaces, including globbed members
- Support two scopes:
  - `repo`: full scan
  - `diff`: scan only changed manifests (plus root for workspace deps)
- Emit a versioned, machine-readable report (receipt/envelope + findings).
- Produce CI-friendly surfaces:
  - Markdown summary
  - GitHub Actions annotations
- Deterministic output (stable ordering).

## Non-goals (v1)

- Full cargo build graph evaluation (no `cargo metadata` dependency graph enforcement).
- Enforcing feature unification or lockfile policy.
- Solving “supply chain” beyond what can be inferred from manifests.

## Usability constraints

- Fast enough for PR checks: avoid spawning `cargo` unless explicitly asked.
- Fail closed on parsing errors only when configured; otherwise emit a tool/runtime finding.
