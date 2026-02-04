# Design Notes

> **Navigation**: [Architecture](architecture.md) | Design | [Microcrates](microcrates.md) | [Implementation Plan](implementation-plan.md) | [Testing](testing.md)

## Why microcrates?

The repo has two very different kinds of complexity:

- **Policy logic**: should be small, readable, heavily tested.
- **Parsing + discovery**: inherently messy (TOML, globs, odd workspaces).

Microcrates let us put those complexities in different rooms. The domain crate can be mutation-tested and fuzzed independently of the filesystem adapter.

## Determinism as a feature

CI tools live and die by stable diffs. Depguard enforces determinism via:

| Mechanism | Purpose |
|-----------|---------|
| `RepoPath` | Canonical paths (forward slashes, repo-relative) |
| Explicit sort order | `severity → path → line → check_id → code → message` |
| Truncation semantics | `max_findings` with explicit `truncated_reason` in output |
| No floating timestamps | Timestamps captured once at start/end, not per-finding |

## Pure domain layer

The `depguard-domain` crate is intentionally pure:

- No filesystem access
- No network calls
- No stdout/stderr
- No clap dependencies
- All inputs via function parameters
- All outputs via return values

This makes the domain:
- Easy to unit test with synthetic inputs
- Safe to fuzz without side effects
- Deterministic regardless of environment

## Check architecture

Each check is a module in `depguard-domain/src/checks/` with a `run()` function:

```rust
pub fn run(
    manifest: &ManifestModel,
    workspace_deps: &HashMap<String, DepSpec>,
    policy: &CheckPolicy,
) -> Vec<Finding>
```

Checks:
- Receive the manifest and relevant context
- Return zero or more `Finding` objects
- Never panic (return errors as findings if needed)
- Are stateless and composable

The engine orchestrates checks:
1. Run all enabled checks
2. Collect findings into a single vector
3. Sort deterministically
4. Truncate to `max_findings`
5. Compute verdict based on severities and `fail_on` setting

## Configuration resolution

Config resolution follows a three-layer precedence model:

```
CLI flags (highest) → Config file → Profile preset (lowest)
```

Resolution happens once at startup:
1. Load preset defaults for the selected profile
2. Overlay config file values (if present)
3. Overlay CLI flag values
4. Produce `EffectiveConfig` passed to domain

This keeps the domain simple—it sees only the final, resolved config.

## Failure modes

**Manifest parse errors:**
- Default: emit `tool.runtime` finding and continue (best-effort scan)
- Alternative (future): treat as hard error and fail early via config flag

**Workspace discovery mismatch:**
- Use root `Cargo.toml` as source of truth
- Avoid `cargo metadata` for performance and reproducibility

**Git diff failures:**
- Missing git → tool error (exit 1) with remediation message
- Shallow clone issues → tool error with suggestion to fetch more history

## Extensibility

### Adding a new check

1. Add `check_id` and `code` constants to `depguard-types/src/ids.rs`
2. Add explanation entry to `depguard-types/src/explain.rs`
3. Implement check in `depguard-domain/src/checks/<name>.rs`
4. Wire into `checks/mod.rs` and engine
5. Add unit tests + BDD scenarios
6. Update `docs/checks.md`

### Adding a new output format

1. Add renderer function to `depguard-render`
2. Add use case wrapper to `depguard-app`
3. Add CLI subcommand to `depguard-cli`
4. Add golden snapshot tests

### Schema versioning

Report schema versioning is explicit:
- Add v2 schemas as new files; don't mutate v1
- Envelope schema is vendored (external contract)
- Use `schema` field in output to indicate version

## See also

- [Architecture](architecture.md) — System overview and data flow
- [Microcrates](microcrates.md) — Crate contracts and APIs
- [Checks Catalog](checks.md) — Available checks
- [Testing](testing.md) — Test strategy
