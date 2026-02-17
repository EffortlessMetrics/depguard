# depguard-settings

Configuration parsing and resolution for depguard.

This crate translates user-facing TOML config plus CLI overrides into `depguard-domain`'s `EffectiveConfig`.

## Owns

- Config model (`DepguardConfigV1`, `CheckConfig`)
- Built-in profiles (`strict`, `warn`, `compat`)
- Resolution logic and precedence (`resolve_config`)

## Public API

- `parse_config_toml(input: &str) -> anyhow::Result<DepguardConfigV1>`
- `resolve_config(cfg: DepguardConfigV1, overrides: Overrides) -> anyhow::Result<ResolvedConfig>`

## Resolution Precedence

1. CLI overrides
2. Config file values
3. Profile defaults

## Design Constraints

- No filesystem or process I/O
- Keep profile defaults stable and explicit
- Return actionable validation errors for invalid config
