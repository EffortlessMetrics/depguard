# Configuration (`depguard.toml`)

> **Navigation**: [Quick Start](quickstart.md) | Configuration | [Checks](checks.md) | [CI Integration](ci-integration.md) | [Architecture](architecture.md) | [Troubleshooting](troubleshooting.md)

The config file is optional. If it doesn't exist, Depguard runs with the default profile (`strict`).

## Full example

```toml
# Schema identifier (optional, for editor support)
schema = "depguard.config.v1"

# Profile preset: "strict" (default), "warn", or "compat"
profile = "strict"

# Scope: "repo" (all manifests) or "diff" (changed manifests only)
scope = "repo"

# When to fail: "error" (default) or "warning"
fail_on = "error"

# Maximum findings to report (0 = unlimited)
max_findings = 100

# Per-check configuration
[checks."deps.no_wildcards"]
enabled = true
severity = "error"
allow = []

[checks."deps.path_requires_version"]
enabled = true
severity = "error"
allow = ["internal-dev-tool"]

[checks."deps.path_safety"]
enabled = true
severity = "error"
allow = []

[checks."deps.workspace_inheritance"]
enabled = true
severity = "warning"
allow = ["legacy-crate"]
```

## Profiles

Profiles provide opinionated defaults. Override individual settings as needed.

| Profile | Description |
|---------|-------------|
| `strict` | All checks enabled at `error` severity. Fails on any error. **(Default)** |
| `warn` | All checks enabled at `warning` severity. Fails on errors only. |
| `compat` | Lenient defaults for gradual adoption. All checks at `warning`. |

### Profile defaults

| Setting | `strict` | `warn` | `compat` |
|---------|----------|--------|----------|
| `fail_on` | `error` | `error` | `error` |
| `no_wildcards` severity | `error` | `warning` | `warning` |
| `path_requires_version` severity | `error` | `warning` | `warning` |
| `path_safety` severity | `error` | `warning` | `warning` |
| `workspace_inheritance` severity | `error` | `warning` | `warning` |

## Scope

| Value | Behavior |
|-------|----------|
| `repo` | Scan all manifests in the workspace |
| `diff` | Scan only manifests changed between `--base` and `--head` refs |

Diff scope always includes the root manifest (for `[workspace.dependencies]`).

## fail_on

Controls when the tool exits with failure code (2):

| Value | Fails when |
|-------|------------|
| `error` | Any finding has severity `error` |
| `warning` | Any finding has severity `warning` or `error` |

## Per-check configuration

Each check can be configured independently:

```toml
[checks."<check_id>"]
enabled = true|false     # Enable/disable the check
severity = "info|warning|error"  # Override severity
allow = ["pattern", ...]  # Allowlist (check-specific semantics)
```

### Check IDs

| Check ID | Purpose |
|----------|---------|
| `deps.no_wildcards` | Detect wildcard versions |
| `deps.path_requires_version` | Require version with path deps |
| `deps.path_safety` | Prevent absolute paths and escapes |
| `deps.workspace_inheritance` | Enforce workspace = true |

Unknown check IDs are allowed for forward compatibility.

### Allowlist semantics

The `allow` list is check-specific:

- **`deps.path_requires_version`**: Crate names that don't need version
- **`deps.workspace_inheritance`**: Crate names allowed to override workspace deps
- **`deps.path_safety`**: Path patterns allowed despite safety concerns

## CLI overrides

CLI flags take precedence over config file values:

```bash
depguard check \
  --profile warn \
  --scope diff \
  --max-findings 50 \
  --base main \
  --head HEAD
```

| Flag | Purpose |
|------|---------|
| `--profile <NAME>` | Override profile preset |
| `--scope <SCOPE>` | Override scope (`repo` or `diff`) |
| `--max-findings <N>` | Override max findings limit |
| `--base <REF>` | Git base ref for diff scope |
| `--head <REF>` | Git head ref for diff scope |
| `--config <PATH>` | Path to config file (default: `depguard.toml`) |
| `--repo-root <PATH>` | Repository root (default: current directory) |

## Resolution precedence

Configuration is resolved in order (highest priority first):

1. **CLI flags** — Always win
2. **Config file** — `depguard.toml` values
3. **Profile preset** — Default values for the selected profile

Example:
```toml
# depguard.toml
profile = "strict"
max_findings = 200

[checks."deps.no_wildcards"]
severity = "warning"
```

```bash
depguard check --max-findings 50
```

Result:
- Profile: `strict` (from config)
- Max findings: `50` (CLI override wins)
- `no_wildcards` severity: `warning` (config override of strict default)

## Environment

Depguard does not read environment variables for configuration. All settings come from the config file or CLI flags.

## Schema validation

The config file can include a schema identifier for editor support:

```toml
schema = "depguard.config.v1"
```

JSON schema available at: `schemas/depguard.config.v1.json`

## See also

- [Quick Start](quickstart.md) — Get started with depguard
- [Checks Catalog](checks.md) — All checks and their options
- [CI Integration](ci-integration.md) — CI pipeline setup
- [Troubleshooting](troubleshooting.md) — Common issues and solutions
