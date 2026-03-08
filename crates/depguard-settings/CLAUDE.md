# CLAUDE.md — depguard-settings

## Purpose

Configuration parsing and profile resolution. Bridges user-facing TOML config files to domain types. Uses `depguard-check-catalog` for check metadata and defaults.

## Key Modules

| Module | Contents |
|--------|----------|
| [`model.rs`] | `DepguardConfigV1`, `CheckConfig` — user-facing schema |
| [`presets.rs`] | Built-in profiles: `strict`, `warn`, `compat` |
| [`resolve.rs`] | `Overrides`, `ResolvedConfig`, `resolve_config()` |
| [`validation_error.rs`] | Config validation error types |

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
| `strict` | Default; checks enabled per catalog defaults; fail on error |
| `warn` | All checks at Warning severity; fail on error |
| `compat` | Lenient defaults; warnings only; fail on error |

Profile defaults come from `depguard-check-catalog`.

## Resolution Precedence

1. CLI flags (highest priority)
2. Config file values
3. Preset defaults from catalog (lowest priority)

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

[checks.path_requires_version]
ignore_publish_false = true
```

## Feature Gates

This crate propagates check features to `depguard-check-catalog`:

```toml
check-no-wildcards = ["depguard-check-catalog/check-no-wildcards"]
```

All 10 checks have corresponding features. Features control which checks are available at compile time.

## Design Constraints

- **No I/O**: Takes string input, returns config
- **Validation**: Returns actionable error messages
- **Extensibility**: New checks add entries to catalog, not here

## Dependencies

| Dependency | Purpose |
|------------|---------|
| `depguard-types` | DTOs, Severity |
| `depguard-domain-core` | `EffectiveConfig`, policy types |
| `depguard-check-catalog` | Check metadata, profile defaults |
| `anyhow` | Error handling |
| `schemars` | JSON schema generation |
| `serde` | Deserialization |
| `toml` | TOML parsing |
| `globset` | Pattern matching for allow lists |

## Testing

```bash
cargo test -p depguard-settings
```

Tests cover:
- Profile precedence
- Validation errors
- Per-check override merging
- Feature-gated check availability

## Architecture Notes

This crate depends only on `depguard-domain-core` (not `depguard-domain-checks`) to avoid pulling in check implementations. Check metadata comes from `depguard-check-catalog` which is data-only.

```
depguard-settings → depguard-domain-core (model/policy)
depguard-settings → depguard-check-catalog (metadata only)
```
