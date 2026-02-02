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
}

impl CheckPolicy {
    pub fn enabled(severity: Severity) -> Self {
        Self {
            enabled: true,
            severity,
            allow: Vec::new(),
        }
    }

    pub fn disabled() -> Self {
        Self {
            enabled: false,
            severity: Severity::Info,
            allow: Vec::new(),
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
