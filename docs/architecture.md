# Architecture

> **Navigation**: [Quick Start](quickstart.md) | [Configuration](config.md) | [Checks](checks.md) | [CI Integration](ci-integration.md) | Architecture | [Design](design.md) | [Testing](testing.md)

Depguard uses **hexagonal (ports & adapters)** architecture with a **pure evaluation core** and a set of **adapters** that translate real repositories into an in-memory model. Think "load-bearing wall" vs "drywall": the domain crate is the wall; everything else can move.

## Crate overview

| Crate | Purpose |
|-------|---------|
| `depguard-types` | DTOs, config, report, findings; schema IDs; stable codes |
| `depguard-domain` | Rule implementations; policy evaluation (pure, no I/O) |
| `depguard-settings` | Config parsing; profile presets; override resolution |
| `depguard-repo` | Workspace discovery; manifest loading; TOML parsing; diff-scope |
| `depguard-render` | Markdown and GitHub annotations renderers |
| `depguard-app` | Use cases: check, md, annotations, explain; error handling |
| `depguard-cli` | clap wiring; filesystem paths; exit code mapping |
| `xtask` | Schema emission; fixture generation; release tasks |

## Data flow

```text
repo on disk / git
     │
     ▼
depguard-cli        (arg parsing, config file read, git diff call)
     │
     ▼
depguard-app        (use case orchestration)
     │
     ├─────────────────────────────────────────┐
     │                                         │
     ▼                                         ▼
depguard-settings                      depguard-repo
(parse config TOML,                    (discover manifests,
 resolve presets,                       parse TOML with locations,
 merge overrides)                       build WorkspaceModel)
     │                                         │
     ▼                                         ▼
EffectiveConfig ───────────────────► depguard-domain
                                     (evaluate policy checks)
                                              │
                                              ▼
                                     DomainReport (findings + verdict)
                                              │
                                              ▼
                                     depguard-types (wrap in envelope)
                                              │
                                              ▼
                                     depguard-render (markdown / annotations)
                                              │
                                              ▼
                                     depguard-cli (write artifacts, exit code)
```

The key seam is between `depguard-repo` and `depguard-domain`: once the input model is built, evaluation is deterministic and testable without touching the filesystem.

## Crate dependency graph

```text
depguard-cli
  └── depguard-app (use cases)
        ├── depguard-render ────────► depguard-types
        ├── depguard-repo ──────────► depguard-domain ────► depguard-types
        ├── depguard-settings ──────► depguard-domain ────► depguard-types
        └── depguard-types

xtask (dev tooling) ─► depguard-types, depguard-settings (schemas)
```

**Rules:**
- `depguard-domain` depends on **only** `depguard-types` (plus minimal error types like `thiserror`).
- `depguard-repo` depends on `depguard-domain` to construct the domain model.
- `depguard-settings` depends on `depguard-domain` for `EffectiveConfig` and policy types.
- `depguard-app` orchestrates use cases but delegates I/O to callers.
- `depguard-cli` is the only place allowed to:
  - call `std::process::Command` (for `git diff`)
  - read/write files to disk
  - decide exit codes

## Core abstractions

Depguard is opinionated about what "policy enforcement" means:

- **Input is manifests, not cargo metadata** (no build graph evaluation).
- **Policy is explicit and versioned** (config + profile).
- **Output is a receipt** (envelope + findings + data summary).
- **CI ergonomics are first-class** (Markdown + annotations + stable ordering).

The core model (owned by `depguard-domain`) is intentionally small:

| Type | Purpose |
|------|---------|
| `WorkspaceModel` | Repo root + workspace dependencies + manifests |
| `ManifestModel` | Path + package metadata + dependency declarations |
| `DependencyDecl` | Kind (normal/dev/build) + name + spec + location |
| `DepSpec` | Version string + path + workspace flag |
| `EffectiveConfig` | Resolved config with profile, scope, fail_on, per-check policies |

## Scopes

Depguard supports two scopes (selected by CLI/config):

| Scope | Behavior |
|-------|----------|
| `repo` | Scan all manifests reachable from the workspace root |
| `diff` | Scan only manifests affected by a git diff (`--base`/`--head`), plus root for workspace deps |

Scope selection is an **adapter concern** (repo/git). The domain only sees the final manifest set.

## Findings model

A finding is a structured event:

| Field | Purpose |
|-------|---------|
| `check_id` | Stable identifier for the check (`deps.no_wildcards`, etc.) |
| `code` | Stable sub-code for the specific condition (`wildcard_version`, etc.) |
| `severity` | `info` / `warning` / `error` |
| `location` | Best-effort file + line/col |
| `message` | Human summary |
| `help` / `url` | Remediation guidance |
| `fingerprint` | Stable hash for dedup/trending |
| `data` | Check-specific structured payload (JSON) |

The emitted report is deterministic:
- Canonical path normalization (`RepoPath`)
- Stable ordering: `severity → path → line → check_id → code → message`
- Optional caps (`max_findings`) with explicit truncation reason

## Where "hexagonal" shows up

The "ports" are deliberately simple: rather than define dozens of traits, the domain expects an in-memory model. The "adapters" are:
- Filesystem + glob expansion + TOML parsing (`depguard-repo`)
- Git diff scoping (`depguard-cli` → `depguard-repo`)
- Config file parsing (`depguard-settings`)

This is the same hexagonal idea, but with fewer moving parts: a **single port** ("provide a workspace model and config") and multiple adapters that can produce it (real FS, in-memory fixtures, synthetic fuzz inputs).

For crate-level contracts, see [microcrates.md](microcrates.md).

## See also

- [Design Notes](design.md) — Design decisions and rationale
- [Microcrates](microcrates.md) — Crate-by-crate contracts
- [Testing](testing.md) — Test strategy and organization
- [Implementation Plan](implementation-plan.md) — Development roadmap
