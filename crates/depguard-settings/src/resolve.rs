use crate::{ValidationError, model::DepguardConfigV1, presets};
use depguard_domain_core::policy::{CheckPolicy, EffectiveConfig, FailOn, Scope};
use depguard_types::Severity;
use globset::Glob;

#[derive(Clone, Debug, Default)]
pub struct Overrides {
    pub profile: Option<String>,
    pub scope: Option<String>,
    pub max_findings: Option<u32>,
    pub baseline: Option<String>,
}

#[derive(Clone, Debug)]
pub struct ResolvedConfig {
    pub effective: EffectiveConfig,
    pub baseline_path: Option<String>,
}

pub fn resolve_config(
    cfg: DepguardConfigV1,
    overrides: Overrides,
) -> anyhow::Result<ResolvedConfig> {
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
            .or_insert_with(CheckPolicy::disabled);

        if let Some(enabled) = cc.enabled {
            entry.enabled = enabled;
        }
        if let Some(sev) = cc.severity.as_deref() {
            entry.severity = parse_severity(check_id, sev)?;
        }
        if !cc.allow.is_empty() {
            validate_allowlist(check_id, &cc.allow)?;
            entry.allow = cc.allow.clone();
        }
        if let Some(ignore_publish_false) = cc.ignore_publish_false {
            entry.ignore_publish_false = ignore_publish_false;
        }
    }

    // fail_on override from config
    if let Some(fail_on_s) = cfg.fail_on.as_deref() {
        effective.fail_on = parse_fail_on(fail_on_s)?;
    }

    let baseline_path = overrides.baseline.or(cfg.baseline);

    Ok(ResolvedConfig {
        effective,
        baseline_path,
    })
}

fn validate_allowlist(check_id: &str, patterns: &[String]) -> anyhow::Result<()> {
    for pattern in patterns {
        Glob::new(pattern).map_err(|e| {
            anyhow::Error::new(ValidationError::invalid_allow_glob(
                check_id,
                pattern,
                &e.to_string(),
            ))
        })?;
    }
    Ok(())
}

fn parse_scope(v: &str) -> anyhow::Result<Scope> {
    match v {
        "repo" => Ok(Scope::Repo),
        "diff" => Ok(Scope::Diff),
        other => Err(anyhow::Error::new(ValidationError::unknown_scope(other))),
    }
}

fn parse_severity(check_id: &str, v: &str) -> anyhow::Result<Severity> {
    match v {
        "info" => Ok(Severity::Info),
        "warning" | "warn" => Ok(Severity::Warning),
        "error" => Ok(Severity::Error),
        other => Err(anyhow::Error::new(ValidationError::unknown_severity(
            check_id, other,
        ))),
    }
}

fn parse_fail_on(v: &str) -> anyhow::Result<FailOn> {
    match v {
        "error" => Ok(FailOn::Error),
        "warning" | "warn" => Ok(FailOn::Warning),
        other => Err(anyhow::Error::new(ValidationError::unknown_fail_on(other))),
    }
}
