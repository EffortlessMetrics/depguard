# depguard — Design

This document describes *how* depguard works: data flow, parsing, rule evaluation, and outputs.

## High-level behavior

depguard performs deterministic, repo-only analysis:

1. Discover workspace manifests (root + members)
2. Parse manifests with formatting tolerance
3. Walk dependency declarations across relevant tables
4. Apply a small set of deterministic checks
5. Emit a versioned receipt and optional renderings

## Core design constraints

- **Repo truth** only: no builds/tests, no network.
- **Deterministic outputs**: stable ordering and stable truncation rules.
- **Protocol discipline**: strict top-level envelope; extension data only under `data`.
- **Adoption valve**: profiles + diff-scope mode + allowlists.

## Data model (domain)

### Workspace
A workspace is represented as:

- `root_manifest: Manifest`
- `member_manifests: Vec<Manifest>`
- `workspace_deps: WorkspaceDeps` (keys in `[workspace.dependencies]`)

### Manifest
A manifest includes:

- `path: RepoPath`
- `doc: toml_edit::Document` (or equivalent)
- `package_meta`:
  - `name`
  - `publish` (bool / optional; defaults to true in Cargo semantics)
  - other minimal fields as needed

### DependencyEntry
Each discovered dependency becomes a normalized entry:

- `manifest_path`
- `section_kind`: `dependencies | dev-dependencies | build-dependencies`
- `target`: optional `cfg(...)` / target triple key
- `dep_name`
- `spec`: one of:
  - `VersionString { raw }`
  - `InlineTable { keys... }` (including `version`, `path`, `workspace`, flags)
  - `WorkspaceInherited` (`{ workspace = true }`)
  - (Optionally later) others, but avoid feature creep

### Finding
A finding is a stable, explainable record:

- `severity`: info|warn|error
- `check_id`: producer identity (e.g. `deps.no_wildcards`)
- `code`: classification (e.g. `wildcard_version`)
- `message`: single-line human text
- `location`: best effort
  - `path` (repo-relative; forward slashes)
  - optional `line`, `col`
- `help`: short remediation guidance (prefer imperative verb)
- optional `data`: structured hints (for future actuators and better rendering)

## Workspace discovery

### Default strategy (no Cargo dependency)
1. Read root `Cargo.toml`.
2. If `[workspace]` exists:
   - read `members` (globs) and `exclude` (globs)
   - expand globs relative to root
   - for each matching directory, locate `Cargo.toml`
   - dedupe and sort paths deterministically
3. Else (single crate):
   - only analyze root `Cargo.toml`

Notes:
- This is intentionally “manifest-semantics first.” It won’t perfectly match Cargo in every exotic case. If needed, an opt-in `cargo-metadata` mode can exist later, but it must never become the default.

### Diff scope selection (`--scope diff`)
Diff scope reduces which manifests are scanned:
- list changed files between base/head
- filter to `Cargo.toml` paths (and optionally `Cargo.lock` ignored)
- scan only those manifests
- still needs workspace context for `workspace.dependencies` (read root manifest regardless)

## Parsing strategy

Use `toml_edit`:
- tolerant of comments and formatting
- preserves structure for best-effort location mapping

The parser must be resilient:
- treat unknown keys as opaque
- treat malformed dependency specs as parse errors (tool error) unless explicitly configured otherwise

## Dependency table walker

Scan these tables in each manifest:
- `[dependencies]`
- `[dev-dependencies]`
- `[build-dependencies]`
- `[target.<...>.dependencies]`, and dev/build variants

For each dep entry, normalize spec shape:
- version string
- inline table (path/version/workspace/etc)
- workspace inheritance

Do not attempt to interpret git dependencies beyond the minimal checks (v0.1 should not gate on git URLs; that’s a different policy class).

## Checks (MVP)

### 1) deps.no_wildcards
Condition:
- any version requirement contains `*` (in string or inline table `version` field)

Applies to:
- string versions: `"*"` or `"1.*"` etc
- inline table: `{ version = "1.*", ... }`
- workspace deps: only if a member overrides a workspace dep with a wildcard (workspace check will catch the override; wildcard check catches wildcard itself)

Finding:
- `check_id`: `deps.no_wildcards`
- `code`: `wildcard_version`
- location: manifest path, best-effort line
- help: “Pin a semver requirement (e.g., `^1.2` or `~1.2.3`) or centralize in `[workspace.dependencies]`.”

### 2) deps.path_requires_version
Condition:
- spec is inline table with `path = "...“`
- and does NOT include `version`
- and is NOT `{ workspace = true }`

Config options:
- `ignore_publish_false`: if true, skip this check for manifests with `package.publish = false`
  - rationale: non-published crates often use local path deps without version requirements
  - default: true in `oss`, false in `team/strict` (or configurable)

Finding:
- `check_id`: `deps.path_requires_version`
- `code`: `missing_version`
- data hint: `{ dep_name, dep_path }`
- help: “Add `version = "<target crate version>"` or inherit via `[workspace.dependencies]`.”

### 3) deps.path_safety
Condition:
- `path` is absolute OR
- lexical normalization escapes workspace root (e.g., `../../..`)

Implementation details:
- absolute path detection must be cross-platform
- escaping root should be lexical (normalize segments, track `..`)
- allowlist globs should exist for rare cases (but treat them as policy exceptions)

Findings:
- `check_id`: `deps.path_safety`
- `code`: `absolute_path` or `escapes_root`
- help: “Use repo-relative paths that do not escape the workspace root.”

### 4) deps.workspace_inheritance
Condition:
- workspace root defines `[workspace.dependencies]` key `<dep>`
- a member manifest specifies `<dep>` without `{ workspace = true }`

The idea:
- centralize versions and prevent drift
- reduce “it compiles locally but not in CI” style mismatch due to divergent feature sets or versions

Config options:
- `enabled` off by default in `oss`
- allowlist `allow_deps` to permit per-crate overrides
- severity mapping depends on profile

Finding:
- `check_id`: `deps.workspace_inheritance`
- `code`: `not_inherited`
- data hint: `{ dep_name }`
- help: “Replace member entry with `{ workspace = true }` and keep flags like `features`, `optional`, `default-features`.”

## Config model (depguard.toml)

Principles:
- small, explicit knobs
- profiles map to defaults; config overrides
- policy is separate from composition (cockpit decides blocking/missing receipts)

Recommended keys:
- `profile`, `scope`, `fail_on`, `max_findings`
- per-check `enabled` and `severity`
- allowlists for paths and dep names

## Receipt emission

Depguard emits:
- `depguard.report.v1` (envelope-compliant)

Tool-specific metrics go under `data`:
- `scope`, `profile`
- counts scanned
- truncation flags and reasons

### Truncation rules
Depguard should not cap the *receipt* by default unless configured.
But the PR comment and annotations must be capped.

If receipt capping is enabled (rare), include:
- `data.truncated = true`
- `data.truncated_reason = "...“`

## Rendering

### Markdown (comment.md)
Constraints:
- short summary + table of top N findings
- link to report artifact
- include a repro line (optional, config-driven)

### Annotations
- only location-bearing findings
- cap to `max_annotations`
- stable ordering matches receipt ordering

## Error classification

- I/O errors: tool/runtime error (exit 1)
- parse errors: tool/runtime error (exit 1), with clear message pointing to which file
- discovery partial: warn + continue where possible (still emit receipt)
- diff-scope missing base/head: tool/runtime error unless a diff file is provided (optional future feature)
