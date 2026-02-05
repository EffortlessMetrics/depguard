use crate::checks;
use crate::model::WorkspaceModel;
use crate::policy::{EffectiveConfig, FailOn};
use crate::report::{DomainReport, SeverityCounts};
use depguard_types::{DepguardData, Finding, Severity, Verdict};

pub fn evaluate(model: &WorkspaceModel, cfg: &EffectiveConfig) -> DomainReport {
    let mut findings: Vec<Finding> = Vec::new();

    checks::run_all(model, cfg, &mut findings);

    // Deterministic ordering before truncation.
    findings.sort_by(compare_findings);

    let total = findings.len() as u32;

    let mut emitted = findings;
    let mut truncated_reason: Option<String> = None;
    if emitted.len() > cfg.max_findings {
        emitted.truncate(cfg.max_findings);
        truncated_reason = Some(format!(
            "findings truncated to max_findings={}",
            cfg.max_findings
        ));
    }

    let verdict = compute_verdict(&emitted, cfg.fail_on);
    let counts = SeverityCounts::from_findings(&emitted);

    let data = DepguardData {
        scope: match cfg.scope {
            crate::policy::Scope::Repo => "repo".to_string(),
            crate::policy::Scope::Diff => "diff".to_string(),
        },
        profile: cfg.profile.clone(),
        manifests_scanned: model.manifests.len() as u32,
        dependencies_scanned: model
            .manifests
            .iter()
            .map(|m| m.dependencies.len() as u32)
            .sum(),
        findings_total: total,
        findings_emitted: emitted.len() as u32,
        truncated_reason,
    };

    DomainReport {
        verdict,
        findings: emitted,
        data,
        counts,
    }
}

fn compute_verdict(findings: &[Finding], fail_on: FailOn) -> Verdict {
    let has_error = findings.iter().any(|f| f.severity == Severity::Error);
    if has_error {
        return Verdict::Fail;
    }

    let has_warn = findings.iter().any(|f| f.severity == Severity::Warning);
    if has_warn {
        return match fail_on {
            FailOn::Warning => Verdict::Fail,
            FailOn::Error => Verdict::Warn,
        };
    }

    Verdict::Pass
}

fn compare_findings(a: &Finding, b: &Finding) -> std::cmp::Ordering {
    // Ordering priority:
    // 1) severity (error -> warning -> info)
    // 2) location.path (missing last)
    // 3) location.line (missing last)
    // 4) check_id
    // 5) code
    // 6) message
    let severity_rank = |sev: Severity| match sev {
        Severity::Error => 0,
        Severity::Warning => 1,
        Severity::Info => 2,
    };
    let (ap, al) = match &a.location {
        Some(l) => (l.path.as_str(), l.line.unwrap_or(u32::MAX)),
        None => ("~", u32::MAX),
    };
    let (bp, bl) = match &b.location {
        Some(l) => (l.path.as_str(), l.line.unwrap_or(u32::MAX)),
        None => ("~", u32::MAX),
    };

    severity_rank(a.severity)
        .cmp(&severity_rank(b.severity))
        .then(ap.cmp(bp))
        .then(al.cmp(&bl))
        .then(a.check_id.cmp(&b.check_id))
        .then(a.code.cmp(&b.code))
        .then(a.message.cmp(&b.message))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        DepKind, DepSpec, DependencyDecl, ManifestModel, PackageMeta, WorkspaceModel,
    };
    use crate::policy::{CheckPolicy, EffectiveConfig, FailOn, Scope};
    use depguard_types::{Location, RepoPath, Severity};
    use std::collections::BTreeMap;

    #[test]
    fn verdict_warn_becomes_fail_when_fail_on_warning() {
        let model = WorkspaceModel {
            repo_root: RepoPath::new("."),
            workspace_dependencies: BTreeMap::new(),
            manifests: vec![ManifestModel {
                path: RepoPath::new("Cargo.toml"),
                package: Some(PackageMeta {
                    name: "root".to_string(),
                    publish: true,
                }),
                dependencies: vec![DependencyDecl {
                    kind: DepKind::Normal,
                    name: "serde".to_string(),
                    spec: DepSpec {
                        version: Some("*".to_string()),
                        path: None,
                        workspace: false,
                    },
                    location: Some(Location {
                        path: RepoPath::new("Cargo.toml"),
                        line: Some(1),
                        col: None,
                    }),
                }],
            }],
        };

        let mut checks = BTreeMap::new();
        checks.insert(
            depguard_types::ids::CHECK_DEPS_NO_WILDCARDS.to_string(),
            CheckPolicy::enabled(Severity::Warning),
        );

        let cfg = EffectiveConfig {
            profile: "warn".to_string(),
            scope: Scope::Repo,
            fail_on: FailOn::Warning,
            max_findings: 200,
            checks,
        };

        let report = evaluate(&model, &cfg);
        assert_eq!(report.verdict, Verdict::Fail);
    }
}
