# Design notes

## Why microcrates?

The repo has two very different kinds of complexity:

- **Policy logic**: should be small, readable, heavily tested.
- **Parsing + discovery**: inherently messy (TOML, globs, odd workspaces).

Microcrates let us put those complexities in different rooms.

## Determinism as a feature

CI tools live and die by stable diffs.
Depguard enforces determinism via:
- canonical paths (`RepoPath`)
- explicit sort order for findings
- explicit truncation semantics (`max_findings`)

## Failure modes

- Manifest parse errors:
  - Preferred: emit `tool.runtime` finding and continue (best-effort scan)
  - Alternative (strict): treat as hard error and fail early

- Workspace discovery mismatch:
  - Use root Cargo.toml as source of truth
  - Avoid `cargo metadata` in v1 for performance and reproducibility in minimal environments

## Extensibility

- New checks add new `check_id` + `code` constants, and a new module in `depguard-domain`.
- Report schema versioning is explicit. Add v2 schemas as new files; don't mutate v1.
