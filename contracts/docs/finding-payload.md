# Finding Payload Spec

Normative shapes for the `data` field in depguard findings. Actuator and buildfix teams dispatch on these payloads to implement automated fixes.

## Per-dependency data shape

Used by 9 of 10 checks. Each finding targets a single dependency in a single manifest.

```json
{
  "current_spec": { "version": "1.0", "path": "../lib" },
  "dependency": "crate-name",
  "fix_action": "<stable_token>",
  "fix_hint": "Human-readable remediation hint",
  "manifest": "crates/foo/Cargo.toml",
  "section": "dependencies",
  "target": "cfg(unix)"
}
```

### Key semantics

- **`current_spec`** — Object containing only non-null keys from the dependency spec. Possible keys: `version`, `path`, `workspace` (bool), `git`, `branch`, `tag`, `rev`, `default-features` (bool), `optional` (bool). Source: `spec_to_json()` in `crates/depguard-domain/src/checks/utils.rs`.
- **`dependency`** — Crate name as it appears in the TOML section.
- **`fix_action`** — Stable machine-readable token for actuator routing. See registry below.
- **`fix_hint`** — Short human-readable hint. Not intended for machine parsing.
- **`manifest`** — Repo-relative path using forward slashes (e.g. `crates/foo/Cargo.toml`).
- **`section`** — One of `dependencies`, `dev-dependencies`, `build-dependencies`. Source: `section_name()` in `crates/depguard-domain/src/checks/utils.rs`.
- **`target`** — Present only for target-specific dependencies. Stores the unquoted TOML key as-is (e.g. `cfg(unix)`, `x86_64-unknown-linux-gnu`). The actuator is responsible for quoting when writing TOML output.

## Workspace-level data shape

Used by `deps.no_multiple_versions` only. The finding is not tied to a single manifest.

```json
{
  "crate": "serde",
  "fix_action": "align_workspace_versions",
  "fix_hint": "Align versions via [workspace.dependencies]",
  "occurrences": [
    ["1.0.195", "crates/a/Cargo.toml", "dependencies"],
    ["1.0.200", "crates/b/Cargo.toml", "dependencies"]
  ],
  "versions": ["1.0.195", "1.0.200"]
}
```

### Key semantics

- **`crate`** — Crate name with multiple versions.
- **`occurrences`** — Array of 3-tuples `[version, manifest, section]`, sorted deterministically (BTreeSet).
- **`versions`** — Deduplicated sorted list of distinct version strings.
- Top-level `current_spec`, `dependency`, `manifest`, `section`, and `target` are **absent** in this shape.

## Fix action token registry

Complete set of stable tokens defined in `crates/depguard-types/src/ids.rs`.

| Token | Check | Actuator action |
|---|---|---|
| `pin_version` | `deps.no_wildcards` | Replace wildcard with pinned semver |
| `add_version` | `deps.path_requires_version` | Add `version` alongside path |
| `use_repo_relative_path` | `deps.path_safety` (absolute_path) | Convert to relative path |
| `remove_parent_escape` | `deps.path_safety` (parent_escape) | Eliminate `..` segments |
| `use_workspace_true` | `deps.workspace_inheritance` | Replace inline spec with `workspace = true` |
| `add_version_with_git` | `deps.git_requires_version` | Add `version` alongside git |
| `move_to_dev_deps` | `deps.dev_only_in_normal` | Move to `[dev-dependencies]` |
| `add_default_features` | `deps.default_features_explicit` | Add explicit `default-features` |
| `align_workspace_versions` | `deps.no_multiple_versions` | Consolidate via `[workspace.dependencies]` |
| `resolve_optional_feature` | `deps.optional_unused` | Add feature ref or remove `optional` |

## Stability rules

- Tokens are **never renamed**. New checks add new tokens.
- Consumers **must tolerate unknown keys** in `data`. Additive keys are non-breaking.
- Dispatch on `fix_action` for routing; ignore unrecognized tokens gracefully.
- The `fix_hint` text may change between releases. Do not parse it programmatically.

## Reference

- Fix action constants: `crates/depguard-types/src/ids.rs`
- Spec serialization: `crates/depguard-domain/src/checks/utils.rs`
- Identity and codes: `contracts/docs/identity-and-codes.md`
