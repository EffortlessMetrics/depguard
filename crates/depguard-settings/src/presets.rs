use depguard_check_catalog as check_catalog;
use depguard_domain_core::policy::{CheckPolicy, EffectiveConfig, FailOn, Scope};
use std::collections::BTreeMap;

/// Preset profiles are opinionated defaults.
///
/// Keep these small and readable. Anything complex should go into repo config.
pub fn preset(profile: &str) -> EffectiveConfig {
    match profile {
        "warn" | "team" => warn_profile(),
        "compat" | "oss" => compat_profile(),
        // default
        _ => strict_profile(),
    }
}

fn strict_profile() -> EffectiveConfig {
    EffectiveConfig {
        profile: "strict".to_string(),
        scope: Scope::Repo,
        fail_on: FailOn::Error,
        max_findings: 200,
        yanked_index: None,
        checks: default_checks("strict"),
    }
}

fn warn_profile() -> EffectiveConfig {
    EffectiveConfig {
        profile: "warn".to_string(),
        scope: Scope::Repo,
        fail_on: FailOn::Warning,
        max_findings: 200,
        yanked_index: None,
        checks: default_checks("warn"),
    }
}

fn compat_profile() -> EffectiveConfig {
    // Compatibility mode is “mostly on”, but keeps some rules as warnings by default.
    EffectiveConfig {
        profile: "compat".to_string(),
        scope: Scope::Repo,
        fail_on: FailOn::Error,
        max_findings: 200,
        yanked_index: None,
        checks: default_checks("compat"),
    }
}

fn default_checks(profile: &str) -> BTreeMap<String, CheckPolicy> {
    let mut m = BTreeMap::new();

    for check in check_catalog::checks_for_profile(profile) {
        let mut policy = CheckPolicy::enabled(check.severity);
        if !check.enabled || !check_catalog::is_check_available(check.id) {
            policy.enabled = false;
        }
        m.insert(check.id.to_string(), policy);
    }

    m
}
