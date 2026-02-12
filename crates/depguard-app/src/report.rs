use anyhow::Context;
use depguard_render::{
    RenderableData, RenderableFinding, RenderableLocation, RenderableReport, RenderableSeverity,
    RenderableVerdictStatus,
};
use depguard_types::{
    ArtifactPointer, Capabilities, CapabilityAvailability, CapabilityStatus, DepguardData,
    DepguardReportV1, DepguardReportV2, FindingV2, SCHEMA_REPORT_V1, SCHEMA_REPORT_V2,
    SCHEMA_SENSOR_REPORT_V1, Severity, SeverityV2, Verdict, VerdictStatus,
};
use time::OffsetDateTime;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReportVersion {
    V1,
    V2,
    /// Universal sensor.report.v1 format for cockpit ecosystem.
    SensorV1,
}

#[derive(Clone, Debug)]
#[allow(clippy::large_enum_variant)]
pub enum ReportVariant {
    V1(DepguardReportV1),
    V2(DepguardReportV2),
}

pub fn parse_report_json(text: &str) -> anyhow::Result<ReportVariant> {
    let value: serde_json::Value = serde_json::from_str(text).context("parse report json")?;

    let schema = value
        .get("schema")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();

    match schema.as_str() {
        SCHEMA_REPORT_V2 | SCHEMA_SENSOR_REPORT_V1 => {
            let report: DepguardReportV2 =
                serde_json::from_value(value).context("parse depguard v2 report")?;
            Ok(ReportVariant::V2(report))
        }
        SCHEMA_REPORT_V1 | "receipt.envelope.v1" => {
            let report: DepguardReportV1 =
                serde_json::from_value(value).context("parse depguard v1 report")?;
            Ok(ReportVariant::V1(report))
        }
        _ => {
            // Fallback: try v2 then v1 for forward/back compat.
            if let Ok(report) = serde_json::from_value::<DepguardReportV2>(value.clone()) {
                Ok(ReportVariant::V2(report))
            } else if let Ok(report) = serde_json::from_value::<DepguardReportV1>(value) {
                Ok(ReportVariant::V1(report))
            } else {
                anyhow::bail!("unknown report schema: {schema}")
            }
        }
    }
}

pub fn serialize_report(report: &ReportVariant) -> anyhow::Result<Vec<u8>> {
    match report {
        ReportVariant::V1(r) => serde_json::to_vec_pretty(r).context("serialize v1 report"),
        ReportVariant::V2(r) => serde_json::to_vec_pretty(r).context("serialize v2 report"),
    }
}

pub fn to_renderable(report: &ReportVariant) -> RenderableReport {
    match report {
        ReportVariant::V1(r) => RenderableReport {
            verdict: match r.verdict {
                Verdict::Pass => RenderableVerdictStatus::Pass,
                Verdict::Warn => RenderableVerdictStatus::Warn,
                Verdict::Fail => RenderableVerdictStatus::Fail,
            },
            findings: r.findings.iter().map(renderable_from_v1).collect(),
            data: RenderableData {
                findings_emitted: r.data.findings_emitted,
                findings_total: r.data.findings_total,
                truncated_reason: r.data.truncated_reason.clone(),
            },
        },
        ReportVariant::V2(r) => RenderableReport {
            verdict: match r.verdict.status {
                VerdictStatus::Pass => RenderableVerdictStatus::Pass,
                VerdictStatus::Warn => RenderableVerdictStatus::Warn,
                VerdictStatus::Fail => RenderableVerdictStatus::Fail,
                VerdictStatus::Skip => RenderableVerdictStatus::Skip,
            },
            findings: r.findings.iter().map(renderable_from_v2).collect(),
            data: RenderableData {
                findings_emitted: r.data.findings_emitted,
                findings_total: r.data.findings_total,
                truncated_reason: r.data.truncated_reason.clone(),
            },
        },
    }
}

fn renderable_from_v1(f: &depguard_types::Finding) -> RenderableFinding {
    RenderableFinding {
        severity: match f.severity {
            Severity::Info => RenderableSeverity::Info,
            Severity::Warning => RenderableSeverity::Warning,
            Severity::Error => RenderableSeverity::Error,
        },
        check_id: Some(f.check_id.clone()),
        code: f.code.clone(),
        message: f.message.clone(),
        location: f.location.as_ref().map(|loc| RenderableLocation {
            path: loc.path.as_str().to_string(),
            line: loc.line,
            col: loc.col,
        }),
        help: f.help.clone(),
        url: f.url.clone(),
    }
}

fn renderable_from_v2(f: &FindingV2) -> RenderableFinding {
    RenderableFinding {
        severity: match f.severity {
            SeverityV2::Info => RenderableSeverity::Info,
            SeverityV2::Warn => RenderableSeverity::Warning,
            SeverityV2::Error => RenderableSeverity::Error,
        },
        check_id: Some(f.check_id.clone()),
        code: f.code.clone(),
        message: f.message.clone(),
        location: f.location.as_ref().map(|loc| RenderableLocation {
            path: loc.path.as_str().to_string(),
            line: loc.line,
            col: loc.col,
        }),
        help: f.help.clone(),
        url: f.url.clone(),
    }
}

pub fn empty_report(version: ReportVersion, scope: &str, profile: &str) -> ReportVariant {
    let data = DepguardData {
        scope: scope.to_string(),
        profile: profile.to_string(),
        manifests_scanned: 0,
        dependencies_scanned: 0,
        findings_total: 0,
        findings_emitted: 0,
        truncated_reason: None,
    };

    let now = OffsetDateTime::now_utc();

    match version {
        ReportVersion::V1 => ReportVariant::V1(DepguardReportV1 {
            schema: SCHEMA_REPORT_V1.to_string(),
            tool: depguard_types::ToolMeta {
                name: "depguard".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            started_at: now,
            finished_at: now,
            verdict: Verdict::Pass,
            findings: Vec::new(),
            data,
        }),
        ReportVersion::V2 | ReportVersion::SensorV1 => {
            let schema = match version {
                ReportVersion::SensorV1 => SCHEMA_SENSOR_REPORT_V1.to_string(),
                _ => SCHEMA_REPORT_V2.to_string(),
            };
            let capabilities = if version == ReportVersion::SensorV1 {
                Some(Capabilities {
                    git: Some(CapabilityStatus {
                        status: CapabilityAvailability::Missing,
                        reason: Some(depguard_types::ids::REASON_NO_MANIFEST_FOUND.to_string()),
                    }),
                    config: Some(CapabilityStatus {
                        status: CapabilityAvailability::Available,
                        reason: None,
                    }),
                })
            } else {
                None
            };
            ReportVariant::V2(DepguardReportV2 {
                schema,
                tool: depguard_types::ToolMetaV2 {
                    name: "depguard".to_string(),
                    version: env!("CARGO_PKG_VERSION").to_string(),
                    commit: None,
                },
                run: depguard_types::RunMeta {
                    started_at: now,
                    ended_at: Some(now),
                    duration_ms: Some(0),
                    host: None,
                    ci: None,
                    git: None,
                    capabilities,
                },
                verdict: depguard_types::VerdictV2 {
                    status: VerdictStatus::Pass,
                    counts: depguard_types::VerdictCounts {
                        info: 0,
                        warn: 0,
                        error: 0,
                        suppressed: 0,
                    },
                    reasons: Vec::new(),
                },
                findings: Vec::new(),
                artifacts: None,
                data,
            })
        }
    }
}

pub fn runtime_error_report(version: ReportVersion, message: &str) -> ReportVariant {
    let now = OffsetDateTime::now_utc();
    let data = DepguardData {
        scope: "repo".to_string(),
        profile: "unknown".to_string(),
        manifests_scanned: 0,
        dependencies_scanned: 0,
        findings_total: 1,
        findings_emitted: 1,
        truncated_reason: None,
    };

    match version {
        ReportVersion::V1 => ReportVariant::V1(DepguardReportV1 {
            schema: SCHEMA_REPORT_V1.to_string(),
            tool: depguard_types::ToolMeta {
                name: "depguard".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            started_at: now,
            finished_at: now,
            verdict: Verdict::Fail,
            findings: vec![depguard_types::Finding {
                severity: depguard_types::Severity::Error,
                check_id: depguard_types::ids::CHECK_TOOL_RUNTIME.to_string(),
                code: depguard_types::ids::CODE_RUNTIME_ERROR.to_string(),
                message: message.to_string(),
                location: None,
                help: Some("Fix the tool error and re-run depguard.".to_string()),
                url: None,
                fingerprint: None,
                data: serde_json::Value::Null,
            }],
            data,
        }),
        ReportVersion::V2 | ReportVersion::SensorV1 => {
            let schema = match version {
                ReportVersion::SensorV1 => SCHEMA_SENSOR_REPORT_V1.to_string(),
                _ => SCHEMA_REPORT_V2.to_string(),
            };
            let capabilities = if version == ReportVersion::SensorV1 {
                Some(Capabilities {
                    git: Some(CapabilityStatus {
                        status: CapabilityAvailability::Missing,
                        reason: Some(depguard_types::ids::REASON_RUNTIME_ERROR.to_string()),
                    }),
                    config: Some(CapabilityStatus {
                        status: CapabilityAvailability::Missing,
                        reason: Some(depguard_types::ids::REASON_RUNTIME_ERROR.to_string()),
                    }),
                })
            } else {
                None
            };
            ReportVariant::V2(DepguardReportV2 {
                schema,
                tool: depguard_types::ToolMetaV2 {
                    name: "depguard".to_string(),
                    version: env!("CARGO_PKG_VERSION").to_string(),
                    commit: None,
                },
                run: depguard_types::RunMeta {
                    started_at: now,
                    ended_at: Some(now),
                    duration_ms: Some(0),
                    host: None,
                    ci: None,
                    git: None,
                    capabilities,
                },
                verdict: depguard_types::VerdictV2 {
                    status: VerdictStatus::Fail,
                    counts: depguard_types::VerdictCounts {
                        info: 0,
                        warn: 0,
                        error: 1,
                        suppressed: 0,
                    },
                    reasons: vec!["tool_error".to_string()],
                },
                findings: vec![depguard_types::FindingV2 {
                    severity: depguard_types::SeverityV2::Error,
                    check_id: depguard_types::ids::CHECK_TOOL_RUNTIME.to_string(),
                    code: depguard_types::ids::CODE_RUNTIME_ERROR.to_string(),
                    message: message.to_string(),
                    location: None,
                    help: Some("Fix the tool error and re-run depguard.".to_string()),
                    url: None,
                    fingerprint: None,
                    data: serde_json::Value::Null,
                }],
                artifacts: None,
                data,
            })
        }
    }
}

pub fn add_artifact(report: &mut ReportVariant, artifact: ArtifactPointer) {
    if let ReportVariant::V2(r) = report {
        r.artifacts.get_or_insert_with(Vec::new).push(artifact);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use depguard_types::{
        DepguardReportV1, DepguardReportV2, Finding, FindingV2, Location, RepoPath, RunMeta,
        Severity, SeverityV2, ToolMeta, ToolMetaV2, Verdict, VerdictCounts, VerdictStatus,
        VerdictV2, ids,
    };
    use time::OffsetDateTime;

    fn sample_data() -> DepguardData {
        DepguardData {
            scope: "repo".to_string(),
            profile: "strict".to_string(),
            manifests_scanned: 1,
            dependencies_scanned: 1,
            findings_total: 1,
            findings_emitted: 1,
            truncated_reason: None,
        }
    }

    fn sample_v1(schema: &str) -> DepguardReportV1 {
        sample_v1_with(schema, Verdict::Warn, Severity::Warning)
    }

    fn sample_v1_with(schema: &str, verdict: Verdict, severity: Severity) -> DepguardReportV1 {
        DepguardReportV1 {
            schema: schema.to_string(),
            tool: ToolMeta {
                name: "depguard".to_string(),
                version: "0.0.0".to_string(),
            },
            started_at: OffsetDateTime::UNIX_EPOCH,
            finished_at: OffsetDateTime::UNIX_EPOCH,
            verdict,
            findings: vec![Finding {
                severity,
                check_id: "deps.no_wildcards".to_string(),
                code: "wildcard_version".to_string(),
                message: "bad".to_string(),
                location: Some(Location {
                    path: RepoPath::new("Cargo.toml"),
                    line: Some(1),
                    col: None,
                }),
                help: None,
                url: None,
                fingerprint: None,
                data: serde_json::Value::Null,
            }],
            data: sample_data(),
        }
    }

    fn sample_v2(schema: &str, status: VerdictStatus, severity: SeverityV2) -> DepguardReportV2 {
        DepguardReportV2 {
            schema: schema.to_string(),
            tool: ToolMetaV2 {
                name: "depguard".to_string(),
                version: "0.0.0".to_string(),
                commit: None,
            },
            run: RunMeta {
                started_at: OffsetDateTime::UNIX_EPOCH,
                ended_at: Some(OffsetDateTime::UNIX_EPOCH),
                duration_ms: Some(0),
                host: None,
                ci: None,
                git: None,
                capabilities: None,
            },
            verdict: VerdictV2 {
                status,
                counts: VerdictCounts {
                    info: 0,
                    warn: 0,
                    error: if matches!(severity, SeverityV2::Error) {
                        1
                    } else {
                        0
                    },
                    suppressed: 0,
                },
                reasons: Vec::new(),
            },
            findings: vec![FindingV2 {
                severity,
                check_id: "deps.no_wildcards".to_string(),
                code: "wildcard_version".to_string(),
                message: "bad".to_string(),
                location: Some(Location {
                    path: RepoPath::new("Cargo.toml"),
                    line: Some(1),
                    col: None,
                }),
                help: None,
                url: None,
                fingerprint: None,
                data: serde_json::Value::Null,
            }],
            artifacts: None,
            data: sample_data(),
        }
    }

    #[test]
    fn parse_report_json_recognizes_versions() {
        let v1 = sample_v1("receipt.envelope.v1");
        let parsed = parse_report_json(&serde_json::to_string(&v1).unwrap()).unwrap();
        assert!(matches!(parsed, ReportVariant::V1(_)));

        let v2 = sample_v2(SCHEMA_REPORT_V2, VerdictStatus::Warn, SeverityV2::Warn);
        let parsed = parse_report_json(&serde_json::to_string(&v2).unwrap()).unwrap();
        assert!(matches!(parsed, ReportVariant::V2(_)));

        let sensor = sample_v2(
            SCHEMA_SENSOR_REPORT_V1,
            VerdictStatus::Pass,
            SeverityV2::Info,
        );
        let parsed = parse_report_json(&serde_json::to_string(&sensor).unwrap()).unwrap();
        assert!(matches!(parsed, ReportVariant::V2(_)));
    }

    #[test]
    fn parse_report_json_fallbacks_work() {
        let mut v2 = sample_v2("custom.schema", VerdictStatus::Pass, SeverityV2::Info);
        v2.schema = "custom.schema".to_string();
        let parsed = parse_report_json(&serde_json::to_string(&v2).unwrap()).unwrap();
        assert!(matches!(parsed, ReportVariant::V2(_)));

        let mut v1 = sample_v1("custom.v1");
        v1.schema = "custom.v1".to_string();
        let parsed = parse_report_json(&serde_json::to_string(&v1).unwrap()).unwrap();
        assert!(matches!(parsed, ReportVariant::V1(_)));
    }

    #[test]
    fn parse_report_json_unknown_schema_errors() {
        let text = r#"{"schema":"unknown"}"#;
        let err = parse_report_json(text).unwrap_err();
        assert!(err.to_string().contains("unknown report schema"));
    }

    #[test]
    fn serialize_report_smoke() {
        let v1 = ReportVariant::V1(sample_v1(SCHEMA_REPORT_V1));
        let bytes = serialize_report(&v1).unwrap();
        let text = String::from_utf8(bytes).unwrap();
        assert!(text.contains(SCHEMA_REPORT_V1));

        let v2 = ReportVariant::V2(sample_v2(
            SCHEMA_REPORT_V2,
            VerdictStatus::Pass,
            SeverityV2::Info,
        ));
        let bytes = serialize_report(&v2).unwrap();
        let text = String::from_utf8(bytes).unwrap();
        assert!(text.contains(SCHEMA_REPORT_V2));
    }

    #[test]
    fn to_renderable_maps_severity_and_verdict() {
        let v1 = ReportVariant::V1(sample_v1(SCHEMA_REPORT_V1));
        let renderable = to_renderable(&v1);
        assert_eq!(renderable.verdict, RenderableVerdictStatus::Warn);
        assert_eq!(renderable.findings[0].severity, RenderableSeverity::Warning);

        let v2 = ReportVariant::V2(sample_v2(
            SCHEMA_REPORT_V2,
            VerdictStatus::Skip,
            SeverityV2::Warn,
        ));
        let renderable = to_renderable(&v2);
        assert_eq!(renderable.verdict, RenderableVerdictStatus::Skip);
        assert_eq!(renderable.findings[0].severity, RenderableSeverity::Warning);
    }

    #[test]
    fn to_renderable_covers_all_v1_verdicts_and_severities() {
        let v1_pass = ReportVariant::V1(sample_v1_with(
            SCHEMA_REPORT_V1,
            Verdict::Pass,
            Severity::Info,
        ));
        let renderable = to_renderable(&v1_pass);
        assert_eq!(renderable.verdict, RenderableVerdictStatus::Pass);
        assert_eq!(renderable.findings[0].severity, RenderableSeverity::Info);

        let v1_fail = ReportVariant::V1(sample_v1_with(
            SCHEMA_REPORT_V1,
            Verdict::Fail,
            Severity::Error,
        ));
        let renderable = to_renderable(&v1_fail);
        assert_eq!(renderable.verdict, RenderableVerdictStatus::Fail);
        assert_eq!(renderable.findings[0].severity, RenderableSeverity::Error);
    }

    #[test]
    fn to_renderable_covers_v2_verdicts_and_info_severity() {
        let v2_pass = ReportVariant::V2(sample_v2(
            SCHEMA_REPORT_V2,
            VerdictStatus::Pass,
            SeverityV2::Info,
        ));
        let renderable = to_renderable(&v2_pass);
        assert_eq!(renderable.verdict, RenderableVerdictStatus::Pass);
        assert_eq!(renderable.findings[0].severity, RenderableSeverity::Info);

        let v2_warn = ReportVariant::V2(sample_v2(
            SCHEMA_REPORT_V2,
            VerdictStatus::Warn,
            SeverityV2::Error,
        ));
        let renderable = to_renderable(&v2_warn);
        assert_eq!(renderable.verdict, RenderableVerdictStatus::Warn);
        assert_eq!(renderable.findings[0].severity, RenderableSeverity::Error);
    }

    #[test]
    fn empty_report_versions() {
        let v1 = unwrap_v1(empty_report(ReportVersion::V1, "repo", "strict"));
        assert_eq!(v1.schema, SCHEMA_REPORT_V1);
        assert_eq!(v1.verdict, Verdict::Pass);
        assert_eq!(v1.started_at, v1.finished_at);

        let v2 = unwrap_v2(empty_report(ReportVersion::V2, "repo", "strict"));
        assert_eq!(v2.schema, SCHEMA_REPORT_V2);
        assert!(v2.run.capabilities.is_none());

        let sensor = unwrap_v2(empty_report(ReportVersion::SensorV1, "repo", "strict"));
        assert_eq!(sensor.schema, SCHEMA_SENSOR_REPORT_V1);
        let caps = sensor.run.capabilities.as_ref().expect("caps");
        assert!(caps.git.is_some());
        assert!(caps.config.is_some());
    }

    #[test]
    fn runtime_error_report_versions() {
        let v1 = unwrap_v1(runtime_error_report(ReportVersion::V1, "boom"));
        assert_eq!(v1.verdict, Verdict::Fail);
        assert_eq!(v1.findings.len(), 1);

        let v2 = unwrap_v2(runtime_error_report(ReportVersion::V2, "boom"));
        assert_eq!(v2.verdict.status, VerdictStatus::Fail);
        assert_eq!(v2.findings.len(), 1);
        assert_eq!(v2.verdict.counts.error, 1);

        let sensor = unwrap_v2(runtime_error_report(ReportVersion::SensorV1, "boom"));
        let caps = sensor.run.capabilities.as_ref().expect("caps");
        assert_eq!(
            caps.git.as_ref().unwrap().status,
            CapabilityAvailability::Missing
        );
        assert_eq!(
            caps.git.as_ref().unwrap().reason.as_deref(),
            Some(ids::REASON_RUNTIME_ERROR)
        );
    }

    #[test]
    fn add_artifact_only_updates_v2() {
        let mut v1 = ReportVariant::V1(sample_v1(SCHEMA_REPORT_V1));
        add_artifact(
            &mut v1,
            ArtifactPointer {
                artifact_type: depguard_types::ArtifactType::Comment,
                path: "comment.md".to_string(),
                format: Some("text/markdown".to_string()),
            },
        );
        assert!(matches!(v1, ReportVariant::V1(_)));

        let mut v2 = ReportVariant::V2(sample_v2(
            SCHEMA_REPORT_V2,
            VerdictStatus::Pass,
            SeverityV2::Info,
        ));
        add_artifact(
            &mut v2,
            ArtifactPointer {
                artifact_type: depguard_types::ArtifactType::Annotation,
                path: "annotations.txt".to_string(),
                format: None,
            },
        );
        let r = unwrap_v2(v2);
        assert_eq!(r.artifacts.as_ref().unwrap().len(), 1);
    }

    fn unwrap_v1(report: ReportVariant) -> DepguardReportV1 {
        match report {
            ReportVariant::V1(r) => r,
            _ => panic!("expected v1 report"),
        }
    }

    fn unwrap_v2(report: ReportVariant) -> DepguardReportV2 {
        match report {
            ReportVariant::V2(r) => r,
            _ => panic!("expected v2 report"),
        }
    }

    #[test]
    #[should_panic(expected = "expected v1 report")]
    fn unwrap_v1_panics_on_v2() {
        let report = empty_report(ReportVersion::V2, "repo", "strict");
        let _ = unwrap_v1(report);
    }

    #[test]
    #[should_panic(expected = "expected v2 report")]
    fn unwrap_v2_panics_on_v1() {
        let report = empty_report(ReportVersion::V1, "repo", "strict");
        let _ = unwrap_v2(report);
    }
}
