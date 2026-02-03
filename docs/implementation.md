# Implementation notes

This is the recommended sequencing for development:

## Build order

1. **Types + schema IDs** (`depguard-types`)
   - Lock down the envelope/report shape early
   - Add ordering helpers
   - Define stable IDs and explanations

2. **Config model + presets** (`depguard-settings`)
   - Parse TOML config
   - Implement profile presets
   - Merge with CLI overrides
   - Produce `EffectiveConfig`

3. **Domain checks** (`depguard-domain`)
   - Implement checks against an in-memory `WorkspaceModel`
   - Unit tests + property tests
   - Keep this crate pure (no I/O)

4. **Repo adapter** (`depguard-repo`)
   - Workspace discovery
   - Manifest parse into domain model
   - Fixtures + fuzz harnesses

5. **Renderers** (`depguard-render`)
   - Markdown + GitHub annotations
   - Golden snapshot tests

6. **App layer** (`depguard-app`)
   - Use case orchestration
   - Wire domain + repo + settings + render
   - Report serialization

7. **CLI glue** (`depguard-cli`)
   - Wire everything
   - Artifacts + exit codes
   - Integration tests

8. **xtask**
   - Schema generation
   - Fixture update automation
   - Release packaging

## Key decisions

### Why `toml_edit` over `toml`?

`toml_edit` preserves span information (byte offsets), allowing us to report precise line numbers for findings. The `toml` crate parses into values without location tracking.

### Why no `cargo metadata`?

Performance and reproducibility:
- `cargo metadata` requires resolving the entire dependency graph
- It spawns cargo, which may trigger downloads or builds
- We only need manifest-level information, not the resolved graph

### Why profiles instead of per-check defaults?

Profiles provide opinionated bundles that are easy to communicate:
- "Use `strict` for new projects"
- "Use `compat` while migrating legacy code"

Individual check overrides are still available for fine-tuning.

### Why stable ordering?

CI tools need deterministic output for:
- Meaningful diffs in PR comments
- Stable fingerprints for dedup/trending
- Reproducible test assertions

Ordering: `severity → path → line → check_id → code → message`

## Crate boundaries

The hexagonal architecture enforces clear boundaries:

| Boundary | Enforced by |
|----------|-------------|
| Domain is pure | No filesystem/network deps in `depguard-domain` |
| Settings are strings | `parse_config_toml()` takes `&str`, not paths |
| Repo is the I/O adapter | Only `depguard-repo` reads from filesystem |
| CLI is the shell adapter | Only `depguard-cli` runs subprocesses |

These boundaries make the codebase testable at each layer.
