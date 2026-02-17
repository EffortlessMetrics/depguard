use depguard_domain::policy::{CheckPolicy, EffectiveConfig, FailOn, Scope};
use depguard_types::Severity;
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
        checks: default_checks(Severity::Error),
    }
}

fn warn_profile() -> EffectiveConfig {
    EffectiveConfig {
        profile: "warn".to_string(),
        scope: Scope::Repo,
        fail_on: FailOn::Warning,
        max_findings: 200,
        yanked_index: None,
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
        yanked_index: None,
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
    let mut workspace_policy = CheckPolicy::enabled(default_severity);
    workspace_policy.enabled = false;
    m.insert(
        CHECK_DEPS_WORKSPACE_INHERITANCE.to_string(),
        workspace_policy,
    );

    let mut git_policy = CheckPolicy::disabled();
    git_policy.severity = default_severity;
    m.insert(CHECK_DEPS_GIT_REQUIRES_VERSION.to_string(), git_policy);

    let mut default_features_policy = CheckPolicy::disabled();
    default_features_policy.severity = Severity::Warning;
    m.insert(
        CHECK_DEPS_DEFAULT_FEATURES_EXPLICIT.to_string(),
        default_features_policy,
    );

    let mut no_multiple_versions_policy = CheckPolicy::disabled();
    no_multiple_versions_policy.severity = Severity::Warning;
    m.insert(
        CHECK_DEPS_NO_MULTIPLE_VERSIONS.to_string(),
        no_multiple_versions_policy,
    );

    let mut optional_unused_policy = CheckPolicy::disabled();
    optional_unused_policy.severity = Severity::Warning;
    m.insert(
        CHECK_DEPS_OPTIONAL_UNUSED.to_string(),
        optional_unused_policy,
    );

    let mut dev_only_in_normal_policy = CheckPolicy::disabled();
    dev_only_in_normal_policy.severity = Severity::Warning;
    m.insert(
        CHECK_DEPS_DEV_ONLY_IN_NORMAL.to_string(),
        dev_only_in_normal_policy,
    );

    let mut yanked_versions_policy = CheckPolicy::disabled();
    yanked_versions_policy.severity = Severity::Error;
    m.insert(
        CHECK_DEPS_YANKED_VERSIONS.to_string(),
        yanked_versions_policy,
    );

    m
}
