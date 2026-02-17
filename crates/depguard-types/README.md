# depguard-types

Stable protocol types and identifier registries for depguard.

This crate defines the contract consumed by depguard adapters, renderers, and CI integrations.

## Owns

- Report envelope types for `depguard.report.v1`, `depguard.report.v2`, and `sensor.report.v1`
- Buildfix actuator plan types for `buildfix.plan.v1`
- Baseline types for `depguard.baseline.v1`
- Stable check IDs, finding codes, reason tokens, and fix-action tokens (`ids`)
- Explanation registry (`lookup_explanation`)
- Canonical repo-relative path type (`RepoPath`)

## Design Constraints

- IDs and codes are stable contract values and must not be renamed.
- Types remain serialization/schema friendly (`serde` + `schemars`).
- No filesystem, process, or network I/O.

## Related Docs

- `../../docs/microcrates.md`
- `../../docs/architecture.md`
