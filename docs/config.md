# Configuration (`depguard.toml`)

The config file is optional. If it doesn't exist, Depguard runs with presets (default: `strict`).

The model in this scaffold is intentionally small:

```toml
schema = "depguard.config.v1"
profile = "strict"      # strict|warn|compat (or a future custom profile)
scope = "repo"          # repo|diff
max_findings = 200

[checks."deps.path_requires_version"]
enabled = true
severity = "error"      # info|warning|error
allow = ["my_internal_crate"]  # semantics are check-specific in v1
```

Notes:
- `checks` keys are `check_id` strings.
- Unknown `check_id`s are allowed (forward compatibility).
- `allow` is a generic list for v1; if you need rich matching (globs, per-kind, per-target), add a v2 model
  rather than overloading strings.

CLI overrides (planned):
- `--profile`
- `--scope`
- `--max-findings`
