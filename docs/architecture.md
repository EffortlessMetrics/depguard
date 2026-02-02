# depguard — Architecture (Hexagonal / Clean)

This document describes *how the codebase is structured* to preserve boundaries, determinism, and testability.

## Architectural stance

depguard is a **repo-truth sensor** that must stay:

- deterministic (same inputs → same outputs)
- cheap (fast lane viable)
- offline (no network)
- decoupled from orchestration (cockpit director composes; depguard just senses)

To preserve those properties, depguard uses a **hexagonal / clean architecture** split:

- **Domain**: pure rules and policy evaluation
- **Application**: use cases orchestration
- **Adapters**: filesystem, git, rendering, CLI wiring

## Data flow

```
CLI -> App(use case) -> Repo discovery -> Manifest parsing -> Dependency walk
    -> Domain checks -> Findings + Verdict -> Receipt writer
    -> Optional renderers: Markdown / Annotations
```

## Ports (interfaces)

These should live in application layer or a dedicated `*-ports` module:

- `RepoReader`
  - read bytes/text for a repo-relative path
- `WorkspaceDiscoverer`
  - enumerate manifest paths (root + members)
- `DiffProvider`
  - list changed files between base/head (diff-scope selection)
- `Clock`
  - provide timestamps
- `Writer`
  - write artifacts (report.json, comment.md)
- `Logger` (optional)
  - capture structured log lines to raw.log if enabled

## Adapters

- `fs_repo`
  - implements `RepoReader` and `Writer`
- `git_diff`
  - implements `DiffProvider` (start with shell-out `git diff --name-only`; make it injectable)
- `workspace_globs`
  - implements `WorkspaceDiscoverer`
- `render_md`
  - markdown rendering from report DTO
- `render_gh_annotations`
  - GitHub workflow command render from report DTO

## Domain layer: rules and invariants

Domain code must not:
- touch filesystem
- shell out
- depend on clap
- log to stdout

It receives:
- parsed/normalized manifests and dependency entries
- effective config (profile + overrides)

It returns:
- findings (with stable ordering)
- verdict computation (status + counts + reasons)

### Deterministic ordering utility

Centralize ordering in one function to avoid drift:

- severity desc (error > warn > info)
- manifest path lexical
- line asc (missing last)
- check_id lexical
- code lexical
- message lexical

All renderers must reuse the same ordering.

## Microcrate workspace layout

This structure keeps compile times sane and boundaries clear:

- `depguard-types`
  - DTOs: config, report, findings
  - schema ids/constants
  - stable codes/check IDs and explain metadata types

- `depguard-domain`
  - rule implementations
  - policy evaluation and verdict computation
  - ordering and normalization utilities

- `depguard-repo`
  - workspace discovery + manifest loading
  - TOML parsing and extraction into normalized structures
  - diff-scope selection helper

- `depguard-render`
  - Markdown renderer
  - GitHub annotations renderer

- `depguard-app`
  - use cases:
    - `check`
    - `md`
    - `annotations`
    - `explain`
  - converts inputs -> domain -> outputs
  - owns “what happens on error” classification (tool error vs skip/warn)

- `depguard-cli`
  - clap wiring
  - filesystem paths and defaults
  - exit code mapping

- `xtask`
  - schema emission/validation tasks
  - fixture generation helpers
  - release helper tasks

## Receipt contract

depguard emits `depguard.report.v1` (envelope-compliant). Canonical output path:

- `artifacts/depguard/report.json`

Tool-specific fields must remain under `data` only.

### Finding identity

- `check_id`: stable producer identity
- `code`: stable classification string

This allows:
- explain lookup
- dedupe in cockpit director
- future actuator mapping without coupling to depguard internals

## Conformance and drift prevention

depguard CI must enforce:

1. schema validation for emitted receipts (`schemas/depguard.report.v1.json` + envelope)
2. golden tests for deterministic outputs
3. explain coverage for every emitted (check_id, code)
4. fuzz targets do not panic
5. mutation testing for domain logic

## Observability (optional)

depguard is a gatekeeper; logs must be minimal and useful:

- structured logs can be written to `artifacts/depguard/raw.log` if enabled
- receipt is the source of truth; logs are debugging aids only

## Security posture

- no network access
- no execution of arbitrary repo code
- if shelling out to git: use fixed args and avoid shell parsing
- handle malicious TOML content robustly (fuzzing)
