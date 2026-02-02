# depguard (workspace scaffold)

Depguard is a Rust dependency policy linter designed for **CI** and **monorepos**:
it inspects `Cargo.toml` manifests (workspaces and single crates), evaluates them against
an explicit policy, and emits a versioned receipt-style report suitable for:
- GitHub Actions annotations
- PR comments (Markdown)
- artifact storage for audit / trend analysis

This repository is structured as a **microcrate workspace**: each crate owns a small, stable surface area.
The intent is simple: keep the policy engine testable, keep IO replaceable, and keep the CLI thin.

## Workspace layout

```text
crates/
  depguard-types     Stable DTOs + receipt envelope + finding codes
  depguard-settings  Config model + profile/preset resolution
  depguard-domain    Pure policy evaluation (checks) + deterministic finding ordering
  depguard-repo      Workspace discovery + Cargo.toml parsing + diff scoping adapters
  depguard-render    Markdown / GitHub annotations renderers
  depguard-cli       CLI binary (the only crate that talks to the outside world)
xtask/               Dev tooling: schema generation, fixture updates, release packaging
schemas/             Versioned JSON Schemas for emitted artifacts
docs/                Design + operating notes
```

## Design rules (the important ones)

- **Domain has no IO**: `depguard-domain` takes an in-memory model and returns findings.
- **Adapters are swappable**: filesystem/git live in `depguard-repo`.
- **DTOs are stable**: receipt/envelope types live in `depguard-types`, with versioned schema IDs.
- **Deterministic output**: sorting and capping rules are explicit so CI diffs are stable.

## Quick start (when implemented)

```bash
cargo run -p depguard-cli -- check --scope=repo
cargo run -p depguard-cli -- check --scope=diff --base origin/main --head HEAD
```

## Documentation

- `docs/architecture.md` — overall flow and boundaries
- `docs/microcrates.md` — crate-by-crate contracts
- `docs/testing.md` — BDD + fuzzing + mutation + property tests
- `docs/checks.md` — check catalog and code registry

---
**Note:** This is a scaffold with compile-friendly stubs. It is intended as a starting point for implementation.
