# depguard Requirements

## Problem
Teams need a clear list of assumptions before adopting depguard in policy-as-code workflows.

## Runtime requirements
- Rust toolchain for local install/build.
- Read access to repository manifests.
- Optional Git for `--scope diff` unless `--diff-file` is used.

## Behavioral requirements
- Offline-by-default operation (no required runtime network dependency).
- Network checks are available as opt-in flags (`--yanked-live`).
- No cargo build invocation.
- Deterministic outputs for identical inputs.

## Output requirements
- Machine-readable report for CI.
- Stable schema IDs and finding contracts.
- Strictly ordered findings.

## Repository expectations
- Repository manifests in standard `Cargo.toml` format.
- Optional `depguard.toml` for custom policy.

## Non-requirements
- Full dependency graph resolution.
- Automatic fixes without `--apply`.
- No unbounded network dependency in default execution mode.

## Adoption checklist
- Verify check IDs and severities in a pilot run.
- Add baseline file if existing violations should be gated.
- Keep CI path roots consistent with local runs.
