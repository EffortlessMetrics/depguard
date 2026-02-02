# Architecture

Depguard is split into a **pure evaluation core** and a set of **adapters** that translate real repositories into
an in-memory model. Think “load-bearing wall” vs “drywall”: the domain crate is the wall; everything else can move.

## Data flow

```text
repo on disk / git
     |
     v
depguard-repo  (discover + read + parse Cargo.toml)
     |
     v
depguard-domain (evaluate policy checks -> findings)
     |
     v
depguard-types  (receipt/envelope DTOs)
     |
     v
depguard-render (markdown / annotations / sarif)
     |
     v
depguard-cli    (writes artifacts, exit codes, stdout)
```

The key is the seam between `depguard-repo` and `depguard-domain`: once the input model is built, evaluation
is deterministic and testable without touching the filesystem.

## Crate dependency graph

```text
depguard-cli
  |-- depguard-render -----> depguard-types
  |-- depguard-repo -------> depguard-domain -----> depguard-types
  |-- depguard-settings ---> depguard-domain -----> depguard-types
  `-- depguard-types

xtask (dev tooling) -> depguard-types (schemas) and reads /schemas
```

Rules:
- `depguard-domain` depends on **only** `depguard-types` (plus minimal error types).
- `depguard-repo` may depend on `depguard-domain` to construct the domain model.
- `depguard-cli` is the only place allowed to:
  - call `std::process::Command`
  - write files to disk
  - decide exit codes

## Core abstractions

Depguard is opinionated about what “policy enforcement” means:

- **Input is manifests, not cargo metadata** (no build graph evaluation).
- **Policy is explicit and versioned** (config + profile).
- **Output is a receipt** (envelope + findings + data summary).
- **CI ergonomics are first-class** (Markdown + annotations + stable ordering).

The core model (owned by `depguard-domain`) is intentionally small:

- `WorkspaceModel` — repo root + workspace dependencies + manifests
- `ManifestModel` — path + package metadata + dependency declarations
- `DependencyDecl` — kind + name + spec (version/path/workspace) + location

## Scopes

Depguard supports two scopes (selected by CLI/config):

- `repo` — scan all manifests reachable from the workspace root
- `diff` — scan only manifests affected by a git diff (`--base` / `--head`), plus the root manifest if needed

Scope selection is an **adapter concern** (repo/git). The domain only sees the final manifest set.

## Findings model

A finding is a structured event:

- `check_id` — stable identifier for the check (`deps.no_wildcards`, etc.)
- `code` — stable sub-code for the specific condition (`wildcard_version`, etc.)
- `severity` — `info` / `warning` / `error`
- `location` — best-effort file + line/col
- `message` — human summary
- `help` / `url` — remediation guidance
- `fingerprint` — stable hash for dedup/trending

The emitted report is deterministic:
- canonical path normalization (`RepoPath`)
- stable ordering (path -> check_id -> code -> message)
- optional caps (max findings) with explicit truncation reason

## Where “hexagonal” shows up

The “ports” are deliberately simple: rather than define dozens of traits, the domain expects an in-memory model.
The “adapters” are:
- filesystem + glob expansion + TOML parsing
- git diff scoping (optional)

This is the same hexagonal idea, but with fewer moving parts: a **single port** (“provide a workspace model”) and
multiple adapters that can produce it (real FS, in-memory fixtures, synthetic fuzz inputs).

For details, see `docs/microcrates.md`.
