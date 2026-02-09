//! The `check` use case: evaluate policy and produce a report.

use anyhow::Context;
use camino::Utf8Path;
use depguard_domain::policy::Scope as DomainScope;
use depguard_repo::ScopeInput;
use depguard_settings::{Overrides, ResolvedConfig};
use depguard_types::{
    Capabilities, CapabilityAvailability, CapabilityStatus, ReportEnvelope, ReportEnvelopeV2,
    RunMeta, SCHEMA_REPORT_V1, SCHEMA_REPORT_V2, SCHEMA_SENSOR_REPORT_V1, ToolMeta, ToolMetaV2,
    Verdict, VerdictCounts, VerdictStatus, VerdictV2, ids,
};
use time::OffsetDateTime;

use crate::report::{ReportVariant, ReportVersion};

/// Input for the check use case.
#[derive(Clone, Debug)]
pub struct CheckInput<'a> {
    /// Repository root path.
    pub repo_root: &'a Utf8Path,
    /// Config file contents (empty string if not found).
    pub config_text: &'a str,
    /// CLI overrides.
    pub overrides: Overrides,
    /// For diff scope: list of changed files (relative to repo root).
    pub changed_files: Option<Vec<depguard_types::RepoPath>>,
    /// Report schema version to emit.
    pub report_version: ReportVersion,
}

/// Output from the check use case.
#[derive(Clone, Debug)]
pub struct CheckOutput {
    /// The generated report.
    pub report: ReportVariant,
    /// The resolved configuration used.
    pub resolved_config: ResolvedConfig,
}

/// Run the check use case: parse config, discover workspace, evaluate policy, produce report.
pub fn run_check(input: CheckInput<'_>) -> anyhow::Result<CheckOutput> {
    let started_at = OffsetDateTime::now_utc();

    // Parse config (empty is allowed, defaults apply).
    let cfg = if input.config_text.trim().is_empty() {
        depguard_settings::DepguardConfigV1::default()
    } else {
        depguard_settings::parse_config_toml(input.config_text).context("parse config")?
    };

    let resolved = depguard_settings::resolve_config(cfg, input.overrides.clone())
        .context("resolve config")?;

    let scope_input = match resolved.effective.scope {
        DomainScope::Repo => ScopeInput::Repo,
        DomainScope::Diff => {
            let changed_files = input
                .changed_files
                .clone()
                .context("diff scope requires changed_files")?;
            ScopeInput::Diff { changed_files }
        }
    };

    let model = depguard_repo::build_workspace_model(input.repo_root, scope_input)
        .context("build workspace model")?;

    let domain_report = depguard_domain::evaluate(&model, &resolved.effective);
    let depguard_domain::report::DomainReport {
        verdict: domain_verdict,
        findings: domain_findings,
        data: domain_data,
        counts: domain_counts,
    } = domain_report;

    let finished_at = OffsetDateTime::now_utc();
    let duration_ms = (finished_at - started_at).whole_milliseconds().max(0) as u64;

    let report = match input.report_version {
        ReportVersion::V1 => ReportVariant::V1(ReportEnvelope {
            schema: SCHEMA_REPORT_V1.to_string(),
            tool: ToolMeta {
                name: "depguard".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            started_at,
            finished_at,
            verdict: domain_verdict,
            findings: domain_findings,
            data: domain_data,
        }),
        ReportVersion::V2 | ReportVersion::SensorV1 => {
            let schema = match input.report_version {
                ReportVersion::SensorV1 => SCHEMA_SENSOR_REPORT_V1.to_string(),
                _ => SCHEMA_REPORT_V2.to_string(),
            };

            // Build capabilities block for SensorV1 (No Green By Omission).
            let capabilities = if input.report_version == ReportVersion::SensorV1 {
                Some(Capabilities {
                    git: Some(CapabilityStatus {
                        status: if input.changed_files.is_some() {
                            CapabilityAvailability::Available
                        } else {
                            CapabilityAvailability::Missing
                        },
                        reason: if input.changed_files.is_none() {
                            Some(ids::REASON_DIFF_SCOPE_DISABLED.to_string())
                        } else {
                            None
                        },
                    }),
                    config: Some(CapabilityStatus {
                        status: if !input.config_text.is_empty() {
                            CapabilityAvailability::Available
                        } else {
                            CapabilityAvailability::Missing
                        },
                        reason: if input.config_text.is_empty() {
                            Some(ids::REASON_CONFIG_MISSING_DEFAULTED.to_string())
                        } else {
                            None
                        },
                    }),
                })
            } else {
                None
            };

            let verdict = VerdictV2 {
                status: match domain_verdict {
                    Verdict::Pass => VerdictStatus::Pass,
                    Verdict::Warn => VerdictStatus::Warn,
                    Verdict::Fail => VerdictStatus::Fail,
                },
                counts: VerdictCounts {
                    info: domain_counts.info,
                    warn: domain_counts.warning,
                    error: domain_counts.error,
                    suppressed: 0,
                },
                reasons: Vec::new(),
            };

            let run = RunMeta {
                started_at,
                ended_at: Some(finished_at),
                duration_ms: Some(duration_ms),
                host: None,
                ci: None,
                git: None,
                capabilities,
            };

            // Convert v1 findings to v2 findings (severity naming change).
            let findings = domain_findings
                .into_iter()
                .map(|f| depguard_types::FindingV2 {
                    severity: match f.severity {
                        depguard_types::Severity::Info => depguard_types::SeverityV2::Info,
                        depguard_types::Severity::Warning => depguard_types::SeverityV2::Warn,
                        depguard_types::Severity::Error => depguard_types::SeverityV2::Error,
                    },
                    check_id: f.check_id,
                    code: f.code,
                    message: f.message,
                    location: f.location,
                    help: f.help,
                    url: f.url,
                    fingerprint: f.fingerprint,
                    data: f.data,
                })
                .collect();

            ReportVariant::V2(ReportEnvelopeV2 {
                schema,
                tool: ToolMetaV2 {
                    name: "depguard".to_string(),
                    version: env!("CARGO_PKG_VERSION").to_string(),
                    commit: None,
                },
                run,
                verdict,
                findings,
                artifacts: None,
                data: domain_data,
            })
        }
    };

    Ok(CheckOutput {
        report,
        resolved_config: resolved,
    })
}

/// Map verdict to exit code: 0 = pass/warn, 2 = fail.
pub fn verdict_exit_code(verdict: Verdict) -> i32 {
    match verdict {
        Verdict::Pass => 0,
        Verdict::Warn => 0,
        Verdict::Fail => 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use depguard_types::{
        CapabilityAvailability, SCHEMA_REPORT_V2, SCHEMA_SENSOR_REPORT_V1, SeverityV2, ids,
    };

    #[test]
    fn empty_config_uses_defaults() {
        let tmp = tempfile::tempdir().expect("create temp dir");
        let root = camino::Utf8Path::from_path(tmp.path()).expect("utf8 path");

        // Create a minimal Cargo.toml
        std::fs::write(
            root.join("Cargo.toml"),
            r#"
[package]
name = "test"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
"#,
        )
        .expect("write Cargo.toml");

        let input = CheckInput {
            repo_root: root,
            config_text: "",
            overrides: Overrides::default(),
            changed_files: None,
            report_version: ReportVersion::V1,
        };

        let output = run_check(input).expect("run_check");
        assert_eq!(output.resolved_config.effective.profile, "strict");
    }

    #[test]
    fn verdict_exit_codes() {
        assert_eq!(verdict_exit_code(Verdict::Pass), 0);
        assert_eq!(verdict_exit_code(Verdict::Warn), 0);
        assert_eq!(verdict_exit_code(Verdict::Fail), 2);
    }

    fn write_manifest(root: &Utf8Path, deps: &str) {
        let deps_block = if deps.trim().is_empty() {
            String::new()
        } else {
            format!("\n[dependencies]\n{deps}\n")
        };
        let content = format!(
            r#"[package]
name = "test"
version = "0.1.0"
edition = "2021"
{deps_block}"#
        );
        std::fs::write(root.join("Cargo.toml"), content).expect("write Cargo.toml");
    }

    #[test]
    fn diff_scope_requires_changed_files() {
        let tmp = tempfile::tempdir().expect("create temp dir");
        let root = camino::Utf8Path::from_path(tmp.path()).expect("utf8 path");
        write_manifest(root, "");

        let input = CheckInput {
            repo_root: root,
            config_text: r#"scope = "diff""#,
            overrides: Overrides::default(),
            changed_files: None,
            report_version: ReportVersion::V1,
        };

        let err = run_check(input).expect_err("expected diff scope error");
        assert!(
            err.to_string()
                .contains("diff scope requires changed_files")
        );
    }

    #[test]
    fn sensor_v1_capabilities_mark_missing() {
        let tmp = tempfile::tempdir().expect("create temp dir");
        let root = camino::Utf8Path::from_path(tmp.path()).expect("utf8 path");
        write_manifest(root, "");

        let input = CheckInput {
            repo_root: root,
            config_text: "",
            overrides: Overrides::default(),
            changed_files: None,
            report_version: ReportVersion::SensorV1,
        };

        let output = run_check(input).expect("run_check");
        match output.report {
            ReportVariant::V2(report) => {
                assert_eq!(report.schema, SCHEMA_SENSOR_REPORT_V1);
                let caps = report.run.capabilities.as_ref().expect("capabilities");
                let git = caps.git.as_ref().expect("git capability");
                assert_eq!(git.status, CapabilityAvailability::Missing);
                assert_eq!(git.reason.as_deref(), Some(ids::REASON_DIFF_SCOPE_DISABLED));
                let config = caps.config.as_ref().expect("config capability");
                assert_eq!(config.status, CapabilityAvailability::Missing);
                assert_eq!(
                    config.reason.as_deref(),
                    Some(ids::REASON_CONFIG_MISSING_DEFAULTED)
                );
            }
            _ => panic!("expected v2 report variant"),
        }
    }

    #[test]
    fn v2_report_converts_findings_and_severity() {
        let tmp = tempfile::tempdir().expect("create temp dir");
        let root = camino::Utf8Path::from_path(tmp.path()).expect("utf8 path");
        write_manifest(root, r#"serde = "*""#);

        let input = CheckInput {
            repo_root: root,
            config_text: "",
            overrides: Overrides::default(),
            changed_files: None,
            report_version: ReportVersion::V2,
        };

        let output = run_check(input).expect("run_check");
        match output.report {
            ReportVariant::V2(report) => {
                assert_eq!(report.schema, SCHEMA_REPORT_V2);
                let finding = report
                    .findings
                    .iter()
                    .find(|f| f.check_id == ids::CHECK_DEPS_NO_WILDCARDS)
                    .expect("wildcard finding");
                assert_eq!(finding.severity, SeverityV2::Error);
            }
            _ => panic!("expected v2 report variant"),
        }
    }
}
