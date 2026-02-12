# CLAUDE.md — depguard-settings

## Purpose

Configuration parsing and profile resolution. Bridges user-facing TOML config files to domain types.

## Key Modules

| Module | Contents |
|--------|----------|
| `model.rs` | `DepguardConfigV1`, `CheckConfig` — user-facing schema |
| `presets.rs` | Built-in profiles: `strict`, `warn`, `compat` |
| `resolve.rs` | `Overrides`, `ResolvedConfig`, `resolve_config()` |

## Public API

```rust
// Parse TOML config (no I/O — takes string)
pub fn parse_config_toml(input: &str) -> Result<DepguardConfigV1>

// Resolve final config with precedence: CLI > file > preset
pub fn resolve_config(cfg: Option<DepguardConfigV1>, overrides: Overrides) -> Result<ResolvedConfig>
```

## Profiles

| Profile | Behavior |
|---------|----------|
| `strict` | Default; all checks enabled at Error severity; fail on error |
| `warn` | All checks enabled at Warning severity; fail on error |
| `compat` | All checks enabled at Warning severity; fail on error (lenient defaults) |

## Resolution Precedence

1. CLI flags (highest priority)
2. Config file values
3. Preset defaults (lowest priority)

## Config File Schema

```toml
# depguard.toml
schema = "depguard.config.v1"
profile = "strict"
scope = "repo"          # or "diff"
fail_on = "error"       # or "warning"
max_findings = 100

[checks.no_wildcards]
enabled = true
severity = "error"
allow = ["some-crate"]
```

## Dependencies

- `depguard-types` — DTOs
- `depguard-domain` — `EffectiveConfig` and policy types
- `toml` — Config parsing
- `schemars` — JSON schema for config

## Testing

```bash
cargo test -p depguard-settings
```

Tests cover profile precedence, validation errors, and per-check override merging.
