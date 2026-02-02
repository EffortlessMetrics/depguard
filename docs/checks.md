# Checks catalog

Depguard checks are identified by a stable `check_id` and a stable `code`.

Naming convention:
- `check_id` is a dotted namespace (e.g. `deps.no_wildcards`)
- `code` is a short snake_case discriminator (e.g. `wildcard_version`)

The code registry lives in `crates/depguard-types/src/ids.rs`.

## Implemented in this scaffold

These are implemented as stubs with basic structure:

- `deps.no_wildcards`
  - `wildcard_version` — dependency version is `*` or contains wildcard segments

- `deps.path_requires_version`
  - `path_without_version` — `path = ...` without an explicit `version = ...` where required

- `deps.path_safety`
  - `absolute_path` — `path = "/abs/..."` (or Windows drive roots)
  - `parent_escape` — `path` includes `..` segments escaping the repo root

- `deps.workspace_inheritance`
  - `missing_workspace_true` — member depends on a `[workspace.dependencies]` entry but doesn't opt in

## Adding a new check

1. Add a `check_id` and any `code` constants to `depguard-types::ids`.
2. Implement the check in `depguard-domain::checks`.
3. Add unit tests in the same module.
4. Add an entry to the explain registry (optional, but recommended).
