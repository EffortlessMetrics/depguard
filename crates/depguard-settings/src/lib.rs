//! Config parsing and profile/preset resolution.
//!
//! This crate is intentionally IO-free: it parses and resolves configuration provided as strings.

#![forbid(unsafe_code)]

mod model;
mod presets;
mod resolve;

pub use model::{CheckConfig, DepguardConfigV1};
pub use resolve::{Overrides, ResolvedConfig};

/// Parse `depguard.toml` (or equivalent) into a typed model.
pub fn parse_config_toml(input: &str) -> anyhow::Result<DepguardConfigV1> {
    let cfg: DepguardConfigV1 = toml::from_str(input)?;
    Ok(cfg)
}

/// Resolve the effective config used by the engine (profiles + overrides + per-check config).
pub fn resolve_config(
    cfg: DepguardConfigV1,
    overrides: Overrides,
) -> anyhow::Result<ResolvedConfig> {
    resolve::resolve_config(cfg, overrides)
}
