# depguard-repo

## Problem
Scanning workspaces, respecting changed-file scopes, and normalizing manifest paths are operational concerns that should not leak into policy logic.

## What this crate does
`depguard-repo` is the I/O adapter for workspace discovery and manifest collection.

## Responsibilities
- Discover workspace and manifest candidates
- Resolve manifest paths in a UTF-8-safe way
- Support diff scope workflows from git or file input
- Provide manifests to parser/domain pipeline

## How to use
- Use this crate from CLI/runtime adapters.
- Configure scope (repo vs diff) before domain evaluation.
- Treat all discovered manifests as plain inputs for parsing.

## Constraints
- Deterministic ordering of discovered paths for stable output
- Handles missing/invalid paths as explicit errors
- Keeps policy semantics separate from transport and execution

## Related crates
- `depguard-repo-parser`
- `depguard-app`
- `depguard-cli`
