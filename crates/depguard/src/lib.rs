//! Public facade over depguard's pure evaluation engine.

#![forbid(unsafe_code)]

pub mod checks {
    pub use depguard_domain::checks::*;
}

pub mod model {
    pub use depguard_domain::model::*;
}

pub mod policy {
    pub use depguard_domain::policy::*;
}

pub mod report {
    pub use depguard_domain::report::*;
}

pub use checks::run_all;
pub use depguard_domain::evaluate;
pub use model::*;
pub use policy::*;
pub use report::{DomainReport, SeverityCounts};

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn empty_cfg() -> EffectiveConfig {
        EffectiveConfig {
            profile: "strict".to_string(),
            scope: Scope::Repo,
            fail_on: FailOn::Error,
            max_findings: 100,
            yanked_index: None,
            checks: BTreeMap::new(),
        }
    }

    #[test]
    fn root_reexports_support_evaluate() {
        let model = WorkspaceModel::default();
        let report: DomainReport = evaluate(&model, &empty_cfg());
        assert!(report.findings.is_empty());
    }

    #[test]
    fn module_paths_are_available() {
        let model = model::WorkspaceModel::default();
        let cfg = policy::EffectiveConfig {
            profile: "strict".to_string(),
            scope: policy::Scope::Repo,
            fail_on: policy::FailOn::Error,
            max_findings: 100,
            yanked_index: None,
            checks: BTreeMap::new(),
        };
        let mut findings = Vec::new();
        checks::run_all(&model, &cfg, &mut findings);
        let _counts = report::SeverityCounts::default();
        assert!(findings.is_empty());
    }
}
