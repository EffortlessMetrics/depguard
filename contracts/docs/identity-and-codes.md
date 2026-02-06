# Identity and Codes

Normative rules for stable identifiers in the cockpit ecosystem.

## check_id naming

- Pattern: `^[a-z][a-z0-9_.]*$` (dotted namespace)
- Examples: `deps.no_wildcards`, `deps.path_safety`, `tool.runtime`
- The namespace prefix groups related checks (e.g., `deps.*` for dependency checks)

## code naming

- Pattern: `^[a-z][a-z0-9_]*$` (snake_case)
- Examples: `wildcard_version`, `absolute_path`, `runtime_error`
- Codes are unique within a check_id but may appear across different check_ids

## Reason tokens

- Pattern: same as codes — `^[a-z][a-z0-9_]*$`
- Used in `verdict.reasons[]` and `capabilities.*.reason`
- MUST NOT contain spaces or prose; machine-readable only
- Examples: `diff_scope_disabled`, `config_missing_defaulted`, `runtime_error`

## Stability rule

- Never rename a check_id or code once published
- Deprecation only via aliases (old ID maps to new)
- Removed checks keep their explanation entry with a deprecation notice

## Reference

- Stable identifiers: `crates/depguard-types/src/ids.rs`
- Explanation registry: `crates/depguard-types/src/explain.rs`
