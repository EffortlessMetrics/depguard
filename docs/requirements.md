# depguard — Requirements

## Purpose

depguard is a **repo-truth dependency manifest hygiene sensor** for Rust workspaces.

It answers one question:

> “Are `Cargo.toml` manifests hygienic and drift-resistant (in ways reviewers routinely miss)?”

It is explicitly **not** a vulnerability scanner, not a dependency resolver, and not a build tool.
It emits **versioned receipts** for cockpit ingestion and optional PR-friendly renderings.

## Goals

1. **Catch high-frequency manifest mistakes**
   - wildcard versions (`*`, `1.*`)
   - path dependencies missing a `version` (publishing hygiene)
   - unsafe path dependencies (absolute paths or paths escaping workspace root)
   - workspace dependency drift (members re-specifying versions when workspace defines a dependency)

2. **Stay deterministic, fast, and offline**
   - repo truth only: reads repo files; no builds/tests; no network
   - stable ordering and stable output bytes for identical inputs

3. **Be adoptable with real-world debt**
   - profiles: `oss | team | strict`
   - scope: `repo | diff` (ratchet adoption path)
   - allowlists and escape hatches (configurable, explicit)

4. **Emit stable receipts (protocol discipline)**
   - canonical artifact path: `artifacts/depguard/report.json`
   - stable schema id and stable code set
   - strict envelope with a single extension point (`data`)

5. **Test posture appropriate for a gatekeeper**
   - fixture-driven golden outputs
   - BDD scenarios
   - proptest for spec-shape combinatorics and ordering invariants
   - fuzz targets for parser and discovery robustness
   - mutation testing on domain rules

## Non-goals

depguard must **not**:

- resolve dependency graphs (do not become Cargo)
- run `cargo metadata` in the default path (optional opt-in later is acceptable)
- run builds/tests/coverage/benchmarks
- perform vulnerability/license scanning (ingest `cargo-deny` / `cargo-audit` later via adapters)
- modify the repo (actuation belongs in buildfix/buildfix-style actuator)
- require network access

## Users and user stories

### Primary users
- Maintainers of Rust workspaces/monorepos
- CI owners who want “cheap, deterministic PR gates”
- Reviewers who want less release-day surprise

### Representative stories
- “We added a path dependency. Make sure it won’t break publishing later.”
- “We touched a manifest. Enforce our basic hygiene without scanning the whole repo.”
- “We centralize versions in `[workspace.dependencies]`. Prevent drift.”
- “Show me a short PR summary and link to details, without noise.”

## Inputs

### Required
- `--root` (repo root; defaults to current working dir)
- root `Cargo.toml`

### Optional (depending on mode)
- `--config depguard.toml`
- `--profile oss|team|strict`
- `--scope repo|diff`
- `--base` and `--head` (for diff-scope selection)
- optional allow/deny lists for checks and path patterns

## Outputs

### Canonical artifacts
- `artifacts/depguard/report.json` (**required**) — `depguard.report.v1` receipt, envelope-compliant

### Optional artifacts
- `artifacts/depguard/comment.md` — PR-friendly summary (capped)
- annotations output (GitHub workflow commands) rendered from receipt (capped)

## CLI surface

Commands:
- `depguard check`
- `depguard md --report <report.json>`
- `depguard annotations --report <report.json>`
- `depguard explain <check_id|code>`

Flags (stable / ecosystem-aligned):
- `--root <path>`
- `--config <path>`
- `--profile oss|team|strict`
- `--scope repo|diff`
- `--base <rev>` `--head <rev>`
- `--out <path>` (default: `artifacts/depguard/report.json`)
- `--md <path>` (optional)
- `--max-findings <n>` (for receipt surface caps; artifacts can still include full details)

## Exit code semantics

- `0` — OK (pass or warn unless warn-as-fail configured)
- `2` — policy failure (blocking findings, or warn-as-fail)
- `1` — tool/runtime error (I/O, parse errors, invalid config, unexpected failures)

## Profiles and scope

Profiles provide **default severity + enablement**. They are applied as a single “effective config” step,
not scattered `if profile == ...` branches throughout checks.

- `oss` (default)
  - warn-heavy
  - skip opinionated checks (workspace inheritance) by default
  - never fail because “house convention files” are missing

- `team`
  - hygiene checks block
  - inheritance may warn or block depending on config

- `strict`
  - everything enabled and blocking unless explicitly downgraded

Scope controls adoption posture:
- `repo` — analyze all manifests (default for mature repos)
- `diff` — analyze only changed manifests between base/head (ratchet adoption path)

## Determinism and stability requirements

Given identical inputs, depguard must produce **byte-stable outputs** for:
- `report.json`
- `comment.md` (if emitted)
- annotation stream (if emitted)

Ordering for findings:
1. severity (`error > warn > info`)
2. manifest path
3. line (missing last)
4. check_id
5. code
6. message

Codes/check IDs are API:
- never rename
- only deprecate with aliases and “since” metadata in explain registry

## Failure behavior requirements

- No `Cargo.toml` at root → verdict `skip` with reason `no_manifest`
- Workspace member discovery partial (glob mismatches) → verdict `warn` with reason `members_discovery_partial` (still emit receipt)
- Manifest parse error → policy-driven:
  - default: tool error (exit 1) but try to emit receipt with `tool.runtime_error` finding
  - optionally: “warn and continue” mode if configured (use sparingly; parsing failure is usually hard stop)
- Diff scope requested but base/head unavailable → tool error (exit 1) with remediation guidance (fetch depth / provide diff file)
