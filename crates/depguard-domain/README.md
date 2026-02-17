# depguard-domain

Pure dependency policy evaluation engine for depguard.

This is the business-logic core. It evaluates an in-memory workspace model and returns findings, verdict, and summary data.

## Critical Constraint

No filesystem, network, subprocess, stdout/stderr, or CLI dependencies.

## Public API

- `evaluate(model: &WorkspaceModel, cfg: &EffectiveConfig) -> DomainReport`
- Domain model and policy types in `model` and `policy`

## Implemented Checks

- `deps.no_wildcards`
- `deps.path_requires_version`
- `deps.path_safety`
- `deps.workspace_inheritance`
- `deps.git_requires_version`
- `deps.dev_only_in_normal`
- `deps.default_features_explicit`
- `deps.no_multiple_versions`
- `deps.optional_unused`
- `deps.yanked_versions`

## Determinism

Findings are ordered by `severity -> path -> line -> check_id -> code -> message` before truncation.
