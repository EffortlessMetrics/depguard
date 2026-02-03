use depguard_domain::policy::{CheckPolicy, EffectiveConfig, FailOn, Scope};
use depguard_types::Severity;
use std::collections::BTreeMap;

/// Preset profiles are opinionated defaults.
///
/// Keep these small and readable. Anything complex should go into repo config.
pub fn preset(profile: &str) -> EffectiveConfig {
    match profile {
        "warn" => warn_profile(),
        "compat" => compat_profile(),
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
        checks: default_checks(Severity::Error),
    }
}

fn warn_profile() -> EffectiveConfig {
    EffectiveConfig {
        profile: "warn".to_string(),
        scope: Scope::Repo,
        fail_on: FailOn::Warning,
        max_findings: 200,
        checks: default_checks(Severity::Warning),
    }
}

fn compat_profile() -> EffectiveConfig {
    // Compatibility mode is “mostly on”, but keeps some rules as warnings by default.
    EffectiveConfig {
        profile: "compat".to_string(),
        scope: Scope::Repo,
        fail_on: FailOn::Error,
        max_findings: 200,
        checks: default_checks(Severity::Warning),
    }
}

fn default_checks(default_severity: Severity) -> BTreeMap<String, CheckPolicy> {
    use depguard_types::ids::*;
    let mut m = BTreeMap::new();

    m.insert(
        CHECK_DEPS_NO_WILDCARDS.to_string(),
        CheckPolicy::enabled(default_severity),
    );
    m.insert(
        CHECK_DEPS_PATH_REQUIRES_VERSION.to_string(),
        CheckPolicy::enabled(default_severity),
    );
    m.insert(
        CHECK_DEPS_PATH_SAFETY.to_string(),
        CheckPolicy::enabled(default_severity),
    );
    m.insert(
        CHECK_DEPS_WORKSPACE_INHERITANCE.to_string(),
        CheckPolicy::enabled(default_severity),
    );

    m
}
