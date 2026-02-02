# depguard — Configuration (`depguard.toml`)

depguard has a small sensor-local config. Cockpit composition policy (blocking, missing receipts, budgets)
lives in `cockpit.toml` and is read by the director.

## Principles

- Minimal knobs; explicit behavior.
- Profiles set defaults; config overrides.
- Avoid “implicit magic” that differs across repos.

## Example

```toml
profile = "team"          # oss|team|strict
scope = "diff"            # repo|diff
fail_on = "error"         # error|warn|never

max_findings = 100
max_comment_findings = 10
max_annotations = 25

[paths]
exclude = ["vendor/**", "third_party/**", "target/**"]

[checks.deps.no_wildcards]
enabled = true
severity = "error"

[checks.deps.path_requires_version]
enabled = true
severity = "error"
ignore_publish_false = true

[checks.deps.path_safety]
enabled = true
severity = "error"
allow_paths = ["crates/**"]

[checks.deps.workspace_inheritance]
enabled = true
severity = "warn"
allow_deps = ["tracing"]
```

## Top-level keys

- `profile`: `oss|team|strict`
- `scope`: `repo|diff`
- `fail_on`: `error|warn|never`
- `max_findings`: cap findings in receipt (rare; prefer leaving receipt uncapped)
- `max_comment_findings`: cap surfaced findings in markdown output
- `max_annotations`: cap annotations

## paths

- `exclude`: globs to ignore entire manifests or paths from analysis (use sparingly)

## checks.<check_id>

Common fields:
- `enabled`: bool
- `severity`: `info|warn|error` (maps to finding severity)
- allowlists/knobs specific to the check

Check-specific knobs:
- `deps.path_requires_version.ignore_publish_false`: bool
- `deps.path_safety.allow_paths`: globs
- `deps.workspace_inheritance.allow_deps`: list of dependency names allowed to override workspace deps

## Scope and base/head

In diff scope, depguard needs base/head revs (or a future `--diff-file` option):

- `depguard check --scope diff --base <sha> --head <sha>`

If shallow clone prevents base resolution, depguard should fail with a clear remediation:
- fetch depth must include the base commit, or provide a patch file (if supported).

## Interaction with cockpit policy

depguard does **not** decide “blocking vs informational.” That lives in `cockpit.toml`:

- `blocking = true|false`
- missing receipt policy: `skip|warn|fail`
- budget caps for cockpit highlight selection
