# Depguard report

- Verdict: **FAIL**
- Findings: 1 (emitted) / 1 (total)

## Summary

1 error

## Findings

### ERROR

#### `deps.no_wildcards`

- `deps.no_wildcards` / `wildcard_version` — dependency 'serde' uses a wildcard version: * ([`Cargo.toml`:9](Cargo.toml:L9))
  - help: Replace wildcard versions with an explicit semver requirement.
