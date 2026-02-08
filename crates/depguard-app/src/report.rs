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
