//! The `check` use case: evaluate policy and produce a report.

use anyhow::Context;
use camino::Utf8Path;
use depguard_domain::policy::Scope as DomainScope;
use depguard_repo::ScopeInput;
use depguard_settings::{Overrides, ResolvedConfig};
use depguard_types::{DepguardReport, ReportEnvelope, ToolMeta, Verdict};
use time::OffsetDateTime;

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
}

/// Output from the check use case.
#[derive(Clone, Debug)]
pub struct CheckOutput {
    /// The generated report.
    pub report: DepguardReport,
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

    let finished_at = OffsetDateTime::now_utc();

    let report: DepguardReport = ReportEnvelope {
        schema: "receipt.envelope.v1".to_string(),
        tool: ToolMeta {
            name: "depguard".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        },
        started_at,
        finished_at,
        verdict: domain_report.verdict,
        findings: domain_report.findings,
        data: domain_report.data,
    };

    Ok(CheckOutput {
        report,
        resolved_config: resolved,
    })
}

/// Map verdict to exit code: 0 = pass, 1 = warn, 2 = fail.
pub fn verdict_exit_code(verdict: Verdict) -> i32 {
    match verdict {
        Verdict::Pass => 0,
        Verdict::Warn => 1,
        Verdict::Fail => 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
edition = "2021"
"#,
        )
        .expect("write Cargo.toml");

        let input = CheckInput {
            repo_root: root,
            config_text: "",
            overrides: Overrides::default(),
            changed_files: None,
        };

        let output = run_check(input).expect("run_check");
        assert_eq!(output.resolved_config.effective.profile, "strict");
    }

    #[test]
    fn verdict_exit_codes() {
        assert_eq!(verdict_exit_code(Verdict::Pass), 0);
        assert_eq!(verdict_exit_code(Verdict::Warn), 1);
        assert_eq!(verdict_exit_code(Verdict::Fail), 2);
    }
}
