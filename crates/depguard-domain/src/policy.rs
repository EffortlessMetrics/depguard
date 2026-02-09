use depguard_types::Severity;
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    Repo,
    Diff,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FailOn {
    Error,
    Warning,
}

#[derive(Clone, Debug)]
pub struct CheckPolicy {
    pub enabled: bool,
    pub severity: Severity,
    pub allow: Vec<String>,
    /// Check-specific option for deps.path_requires_version.
    pub ignore_publish_false: bool,
}

impl CheckPolicy {
    pub fn enabled(severity: Severity) -> Self {
        Self {
            enabled: true,
            severity,
            allow: Vec::new(),
            ignore_publish_false: false,
        }
    }

    pub fn disabled() -> Self {
        Self {
            enabled: false,
            severity: Severity::Info,
            allow: Vec::new(),
            ignore_publish_false: false,
        }
    }
}

#[derive(Clone, Debug)]
pub struct EffectiveConfig {
    pub profile: String,
    pub scope: Scope,
    pub fail_on: FailOn,
    pub max_findings: usize,
    pub checks: BTreeMap<String, CheckPolicy>,
}

impl EffectiveConfig {
    pub fn check_policy(&self, check_id: &str) -> Option<&CheckPolicy> {
        self.checks.get(check_id).filter(|p| p.enabled)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use depguard_types::Severity;
    use std::collections::BTreeMap;

    #[test]
    fn check_policy_enabled_and_disabled() {
        let enabled = CheckPolicy::enabled(Severity::Warning);
        assert!(enabled.enabled);
        assert_eq!(enabled.severity, Severity::Warning);
        assert!(!enabled.ignore_publish_false);

        let disabled = CheckPolicy::disabled();
        assert!(!disabled.enabled);
        assert_eq!(disabled.severity, Severity::Info);
    }

    #[test]
    fn effective_config_filters_disabled_checks() {
        let mut checks = BTreeMap::new();
        checks.insert(
            "enabled".to_string(),
            CheckPolicy::enabled(Severity::Warning),
        );
        checks.insert("disabled".to_string(), CheckPolicy::disabled());

        let cfg = EffectiveConfig {
            profile: "test".to_string(),
            scope: Scope::Repo,
            fail_on: FailOn::Error,
            max_findings: 10,
            checks,
        };

        assert!(cfg.check_policy("enabled").is_some());
        assert!(cfg.check_policy("disabled").is_none());
        assert!(cfg.check_policy("missing").is_none());
    }
}
