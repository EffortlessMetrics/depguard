# depguard-settings

Configuration parsing and resolution for depguard.

This crate translates user-facing TOML config plus CLI overrides into `EffectiveConfig` for the domain layer. It provides profile presets, check configuration, and policy resolution.

## Purpose

The settings crate:
- Parses `depguard.toml` configuration files
- Provides built-in profile presets (`strict`, `warn`, `compat`)
- Resolves configuration with proper precedence
- Validates configuration and returns actionable errors

## Key Features

### Configuration Model

```rust
pub struct DepguardConfigV1 {
    pub profile: Option<String>,
    pub scope: Option<String>,
    pub fail_on: Option<String>,
    pub baseline: Option<String>,
    pub checks: BTreeMap<String, CheckConfig>,
}

pub struct CheckConfig {
    pub enabled: Option<bool>,
    pub severity: Option<String>,
    pub allow: Vec<String>,
}
```

### Built-in Profiles

| Profile | Fail On | Default Severity | Description |
|---------|---------|------------------|-------------|
| `strict` | Error | Error | Strictest policy, all checks enabled |
| `warn` | Warning | Warning | Lenient policy, warnings only |
| `compat` | Warning | Warning | Compatibility mode for migration |

### Resolution Precedence

Configuration is resolved in this order (highest to lowest priority):

1. **CLI overrides** - Command-line arguments
2. **Config file values** - `depguard.toml` settings
3. **Profile defaults** - Preset profile values

## Public API

```rust
/// Parse a depguard.toml configuration file
pub fn parse_config_toml(input: &str) -> anyhow::Result<DepguardConfigV1>;

/// Resolve configuration with overrides
pub fn resolve_config(
    cfg: DepguardConfigV1,
    overrides: Overrides,
) -> anyhow::Result<ResolvedConfig>;

/// Configuration model
pub struct DepguardConfigV1 { /* ... */ }
pub struct CheckConfig { /* ... */ }

/// Resolution types
pub struct ResolvedConfig {
    pub effective: EffectiveConfig,
    pub baseline_path: Option<Utf8PathBuf>,
}

pub struct Overrides {
    pub profile: Option<String>,
    pub scope: Option<String>,
    pub fail_on: Option<String>,
}

/// Validation errors
pub struct ValidationError { /* ... */ }
pub struct ValidationErrors { /* ... */ }
```

## Usage Example

```rust
use depguard_settings::{parse_config_toml, resolve_config, Overrides};

// Parse configuration
let toml = r#"
profile = "strict"

[checks."deps.no_wildcards"]
severity = "error"

[checks."deps.path_safety"]
enabled = false
"#;

let config = parse_config_toml(toml)?;

// Resolve with overrides
let overrides = Overrides {
    profile: Some("warn".to_string()),
    ..Default::default()
};

let resolved = resolve_config(config, overrides)?;

println!("Effective profile: {}", resolved.effective.profile);
println!("Fail on: {:?}", resolved.effective.fail_on);
```

## Configuration File Format

```toml
# Profile preset (strict, warn, compat)
profile = "strict"

# Analysis scope (repo, diff)
scope = "repo"

# When to fail the build (error, warning, never)
fail_on = "error"

# Baseline suppressions file
baseline = ".depguard-baseline.json"

# Per-check configuration
[checks."deps.no_wildcards"]
enabled = true
severity = "error"
allow = []

[checks."deps.path_safety"]
enabled = true
severity = "warning"
allow = ["vendor/*", "../external/*"]

[checks."deps.workspace_inheritance"]
enabled = false
```

## Validation

The crate provides detailed validation errors:

```rust
use depguard_settings::{parse_config_toml, ValidationErrors};

match parse_config_toml(toml) {
    Ok(config) => { /* ... */ }
    Err(e) => {
        // Parse errors from TOML
        eprintln!("Parse error: {}", e);
    }
}

// Validation happens during resolution
match resolve_config(config, overrides) {
    Ok(resolved) => { /* ... */ }
    Err(e) => {
        eprintln!("Validation error: {}", e);
    }
}
```

## Design Constraints

- **No filesystem I/O**: Config is provided as string
- **No process I/O**: All values are in-memory
- **Stable profile defaults**: Presets are explicit and versioned
- **Actionable errors**: Validation errors include context and suggestions

## Dependencies

| Crate | Purpose |
|-------|---------|
| `depguard-types` | Severity, IDs |
| `depguard-domain-core` | Policy types |
| `depguard-check-catalog` | Check metadata and defaults |
| `anyhow` | Error handling |
| `serde` | Deserialization |
| `toml` | TOML parsing |
| `schemars` | JSON schema generation |
| `globset` | Allow list glob matching |

## Feature Flags

All check features are propagated through from check-catalog:
- `check-no-wildcards`
- `check-path-requires-version`
- `check-path-safety`
- `check-workspace-inheritance`
- `check-git-requires-version`
- `check-dev-only-in-normal`
- `check-default-features-explicit`
- `check-no-multiple-versions`
- `check-optional-unused`
- `check-yanked-versions`

## Related Crates

- [`depguard-app`](../depguard-app/) - Uses resolved config for checks
- [`depguard-check-catalog`](../depguard-check-catalog/) - Check defaults
- [`depguard-domain-core`](../depguard-domain-core/) - Policy types
- [`depguard-cli`](../depguard-cli/) - CLI override handling
