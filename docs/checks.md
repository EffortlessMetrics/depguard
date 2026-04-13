# depguard Checks Catalog

## Problem
Without a clear check-level map, teams cannot predict rule behavior or justify failures.

## Stable check identity
Each finding uses:
- `check_id`: stable namespace (`deps.no_wildcards`)
- `code`: stable discriminator (`wildcard_version`)

## Check list

- `deps.no_wildcards` — disallow wildcard dependency versions.
- `deps.path_requires_version` — require versions for path dependencies.
- `deps.path_safety` — prevent unsafe path escapes.
- `deps.git_requires_version` — require version context for git dependencies.
- `deps.workspace_inheritance` — ensure workspace inheritance is used correctly.
- `deps.dev_only_in_normal` — catch dev-only deps in normal dependency tables.
- `deps.default_features_explicit` — require explicit `default-features` when needed.
- `deps.no_multiple_versions` — report duplicate version patterns.
- `deps.optional_unused` — detect optional unused dependencies.
- `deps.yanked_versions` — exact-match yanked version detection.

## How to customize

- Disable a check:
```toml
[checks."deps.no_wildcards"]
enabled = false
```
- Raise severity:
```toml
[checks."deps.path_requires_version"]
severity = "error"
```
- Add allow-list exceptions:
```toml
[checks."deps.no_wildcards"]
allow = ["internal-*", "vendor-*"]
```

## Remediation flow
1. Identify the `check_id`/`code` from report.
2. Use `depguard explain <check_id>` or `<code>`.
3. Decide strictness/allow exceptions in config.
4. Add baseline suppressions for intentional debt.

## Related docs
- [`docs/config.md`](config.md)
- [`docs/troubleshooting.md`](troubleshooting.md)
