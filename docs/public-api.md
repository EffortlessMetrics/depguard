# depguard Public API Surfaces (Alpha)

## Supported for users

The supported user-facing entrypoints are:

- `depguard-cli` — command surface for local and CI execution
- `depguard` — Rust embedding facade
- `depguard-types` — report/config schema contracts and IDs

## Supported CLI contract

The CLI contract is:

- `depguard check`
- `depguard baseline`
- `depguard explain`
- `depguard ci github`
- `depguard report md|annotations|sarif|junit|jsonl`
- `depguard fix`

`depguard md`, `depguard annotations`, `depguard sarif`, `depguard junit`, and `depguard jsonl` remain
as compatibility aliases for `depguard report ...` and are not the preferred docs-first pattern.

## Supporting published crates

The following crates are part of the publishable implementation surface but are not the primary integration entrypoint:

- `depguard-domain`
- `depguard-repo`
- `depguard-settings`
- `depguard-render`
- `depguard-yanked`
- `depguard-app`

## Internal-only packages

- `depguard-domain-core` (`internal`)
- `depguard-domain-checks` (`internal`)
- `depguard-check-catalog` (`internal`)
- `depguard-repo` parser modules (`internal`)
- `depguard-test-util` (`publish = false`)
- `xtask` (`publish = false`)

## Recommended CI integration

Use `depguard ci github` as the stable GitHub Actions entrypoint and consume reports from:

- `artifacts/depguard/report.json`
- `depguard report md|annotations|sarif|junit|jsonl`

The reusable workflow in `/.github/workflows/depguard-reusable.yml` is the recommended cross-repo integration pattern.
