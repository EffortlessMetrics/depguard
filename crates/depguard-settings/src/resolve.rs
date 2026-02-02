use crate::{model::DepguardConfigV1, presets};
use anyhow::Context;
use depguard_domain::{CheckPolicy, EffectiveConfig, FailOn, Scope};
use depguard_types::Severity;

#[derive(Clone, Debug, Default)]
pub struct Overrides {
    pub profile: Option<String>,
    pub scope: Option<String>,
    pub max_findings: Option<u32>,
}

#[derive(Clone, Debug)]
pub struct ResolvedConfig {
    pub effective: EffectiveConfig,
}

pub fn resolve_config(cfg: DepguardConfigV1, overrides: Overrides) -> anyhow::Result<ResolvedConfig> {
    let profile = overrides
        .profile
        .clone()
        .or(cfg.profile.clone())
        .unwrap_or_else(|| "strict".to_string());

    let mut effective = presets::preset(&profile);

    // Scope
    if let Some(scope_s) = overrides.scope.clone().or(cfg.scope.clone()) {
        effective.scope = parse_scope(&scope_s)?;
    }

    // max findings
    if let Some(mf) = overrides.max_findings.or(cfg.max_findings) {
        effective.max_findings = mf as usize;
    }

    // per-check overrides
    for (check_id, cc) in cfg.checks.iter() {
        let entry = effective
            .checks
            .entry(check_id.clone())
            .or_insert_with(|| CheckPolicy::disabled());

        if let Some(enabled) = cc.enabled {
            entry.enabled = enabled;
        }
        if let Some(sev) = cc.severity.as_deref() {
            entry.severity = parse_severity(sev)
                .with_context(|| format!("invalid severity for {check_id}"))?;
        }
        if !cc.allow.is_empty() {
            entry.allow = cc.allow.clone();
        }
    }

    // fail_on may be profile-driven for now; keep override space for v2 config.

    Ok(ResolvedConfig { effective })
}

fn parse_scope(v: &str) -> anyhow::Result<Scope> {
    match v {
        "repo" => Ok(Scope::Repo),
        "diff" => Ok(Scope::Diff),
        other => anyhow::bail!("unknown scope: {other} (expected 'repo' or 'diff')"),
    }
}

fn parse_severity(v: &str) -> anyhow::Result<Severity> {
    match v {
        "info" => Ok(Severity::Info),
        "warning" | "warn" => Ok(Severity::Warning),
        "error" => Ok(Severity::Error),
        other => anyhow::bail!("unknown severity: {other} (expected info|warning|error)"),
    }
}

#[allow(dead_code)]
fn parse_fail_on(v: &str) -> anyhow::Result<FailOn> {
    match v {
        "error" => Ok(FailOn::Error),
        "warning" | "warn" => Ok(FailOn::Warning),
        other => anyhow::bail!("unknown fail_on: {other} (expected error|warning)"),
    }
}
