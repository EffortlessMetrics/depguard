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

    // ==========================================================================
    // Determinism tests for stable findings ordering
    // ==========================================================================
    //
    // According to CLAUDE.md, findings must be ordered deterministically:
    // severity -> path -> line -> check_id -> code -> message
    //
    // These tests verify that the sorting is:
    // 1. Deterministic (same input always produces same output)
    // 2. Stable (insertion order doesn't affect final order)
    // 3. Correct (follows the documented priority)

    /// Helper to create a Finding with specific attributes for ordering tests.
    fn make_finding(
        severity: Severity,
        path: Option<&str>,
        line: Option<u32>,
        check_id: &str,
        code: &str,
        message: &str,
    ) -> Finding {
        Finding {
            severity,
            check_id: check_id.to_string(),
            code: code.to_string(),
            message: message.to_string(),
            location: path.map(|p| Location {
                path: RepoPath::new(p),
                line,
                col: None,
            }),
            help: None,
            url: None,
            fingerprint: None,
            data: serde_json::Value::Null,
        }
    }

    #[test]
    fn ordering_severity_takes_priority() {
        // Error < Warning < Info (Error comes first)
        let mut findings = [
            make_finding(
                Severity::Info,
                Some("a.toml"),
                Some(1),
                "check",
                "code",
                "msg",
            ),
            make_finding(
                Severity::Error,
                Some("z.toml"),
                Some(99),
                "check",
                "code",
                "msg",
            ),
            make_finding(
                Severity::Warning,
                Some("m.toml"),
                Some(50),
                "check",
                "code",
                "msg",
            ),
        ];

        findings.sort_by(compare_findings);

        assert_eq!(findings[0].severity, Severity::Error);
        assert_eq!(findings[1].severity, Severity::Warning);
        assert_eq!(findings[2].severity, Severity::Info);
    }

    #[test]
    fn ordering_path_is_second_priority() {
        // Same severity, different paths -> alphabetical path order
        let mut findings = [
            make_finding(
                Severity::Error,
                Some("crates/z/Cargo.toml"),
                Some(1),
                "check",
                "code",
                "msg",
            ),
            make_finding(
                Severity::Error,
                Some("Cargo.toml"),
                Some(1),
                "check",
                "code",
                "msg",
            ),
            make_finding(
                Severity::Error,
                Some("crates/a/Cargo.toml"),
                Some(1),
                "check",
                "code",
                "msg",
            ),
        ];

        findings.sort_by(compare_findings);

        assert_eq!(
            findings[0].location.as_ref().unwrap().path.as_str(),
            "Cargo.toml"
        );
        assert_eq!(
            findings[1].location.as_ref().unwrap().path.as_str(),
            "crates/a/Cargo.toml"
        );
        assert_eq!(
            findings[2].location.as_ref().unwrap().path.as_str(),
            "crates/z/Cargo.toml"
        );
    }

    #[test]
    fn ordering_line_is_third_priority() {
        // Same severity and path, different lines -> numeric line order
        let mut findings = [
            make_finding(
                Severity::Warning,
                Some("Cargo.toml"),
                Some(100),
                "check",
                "code",
                "msg",
            ),
            make_finding(
                Severity::Warning,
                Some("Cargo.toml"),
                Some(1),
                "check",
                "code",
                "msg",
            ),
            make_finding(
                Severity::Warning,
                Some("Cargo.toml"),
                Some(50),
                "check",
                "code",
                "msg",
            ),
        ];

        findings.sort_by(compare_findings);

        assert_eq!(findings[0].location.as_ref().unwrap().line, Some(1));
        assert_eq!(findings[1].location.as_ref().unwrap().line, Some(50));
        assert_eq!(findings[2].location.as_ref().unwrap().line, Some(100));
    }

    #[test]
    fn ordering_check_id_is_fourth_priority() {
        // Same severity, path, line -> alphabetical check_id
        let mut findings = [
            make_finding(
                Severity::Error,
                Some("Cargo.toml"),
                Some(10),
                "deps.z_check",
                "code",
                "msg",
            ),
            make_finding(
                Severity::Error,
                Some("Cargo.toml"),
                Some(10),
                "deps.a_check",
                "code",
                "msg",
            ),
            make_finding(
                Severity::Error,
                Some("Cargo.toml"),
                Some(10),
                "deps.m_check",
                "code",
                "msg",
            ),
        ];

        findings.sort_by(compare_findings);

        assert_eq!(findings[0].check_id, "deps.a_check");
        assert_eq!(findings[1].check_id, "deps.m_check");
        assert_eq!(findings[2].check_id, "deps.z_check");
    }

    #[test]
    fn ordering_code_is_fifth_priority() {
        // Same severity, path, line, check_id -> alphabetical code
        let mut findings = [
            make_finding(
                Severity::Error,
                Some("Cargo.toml"),
                Some(10),
                "deps.check",
                "z_code",
                "msg",
            ),
            make_finding(
                Severity::Error,
                Some("Cargo.toml"),
                Some(10),
                "deps.check",
                "a_code",
                "msg",
            ),
            make_finding(
                Severity::Error,
                Some("Cargo.toml"),
                Some(10),
                "deps.check",
                "m_code",
                "msg",
            ),
        ];

        findings.sort_by(compare_findings);

        assert_eq!(findings[0].code, "a_code");
        assert_eq!(findings[1].code, "m_code");
        assert_eq!(findings[2].code, "z_code");
    }

    #[test]
    fn ordering_message_is_last_priority() {
        // Same severity, path, line, check_id, code -> alphabetical message
        let mut findings = [
            make_finding(
                Severity::Error,
                Some("Cargo.toml"),
                Some(10),
                "deps.check",
                "code",
                "zebra",
            ),
            make_finding(
                Severity::Error,
                Some("Cargo.toml"),
                Some(10),
                "deps.check",
                "code",
                "apple",
            ),
            make_finding(
                Severity::Error,
                Some("Cargo.toml"),
                Some(10),
                "deps.check",
                "code",
                "mango",
            ),
        ];

        findings.sort_by(compare_findings);

        assert_eq!(findings[0].message, "apple");
        assert_eq!(findings[1].message, "mango");
        assert_eq!(findings[2].message, "zebra");
    }

    #[test]
    fn ordering_missing_location_comes_last() {
        // Findings without location should sort after those with location
        let mut findings = [
            make_finding(Severity::Error, None, None, "check", "code", "no location"),
            make_finding(
                Severity::Error,
                Some("a.toml"),
                Some(1),
                "check",
                "code",
                "has location",
            ),
        ];

        findings.sort_by(compare_findings);

        assert!(findings[0].location.is_some());
        assert!(findings[1].location.is_none());
    }

    #[test]
    fn ordering_missing_line_comes_last_within_path() {
        // Findings without line should sort after those with line (same path)
        let mut findings = [
            make_finding(
                Severity::Error,
                Some("Cargo.toml"),
                None,
                "check",
                "code",
                "no line",
            ),
            make_finding(
                Severity::Error,
                Some("Cargo.toml"),
                Some(1),
                "check",
                "code",
                "has line",
            ),
            make_finding(
                Severity::Error,
                Some("Cargo.toml"),
                Some(999),
                "check",
                "code",
                "high line",
            ),
        ];

        findings.sort_by(compare_findings);

        assert_eq!(findings[0].location.as_ref().unwrap().line, Some(1));
        assert_eq!(findings[1].location.as_ref().unwrap().line, Some(999));
        assert_eq!(findings[2].location.as_ref().unwrap().line, None);
    }

    #[test]
    fn ordering_is_deterministic_across_multiple_sorts() {
        // Create findings in a specific order
        let original = vec![
            make_finding(Severity::Info, Some("z.toml"), Some(1), "z", "z", "z"),
            make_finding(Severity::Error, Some("a.toml"), Some(1), "a", "a", "a"),
            make_finding(Severity::Warning, Some("m.toml"), Some(1), "m", "m", "m"),
            make_finding(Severity::Error, Some("a.toml"), Some(2), "b", "b", "b"),
            make_finding(Severity::Error, Some("b.toml"), Some(1), "a", "a", "a"),
        ];

        // Sort multiple times and verify identical results
        let mut sorted1 = original.clone();
        let mut sorted2 = original.clone();
        let mut sorted3 = original.clone();

        sorted1.sort_by(compare_findings);
        sorted2.sort_by(compare_findings);
        sorted3.sort_by(compare_findings);

        for i in 0..sorted1.len() {
            assert_eq!(sorted1[i].severity, sorted2[i].severity);
            assert_eq!(sorted1[i].severity, sorted3[i].severity);
            assert_eq!(sorted1[i].check_id, sorted2[i].check_id);
            assert_eq!(sorted1[i].check_id, sorted3[i].check_id);
            assert_eq!(sorted1[i].code, sorted2[i].code);
            assert_eq!(sorted1[i].code, sorted3[i].code);
            assert_eq!(sorted1[i].message, sorted2[i].message);
            assert_eq!(sorted1[i].message, sorted3[i].message);
        }
    }

    #[test]
    fn ordering_is_stable_regardless_of_insertion_order() {
        // Create the same findings but in different insertion orders
        let findings_a = vec![
            make_finding(
                Severity::Error,
                Some("a.toml"),
                Some(1),
                "check1",
                "code1",
                "msg1",
            ),
            make_finding(
                Severity::Warning,
                Some("b.toml"),
                Some(2),
                "check2",
                "code2",
                "msg2",
            ),
            make_finding(
                Severity::Info,
                Some("c.toml"),
                Some(3),
                "check3",
                "code3",
                "msg3",
            ),
        ];

        let findings_b = vec![
            make_finding(
                Severity::Info,
                Some("c.toml"),
                Some(3),
                "check3",
                "code3",
                "msg3",
            ),
            make_finding(
                Severity::Error,
                Some("a.toml"),
                Some(1),
                "check1",
                "code1",
                "msg1",
            ),
            make_finding(
                Severity::Warning,
                Some("b.toml"),
                Some(2),
                "check2",
                "code2",
                "msg2",
            ),
        ];

        let findings_c = vec![
            make_finding(
                Severity::Warning,
                Some("b.toml"),
                Some(2),
                "check2",
                "code2",
                "msg2",
            ),
            make_finding(
                Severity::Info,
                Some("c.toml"),
                Some(3),
                "check3",
                "code3",
                "msg3",
            ),
            make_finding(
                Severity::Error,
                Some("a.toml"),
                Some(1),
                "check1",
                "code1",
                "msg1",
            ),
        ];

        let mut sorted_a = findings_a;
        let mut sorted_b = findings_b;
        let mut sorted_c = findings_c;

        sorted_a.sort_by(compare_findings);
        sorted_b.sort_by(compare_findings);
        sorted_c.sort_by(compare_findings);

        // All should have Error first, Warning second, Info third
        for (a, b, c) in sorted_a
            .iter()
            .zip(sorted_b.iter())
            .zip(sorted_c.iter())
            .map(|((a, b), c)| (a, b, c))
        {
            assert_eq!(a.severity, b.severity);
            assert_eq!(b.severity, c.severity);
            assert_eq!(a.check_id, b.check_id);
            assert_eq!(b.check_id, c.check_id);
            assert_eq!(a.message, b.message);
            assert_eq!(b.message, c.message);
        }
    }

    #[test]
    fn ordering_complex_mixed_scenario() {
        // A complex scenario with multiple attributes varying
        let mut findings = [
            // Error findings
            make_finding(
                Severity::Error,
                Some("Cargo.toml"),
                Some(10),
                "deps.wildcards",
                "wildcard",
                "dep1",
            ),
            make_finding(
                Severity::Error,
                Some("Cargo.toml"),
                Some(5),
                "deps.wildcards",
                "wildcard",
                "dep2",
            ),
            make_finding(
                Severity::Error,
                Some("crates/a/Cargo.toml"),
                Some(1),
                "deps.path",
                "absolute",
                "dep3",
            ),
            // Warning findings
            make_finding(
                Severity::Warning,
                Some("Cargo.toml"),
                Some(1),
                "deps.wildcards",
                "wildcard",
                "dep4",
            ),
            make_finding(
                Severity::Warning,
                Some("Cargo.toml"),
                Some(1),
                "deps.path",
                "escape",
                "dep5",
            ),
            // Info findings
            make_finding(
                Severity::Info,
                Some("Cargo.toml"),
                Some(1),
                "deps.info",
                "hint",
                "dep6",
            ),
            // No location
            make_finding(
                Severity::Error,
                None,
                None,
                "deps.error",
                "code",
                "no location",
            ),
        ];

        findings.sort_by(compare_findings);

        // Verify the order: Error (Cargo.toml:5, Cargo.toml:10, crates/a:1, no loc) -> Warning -> Info
        assert_eq!(findings[0].severity, Severity::Error);
        assert_eq!(findings[0].location.as_ref().unwrap().line, Some(5));

        assert_eq!(findings[1].severity, Severity::Error);
        assert_eq!(findings[1].location.as_ref().unwrap().line, Some(10));

        assert_eq!(findings[2].severity, Severity::Error);
        assert_eq!(
            findings[2].location.as_ref().unwrap().path.as_str(),
            "crates/a/Cargo.toml"
        );

        // Error with no location should come after errors with location (path "~" sorts after real paths)
        assert_eq!(findings[3].severity, Severity::Error);
        assert!(findings[3].location.is_none());

        // Warning findings sorted by check_id
        assert_eq!(findings[4].severity, Severity::Warning);
        assert_eq!(findings[4].check_id, "deps.path");

        assert_eq!(findings[5].severity, Severity::Warning);
        assert_eq!(findings[5].check_id, "deps.wildcards");

        // Info last
        assert_eq!(findings[6].severity, Severity::Info);
    }

    #[test]
    fn ordering_through_evaluate_is_deterministic() {
        // Test that the full evaluate() function produces deterministic ordering
        let model = WorkspaceModel {
            repo_root: RepoPath::new("."),
            workspace_dependencies: BTreeMap::new(),
            manifests: vec![ManifestModel {
                path: RepoPath::new("Cargo.toml"),
                package: Some(PackageMeta {
                    name: "root".to_string(),
                    publish: true,
                }),
                features: BTreeMap::new(),
                dependencies: vec![
                    // Create multiple violations in non-sorted order
                    DependencyDecl {
                        kind: DepKind::Normal,
                        name: "dep_z".to_string(),
                        spec: DepSpec {
                            version: Some("*".to_string()),
                            ..DepSpec::default()
                        },
                        location: Some(Location {
                            path: RepoPath::new("Cargo.toml"),
                            line: Some(20),
                            col: None,
                        }),
                    },
                    DependencyDecl {
                        kind: DepKind::Normal,
                        name: "dep_a".to_string(),
                        spec: DepSpec {
                            version: Some("*".to_string()),
                            ..DepSpec::default()
                        },
                        location: Some(Location {
                            path: RepoPath::new("Cargo.toml"),
                            line: Some(10),
                            col: None,
                        }),
                    },
                    DependencyDecl {
                        kind: DepKind::Normal,
                        name: "dep_m".to_string(),
                        spec: DepSpec {
                            version: Some("*".to_string()),
                            ..DepSpec::default()
                        },
                        location: Some(Location {
                            path: RepoPath::new("Cargo.toml"),
                            line: Some(15),
                            col: None,
                        }),
                    },
                ],
            }],
        };

        let mut checks = BTreeMap::new();
        checks.insert(
            depguard_types::ids::CHECK_DEPS_NO_WILDCARDS.to_string(),
            CheckPolicy::enabled(Severity::Warning),
        );

        let cfg = EffectiveConfig {
            profile: "test".to_string(),
            scope: Scope::Repo,
            fail_on: FailOn::Error,
            max_findings: 200,
            checks,
        };

        // Evaluate multiple times
        let report1 = evaluate(&model, &cfg);
        let report2 = evaluate(&model, &cfg);
        let report3 = evaluate(&model, &cfg);

        // All reports should have identical findings in identical order
        assert_eq!(report1.findings.len(), report2.findings.len());
        assert_eq!(report2.findings.len(), report3.findings.len());

        for i in 0..report1.findings.len() {
            assert_eq!(report1.findings[i].message, report2.findings[i].message);
            assert_eq!(report2.findings[i].message, report3.findings[i].message);
        }

        // Verify the order is by line number (10, 15, 20)
        assert!(report1.findings[0].message.contains("dep_a"));
        assert!(report1.findings[1].message.contains("dep_m"));
        assert!(report1.findings[2].message.contains("dep_z"));
    }

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
                features: BTreeMap::new(),
                dependencies: vec![DependencyDecl {
                    kind: DepKind::Normal,
                    name: "serde".to_string(),
                    spec: DepSpec {
                        version: Some("*".to_string()),
                        ..DepSpec::default()
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
