# Capabilities and Missingness

Rules for No Green By Omission reporting.

## Shape

The `capabilities` object uses named keys, each with a `CapabilityStatus`:

```json
{
  "git": { "status": "available" },
  "config": { "status": "missing", "reason": "config_missing_defaulted" }
}
```

Keys are extensible — sensors may add domain-specific capabilities beyond `git` and `config`.

## Status values

| Status | Meaning |
|--------|---------|
| `available` | Capability fully functional |
| `missing` | Capability not present; analysis is incomplete |
| `degraded` | Capability partially functional; results may be incomplete |

## Reason field

- MUST be a snake_case token matching `^[a-z][a-z0-9_]*$`
- MUST NOT be prose (no spaces, no sentences)
- Required when status is `missing` or `degraded`; omitted when `available`

## No Green By Omission

A `pass` verdict with any `missing` or `degraded` capability does NOT mean clean.
Consumers MUST inspect capabilities before treating a pass as authoritative.

## Depguard reason token registry

| Token | Used when |
|-------|-----------|
| `diff_scope_disabled` | Git diff scope not enabled (repo-wide scan only) |
| `config_missing_defaulted` | No config file found; using built-in defaults |
| `runtime_error` | Tool encountered a runtime error |
| `no_manifest_found` | No Cargo.toml manifests discovered |
