//! CLI entry point for depguard.
//!
//! This module is intentionally thin: it handles argument parsing, I/O, and exit codes.
//! All business logic lives in the `depguard-app` crate.

#![allow(unexpected_cfgs)]

use anyhow::Context;
use camino::Utf8PathBuf;
use clap::{Parser, Subcommand};
use depguard_app::{
    CheckInput, ExplainOutput, ReportVariant, ReportVersion, add_artifact, empty_report,
    parse_report_json, render_annotations, render_markdown, run_check, run_explain,
    runtime_error_report, serialize_report, to_renderable, verdict_exit_code,
};
use depguard_settings::Overrides;
use depguard_types::RepoPath;
use depguard_types::{ArtifactPointer, ArtifactType};
use std::process::Command;

#[cfg(test)]
fn terminate(code: i32) -> ! {
    panic!("process exit: {code}");
}

#[cfg(not(test))]
fn terminate(code: i32) -> ! {
    #[allow(unexpected_cfgs)]
    #[cfg(coverage)]
    {
        // Best effort: flush coverage data before ExitProcess on Windows.
        unsafe {
            unsafe extern "C" {
                fn __llvm_profile_write_file() -> i32;
            }
            let _ = __llvm_profile_write_file();
        }
    }
    std::process::exit(code)
}

/// Run mode for depguard check command.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, clap::ValueEnum)]
enum RunMode {
    /// Standard mode: exit 2 on policy failure, exit 1 on error.
    #[default]
    Standard,
    /// Cockpit mode: exit 0 if receipt written, regardless of verdict.
    Cockpit,
}

/// Options for the check command (reduces function argument count).
struct CheckOpts {
    base: Option<String>,
    head: Option<String>,
    report_out: Utf8PathBuf,
    report_version: String,
    write_markdown: bool,
    markdown_out: Utf8PathBuf,
    mode: RunMode,
}

#[derive(Parser, Debug)]
#[command(
    name = "depguard",
    version,
    about = "Dependency policy guard for Rust workspaces"
)]
struct Cli {
    /// Repository root (directory containing the root Cargo.toml).
    #[arg(long, default_value = ".")]
    repo_root: Utf8PathBuf,

    /// Path to depguard config TOML.
    #[arg(long, default_value = "depguard.toml")]
    config: Utf8PathBuf,

    /// Override profile (strict|warn|compat or custom).
    #[arg(long)]
    profile: Option<String>,

    /// Override scope (repo|diff).
    #[arg(long)]
    scope: Option<String>,

    /// Override maximum findings to emit.
    #[arg(long)]
    max_findings: Option<u32>,

    #[command(subcommand)]
    cmd: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Evaluate policy and write artifacts.
    Check {
        /// In diff scope: git base revision (e.g. origin/main).
        #[arg(long)]
        base: Option<String>,
        /// In diff scope: git head revision (e.g. HEAD).
        #[arg(long)]
        head: Option<String>,

        /// Where to write the JSON report.
        #[arg(long, default_value = "artifacts/depguard/report.json")]
        report_out: Utf8PathBuf,

        /// Report schema version to emit (v1, v2, or sensor-v1).
        #[arg(long, default_value = "v2")]
        report_version: String,

        /// Write a Markdown report alongside the JSON.
        #[arg(long)]
        write_markdown: bool,

        /// Where to write the Markdown report (if enabled).
        #[arg(long, default_value = "artifacts/depguard/comment.md")]
        markdown_out: Utf8PathBuf,

        /// Run mode: standard (exit 2 on fail) or cockpit (exit 0 if receipt written).
        #[arg(long, value_enum, default_value = "standard")]
        mode: RunMode,
    },

    /// Render markdown from an existing JSON report.
    Md {
        /// Path to the JSON report file.
        #[arg(long, default_value = "artifacts/depguard/report.json")]
        report: Utf8PathBuf,

        /// Where to write the Markdown output (if not specified, prints to stdout).
        #[arg(long, short)]
        output: Option<Utf8PathBuf>,
    },

    /// Render GitHub Actions annotations from an existing JSON report.
    Annotations {
        /// Path to the JSON report file.
        #[arg(long, default_value = "artifacts/depguard/report.json")]
        report: Utf8PathBuf,

        /// Maximum number of annotations to emit (default 10, per GHA best practices).
        #[arg(long, default_value = "10")]
        max: usize,
    },

    /// Explain a check_id or code with remediation guidance.
    Explain {
        /// The check_id (e.g., "deps.no_wildcards") or code (e.g., "wildcard_version") to explain.
        identifier: String,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.cmd {
        Commands::Check {
            ref base,
            ref head,
            ref report_out,
            ref report_version,
            write_markdown,
            ref markdown_out,
            mode,
        } => cmd_check(
            &cli,
            CheckOpts {
                base: base.clone(),
                head: head.clone(),
                report_out: report_out.clone(),
                report_version: report_version.clone(),
                write_markdown,
                markdown_out: markdown_out.clone(),
                mode,
            },
        ),
        Commands::Md { report, output } => cmd_md(report, output),
        Commands::Annotations { report, max } => cmd_annotations(report, max),
        Commands::Explain { identifier } => cmd_explain(&identifier),
    }
}

fn cmd_check(cli: &Cli, opts: CheckOpts) -> anyhow::Result<()> {
    let repo_root = cli
        .repo_root
        .canonicalize_utf8()
        .unwrap_or_else(|_| cli.repo_root.clone());

    let report_version = parse_report_version(&opts.report_version)?;

    let result = (|| -> anyhow::Result<i32> {
        if !repo_root.exists() {
            anyhow::bail!("repo root does not exist: {}", repo_root);
        }
        // Load config if present; missing file is allowed (defaults apply).
        let cfg_path = repo_root.join(&cli.config);
        let cfg_text = std::fs::read_to_string(&cfg_path).unwrap_or_default();

        let overrides = Overrides {
            profile: cli.profile.clone(),
            scope: cli.scope.clone(),
            max_findings: cli.max_findings,
        };

        // Fast path: missing root Cargo.toml -> emit empty report.
        let root_manifest = repo_root.join("Cargo.toml");
        if !root_manifest.exists() {
            let cfg = if cfg_text.trim().is_empty() {
                depguard_settings::DepguardConfigV1::default()
            } else {
                depguard_settings::parse_config_toml(&cfg_text).context("parse config")?
            };
            let resolved = depguard_settings::resolve_config(cfg, overrides.clone())
                .context("resolve config")?;
            let scope = match resolved.effective.scope {
                depguard_domain::policy::Scope::Repo => "repo",
                depguard_domain::policy::Scope::Diff => "diff",
            };
            let mut report = empty_report(report_version, scope, &resolved.effective.profile);
            if opts.write_markdown {
                let renderable = to_renderable(&report);
                let md = render_markdown(&renderable);
                write_text_file(&opts.markdown_out, &md).context("write markdown")?;
                add_artifact(
                    &mut report,
                    ArtifactPointer {
                        artifact_type: ArtifactType::Comment,
                        path: opts.markdown_out.to_string(),
                        format: Some("text/markdown".to_string()),
                    },
                );
            }
            write_report_file(&opts.report_out, &report).context("write report json")?;
            eprintln!(
                "depguard: no Cargo.toml found at {}; emitting empty report",
                root_manifest
            );
            return Ok(0);
        }

        // For diff scope, we need to get changed files via git.
        let changed_files = if cli.scope.as_deref() == Some("diff")
            || (cli.scope.is_none() && scope_from_config(&cfg_text) == Some("diff"))
        {
            let base = opts.base.as_ref().context("diff scope requires --base")?;
            let head = opts.head.as_ref().context("diff scope requires --head")?;
            Some(git_changed_files(&repo_root, base, head).context("git diff --name-only failed")?)
        } else {
            None
        };

        let input = CheckInput {
            repo_root: &repo_root,
            config_text: &cfg_text,
            overrides,
            changed_files,
            report_version,
        };

        let mut output = run_check(input)?;

        if opts.write_markdown {
            let renderable = to_renderable(&output.report);
            let md = render_markdown(&renderable);
            write_text_file(&opts.markdown_out, &md).context("write markdown")?;
            add_artifact(
                &mut output.report,
                ArtifactPointer {
                    artifact_type: ArtifactType::Comment,
                    path: opts.markdown_out.to_string(),
                    format: Some("text/markdown".to_string()),
                },
            );
        }

        write_report_file(&opts.report_out, &output.report).context("write report json")?;

        Ok(report_exit_code(&output.report))
    })();

    match result {
        Ok(code) => {
            // In cockpit mode, always exit 0 if receipt was written successfully.
            let final_code = match opts.mode {
                RunMode::Cockpit => 0,
                RunMode::Standard => code,
            };
            if final_code != 0 {
                terminate(final_code);
            }
            Ok(())
        }
        Err(err) => {
            let report = runtime_error_report(report_version, &format!("{err:#}"));
            let receipt_written = write_report_file(&opts.report_out, &report).is_ok();
            eprintln!("depguard error: {err:#}");

            // In cockpit mode, exit 0 if we successfully wrote an error receipt.
            match (opts.mode, receipt_written) {
                (RunMode::Cockpit, true) => Ok(()),
                _ => terminate(1),
            }
        }
    }
}

/// Quick parse of config to check scope (avoids full resolution just to check diff scope).
fn scope_from_config(cfg_text: &str) -> Option<&str> {
    for line in cfg_text.lines() {
        let line = line.trim();
        if line.starts_with("scope") {
            if line.contains("\"diff\"") || line.contains("'diff'") {
                return Some("diff");
            }
            if line.contains("\"repo\"") || line.contains("'repo'") {
                return Some("repo");
            }
        }
    }
    None
}

/// Error type for git diff operations, providing specific remediation guidance.
#[derive(Debug)]
enum GitDiffError {
    /// Git executable not found or failed to spawn.
    SpawnFailed(std::io::Error),
    /// The base commit is not reachable (common in shallow clones).
    BaseCommitNotReachable { base: String, stderr: String },
    /// The head commit is not reachable.
    HeadCommitNotReachable { head: String, stderr: String },
    /// Generic git error.
    Other { stderr: String },
}

impl std::fmt::Display for GitDiffError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GitDiffError::SpawnFailed(e) => {
                write!(f, "failed to run git: {e}")
            }
            GitDiffError::BaseCommitNotReachable { base, stderr } => {
                write!(
                    f,
                    "git base revision '{base}' is not reachable.\n\n\
                    This commonly happens in CI environments with shallow clones.\n\n\
                    Remediation options:\n\
                    1. Fetch more history: git fetch --deepen=100\n\
                    2. Fetch the full history: git fetch --unshallow\n\
                    3. Fetch the specific base ref: git fetch origin {base}\n\
                    4. Use --scope repo instead of --scope diff\n\n\
                    Git error: {stderr}"
                )
            }
            GitDiffError::HeadCommitNotReachable { head, stderr } => {
                write!(
                    f,
                    "git head revision '{head}' is not reachable.\n\n\
                    Remediation: ensure the head ref exists locally.\n\n\
                    Git error: {stderr}"
                )
            }
            GitDiffError::Other { stderr } => {
                write!(f, "git diff failed: {stderr}")
            }
        }
    }
}

impl std::error::Error for GitDiffError {}

fn classify_git_diff_error(base: &str, head: &str, stderr: String) -> GitDiffError {
    let stderr_lower = stderr.to_lowercase();

    // Detect shallow clone / missing base commit errors.
    // Git produces various error messages depending on the situation:
    // - "fatal: ambiguous argument 'origin/main': unknown revision or path"
    // - "fatal: bad revision 'origin/main..HEAD'"
    // - "fatal: Invalid revision range"
    let is_base_unreachable = stderr_lower.contains("unknown revision")
        || stderr_lower.contains("bad revision")
        || stderr_lower.contains("invalid revision range")
        || (stderr_lower.contains("fatal:") && stderr_lower.contains(base.to_lowercase().as_str()));

    // Check if the error specifically mentions the head ref.
    let is_head_unreachable = !is_base_unreachable
        && stderr_lower.contains("fatal:")
        && stderr_lower.contains(head.to_lowercase().as_str());

    if is_base_unreachable {
        GitDiffError::BaseCommitNotReachable {
            base: base.to_string(),
            stderr,
        }
    } else if is_head_unreachable {
        GitDiffError::HeadCommitNotReachable {
            head: head.to_string(),
            stderr,
        }
    } else {
        GitDiffError::Other { stderr }
    }
}

fn git_changed_files(
    repo_root: &camino::Utf8Path,
    base: &str,
    head: &str,
) -> anyhow::Result<Vec<RepoPath>> {
    let output = Command::new("git")
        .current_dir(repo_root)
        .args(["diff", "--name-only", &format!("{base}..{head}")])
        .output()
        .map_err(GitDiffError::SpawnFailed)?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(classify_git_diff_error(base, head, stderr).into());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let paths = stdout
        .lines()
        .map(|l| RepoPath::new(l.trim()))
        .collect::<Vec<_>>();

    Ok(paths)
}

fn parse_report_version(v: &str) -> anyhow::Result<ReportVersion> {
    match v {
        "v1" | "1" | "depguard.report.v1" => Ok(ReportVersion::V1),
        "v2" | "2" | "depguard.report.v2" => Ok(ReportVersion::V2),
        "sensor-v1" | "sensor.report.v1" => Ok(ReportVersion::SensorV1),
        other => anyhow::bail!("unknown report version: {other} (expected v1, v2, or sensor-v1)"),
    }
}

fn report_exit_code(report: &ReportVariant) -> i32 {
    match report {
        ReportVariant::V1(r) => verdict_exit_code(r.verdict.clone()),
        ReportVariant::V2(r) => match r.verdict.status {
            depguard_types::VerdictStatus::Pass => 0,
            depguard_types::VerdictStatus::Warn => 0,
            depguard_types::VerdictStatus::Fail => 2,
            depguard_types::VerdictStatus::Skip => 0,
        },
    }
}

fn write_report_file(path: &camino::Utf8Path, report: &ReportVariant) -> anyhow::Result<()> {
    if let Some(parent) = path.parent().filter(|p| !p.as_str().is_empty()) {
        std::fs::create_dir_all(parent).with_context(|| format!("create directory: {}", parent))?;
    }
    let data = serialize_report(report).context("serialize report")?;
    std::fs::write(path, data).with_context(|| format!("write report: {}", path))?;
    Ok(())
}

fn write_text_file(path: &camino::Utf8Path, text: &str) -> anyhow::Result<()> {
    if let Some(parent) = path.parent().filter(|p| !p.as_str().is_empty()) {
        std::fs::create_dir_all(parent).with_context(|| format!("create directory: {}", parent))?;
    }
    std::fs::write(path, text).with_context(|| format!("write text: {}", path))?;
    Ok(())
}

fn cmd_md(report_path: Utf8PathBuf, output: Option<Utf8PathBuf>) -> anyhow::Result<()> {
    let report_text = std::fs::read_to_string(&report_path)
        .with_context(|| format!("read report: {}", report_path))?;
    let report = parse_report_json(&report_text)?;
    let renderable = to_renderable(&report);
    let md = render_markdown(&renderable);

    if let Some(out_path) = output {
        write_text_file(&out_path, &md).context("write markdown output")?;
    } else {
        print!("{}", md);
    }

    Ok(())
}

fn cmd_annotations(report_path: Utf8PathBuf, max: usize) -> anyhow::Result<()> {
    let report_text = std::fs::read_to_string(&report_path)
        .with_context(|| format!("read report: {}", report_path))?;
    let report = parse_report_json(&report_text)?;
    let renderable = to_renderable(&report);
    let annotations = render_annotations(&renderable, max);

    for annotation in annotations {
        println!("{}", annotation);
    }

    Ok(())
}

fn cmd_explain(identifier: &str) -> anyhow::Result<()> {
    match run_explain(identifier) {
        ExplainOutput::Found(exp) => {
            print!("{}", depguard_app::format_explanation(&exp));
            Ok(())
        }
        ExplainOutput::NotFound {
            identifier,
            available_check_ids,
            available_codes,
        } => {
            eprint!(
                "{}",
                depguard_app::format_not_found(&identifier, available_check_ids, available_codes)
            );
            terminate(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::any::Any;
    use std::panic::{catch_unwind, AssertUnwindSafe};
    use tempfile::TempDir;

    #[test]
    fn git_diff_error_display_base_unreachable() {
        let err = GitDiffError::BaseCommitNotReachable {
            base: "origin/main".to_string(),
            stderr: "fatal: ambiguous argument 'origin/main': unknown revision".to_string(),
        };
        let msg = format!("{err}");

        assert!(msg.contains("origin/main"));
        assert!(msg.contains("not reachable"));
        assert!(msg.contains("shallow clone"));
        assert!(msg.contains("git fetch --deepen=100"));
        assert!(msg.contains("git fetch --unshallow"));
        assert!(msg.contains("--scope repo"));
    }

    #[test]
    fn git_diff_error_display_head_unreachable() {
        let err = GitDiffError::HeadCommitNotReachable {
            head: "feature-branch".to_string(),
            stderr: "fatal: bad revision 'feature-branch'".to_string(),
        };
        let msg = format!("{err}");

        assert!(msg.contains("feature-branch"));
        assert!(msg.contains("not reachable"));
        assert!(msg.contains("ensure the head ref exists"));
    }

    #[test]
    fn git_diff_error_display_other() {
        let err = GitDiffError::Other {
            stderr: "fatal: Not a git repository".to_string(),
        };
        let msg = format!("{err}");

        assert!(msg.contains("git diff failed"));
        assert!(msg.contains("Not a git repository"));
    }

    #[test]
    fn git_diff_error_display_spawn_failed() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "git not found");
        let err = GitDiffError::SpawnFailed(io_err);
        let msg = format!("{err}");

        assert!(msg.contains("failed to run git"));
        assert!(msg.contains("git not found"));
    }

    #[test]
    fn scope_from_config_detects_diff_and_repo() {
        let cfg = r#"
            # comment
            scope = "diff"
        "#;
        assert_eq!(scope_from_config(cfg), Some("diff"));

        let cfg = "scope = 'repo'";
        assert_eq!(scope_from_config(cfg), Some("repo"));
    }

    #[test]
    fn scope_from_config_returns_none_for_unknown() {
        let cfg = r#"
            scope = "other"
        "#;
        assert_eq!(scope_from_config(cfg), None);
    }

    #[test]
    fn parse_report_version_accepts_aliases() {
        assert!(matches!(
            parse_report_version("v1").unwrap(),
            ReportVersion::V1
        ));
        assert!(matches!(
            parse_report_version("1").unwrap(),
            ReportVersion::V1
        ));
        assert!(matches!(
            parse_report_version("depguard.report.v1").unwrap(),
            ReportVersion::V1
        ));

        assert!(matches!(
            parse_report_version("v2").unwrap(),
            ReportVersion::V2
        ));
        assert!(matches!(
            parse_report_version("2").unwrap(),
            ReportVersion::V2
        ));
        assert!(matches!(
            parse_report_version("depguard.report.v2").unwrap(),
            ReportVersion::V2
        ));

        assert!(matches!(
            parse_report_version("sensor-v1").unwrap(),
            ReportVersion::SensorV1
        ));
        assert!(matches!(
            parse_report_version("sensor.report.v1").unwrap(),
            ReportVersion::SensorV1
        ));
    }

    #[test]
    fn parse_report_version_rejects_unknown() {
        let err = parse_report_version("nope").unwrap_err();
        assert!(err.to_string().contains("unknown report version"));
    }

    #[test]
    fn report_exit_code_maps_v1_and_v2_verdicts() {
        let mut v1 = empty_report(ReportVersion::V1, "repo", "strict");
        if let ReportVariant::V1(ref mut r) = v1 {
            r.verdict = depguard_types::Verdict::Fail;
        }
        assert_eq!(report_exit_code(&v1), 2);

        let mut v2 = empty_report(ReportVersion::V2, "repo", "strict");
        if let ReportVariant::V2(ref mut r) = v2 {
            r.verdict.status = depguard_types::VerdictStatus::Pass;
        }
        assert_eq!(report_exit_code(&v2), 0);

        if let ReportVariant::V2(ref mut r) = v2 {
            r.verdict.status = depguard_types::VerdictStatus::Warn;
        }
        assert_eq!(report_exit_code(&v2), 0);

        if let ReportVariant::V2(ref mut r) = v2 {
            r.verdict.status = depguard_types::VerdictStatus::Skip;
        }
        assert_eq!(report_exit_code(&v2), 0);

        if let ReportVariant::V2(ref mut r) = v2 {
            r.verdict.status = depguard_types::VerdictStatus::Fail;
        }
        assert_eq!(report_exit_code(&v2), 2);
    }

    #[test]
    fn classify_git_diff_error_variants() {
        let err = classify_git_diff_error(
            "origin/main",
            "HEAD",
            "fatal: ambiguous argument 'origin/main': unknown revision or path".to_string(),
        );
        assert!(matches!(err, GitDiffError::BaseCommitNotReachable { .. }));

        let err = classify_git_diff_error(
            "origin/main",
            "feature-branch",
            "fatal: bad object feature-branch".to_string(),
        );
        assert!(matches!(err, GitDiffError::HeadCommitNotReachable { .. }));

        let err = classify_git_diff_error(
            "origin/main",
            "HEAD",
            "fatal: not a git repository".to_string(),
        );
        assert!(matches!(err, GitDiffError::Other { .. }));
    }

    fn write_manifest(root: &Utf8PathBuf, deps: &str) {
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

    fn cli_for_root(root: &Utf8PathBuf) -> Cli {
        Cli {
            repo_root: root.clone(),
            config: Utf8PathBuf::from("depguard.toml"),
            profile: None,
            scope: None,
            max_findings: None,
            cmd: Commands::Check {
                base: None,
                head: None,
                report_out: Utf8PathBuf::from("report.json"),
                report_version: "v2".to_string(),
                write_markdown: false,
                markdown_out: Utf8PathBuf::from("comment.md"),
                mode: RunMode::Standard,
            },
        }
    }

    fn panic_payload_to_string(err: &(dyn Any + Send)) -> String {
        if let Some(s) = err.downcast_ref::<String>() {
            s.clone()
        } else if let Some(s) = err.downcast_ref::<&str>() {
            s.to_string()
        } else {
            String::new()
        }
    }

    fn assert_exit_code(expected: i32, f: impl FnOnce()) {
        let err = catch_unwind(AssertUnwindSafe(f)).expect_err("expected exit");
        let msg = panic_payload_to_string(err.as_ref());
        assert!(msg.contains(&format!("process exit: {expected}")));
    }

    #[test]
    fn cmd_check_empty_repo_writes_empty_report_and_markdown() {
        let tmp = TempDir::new().expect("temp dir");
        let root = Utf8PathBuf::from_path_buf(tmp.path().to_path_buf()).expect("utf8 path");

        std::fs::write(root.join("depguard.toml"), "scope = \"diff\"")
            .expect("write config");

        let cli = cli_for_root(&root);
        let report_out = root.join("out").join("report.json");
        let markdown_out = root.join("out").join("comment.md");
        let opts = CheckOpts {
            base: None,
            head: None,
            report_out: report_out.clone(),
            report_version: "v2".to_string(),
            write_markdown: true,
            markdown_out: markdown_out.clone(),
            mode: RunMode::Standard,
        };

        cmd_check(&cli, opts).expect("cmd_check");
        assert!(report_out.exists());
        assert!(markdown_out.exists());
    }

    #[test]
    fn cmd_check_with_markdown_on_valid_repo() {
        let tmp = TempDir::new().expect("temp dir");
        let root = Utf8PathBuf::from_path_buf(tmp.path().to_path_buf()).expect("utf8 path");
        write_manifest(&root, "");

        let cli = cli_for_root(&root);
        let report_out = root.join("artifacts").join("report.json");
        let markdown_out = root.join("artifacts").join("comment.md");
        let opts = CheckOpts {
            base: None,
            head: None,
            report_out: report_out.clone(),
            report_version: "v2".to_string(),
            write_markdown: true,
            markdown_out: markdown_out.clone(),
            mode: RunMode::Standard,
        };

        cmd_check(&cli, opts).expect("cmd_check");
        assert!(report_out.exists());
        assert!(markdown_out.exists());
    }

    #[test]
    fn cmd_check_cockpit_mode_suppresses_exit_on_fail() {
        let tmp = TempDir::new().expect("temp dir");
        let root = Utf8PathBuf::from_path_buf(tmp.path().to_path_buf()).expect("utf8 path");
        write_manifest(&root, r#"serde = "*""#);

        let cli = cli_for_root(&root);
        let report_out = root.join("report.json");
        let opts = CheckOpts {
            base: None,
            head: None,
            report_out,
            report_version: "v2".to_string(),
            write_markdown: false,
            markdown_out: root.join("comment.md"),
            mode: RunMode::Cockpit,
        };

        cmd_check(&cli, opts).expect("cmd_check");
    }

    #[test]
    fn cmd_check_standard_mode_exits_on_fail() {
        let tmp = TempDir::new().expect("temp dir");
        let root = Utf8PathBuf::from_path_buf(tmp.path().to_path_buf()).expect("utf8 path");
        write_manifest(&root, r#"serde = "*""#);

        let cli = cli_for_root(&root);
        let report_out = root.join("report.json");
        let opts = CheckOpts {
            base: None,
            head: None,
            report_out: report_out.clone(),
            report_version: "v2".to_string(),
            write_markdown: false,
            markdown_out: root.join("comment.md"),
            mode: RunMode::Standard,
        };

        assert_exit_code(2, || {
            let _ = cmd_check(&cli, opts);
        });
        assert!(report_out.exists());
    }

    #[test]
    fn cmd_check_error_path_writes_runtime_report_in_cockpit() {
        let tmp = TempDir::new().expect("temp dir");
        let root = Utf8PathBuf::from_path_buf(tmp.path().to_path_buf()).expect("utf8 path");
        let missing_root = root.join("missing");

        let cli = cli_for_root(&missing_root);
        let report_out = root.join("report.json");
        let opts = CheckOpts {
            base: None,
            head: None,
            report_out: report_out.clone(),
            report_version: "v2".to_string(),
            write_markdown: false,
            markdown_out: root.join("comment.md"),
            mode: RunMode::Cockpit,
        };

        cmd_check(&cli, opts).expect("cmd_check");
        assert!(report_out.exists());
    }

    #[test]
    fn cmd_md_writes_output_file() {
        let tmp = TempDir::new().expect("temp dir");
        let root = Utf8PathBuf::from_path_buf(tmp.path().to_path_buf()).expect("utf8 path");

        let report = empty_report(ReportVersion::V2, "repo", "strict");
        let data = serialize_report(&report).expect("serialize report");
        let report_path = root.join("report.json");
        std::fs::write(&report_path, data).expect("write report");

        let output_path = root.join("report.md");
        cmd_md(report_path, Some(output_path.clone())).expect("cmd_md");
        assert!(output_path.exists());
    }

    #[test]
    fn cmd_explain_not_found_exits() {
        assert_exit_code(1, || {
            let _ = cmd_explain("not-a-real-id");
        });
    }

    #[test]
    fn cmd_check_error_exits_in_standard_mode() {
        let tmp = TempDir::new().expect("temp dir");
        let root = Utf8PathBuf::from_path_buf(tmp.path().join("missing")).expect("utf8 path");

        let cli = Cli {
            repo_root: root.clone(),
            config: Utf8PathBuf::from("depguard.toml"),
            profile: None,
            scope: None,
            max_findings: None,
            cmd: Commands::Check {
                base: None,
                head: None,
                report_out: Utf8PathBuf::from("report.json"),
                report_version: "v2".to_string(),
                write_markdown: false,
                markdown_out: Utf8PathBuf::from("comment.md"),
                mode: RunMode::Standard,
            },
        };

        let report_out = Utf8PathBuf::from_path_buf(tmp.path().join("out").join("report.json"))
            .expect("utf8 report path");
        let opts = CheckOpts {
            base: None,
            head: None,
            report_out: report_out.clone(),
            report_version: "v2".to_string(),
            write_markdown: false,
            markdown_out: Utf8PathBuf::from("comment.md"),
            mode: RunMode::Standard,
        };

        assert_exit_code(1, || {
            let _ = cmd_check(&cli, opts);
        });
        assert!(report_out.exists());
    }

    #[test]
    fn write_report_and_text_files_no_parent() {
        let tmp = TempDir::new().expect("temp dir");
        let cwd = std::env::current_dir().expect("cwd");
        std::env::set_current_dir(tmp.path()).expect("set cwd");

        let report = empty_report(ReportVersion::V2, "repo", "strict");
        let report_path = Utf8PathBuf::from("report.json");
        write_report_file(&report_path, &report).expect("write report");
        assert!(report_path.exists());

        let text_path = Utf8PathBuf::from("note.txt");
        write_text_file(&text_path, "hello").expect("write text");
        assert!(text_path.exists());

        std::env::set_current_dir(cwd).expect("restore cwd");
    }

    #[test]
    fn assert_exit_code_accepts_str_payload() {
        assert_exit_code(2, || {
            panic!("process exit: 2");
        });
    }

    #[test]
    fn panic_payload_to_string_handles_unknown() {
        let payload: Box<dyn Any + Send> = Box::new(42_i32);
        let msg = panic_payload_to_string(payload.as_ref());
        assert!(msg.is_empty());
    }

    #[test]
    fn git_changed_files_errors_on_non_repo() {
        let tmp = TempDir::new().expect("temp dir");
        let root = Utf8PathBuf::from_path_buf(tmp.path().to_path_buf()).expect("utf8 path");
        let err = git_changed_files(&root, "HEAD", "HEAD~1").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("git diff") || msg.contains("failed to run git"));
    }

    #[test]
    fn write_report_and_text_files_create_parent() {
        let tmp = TempDir::new().expect("temp dir");
        let root = Utf8PathBuf::from_path_buf(tmp.path().to_path_buf()).expect("utf8 path");

        let report = empty_report(ReportVersion::V2, "repo", "strict");
        let report_path = root.join("nested").join("report.json");
        write_report_file(&report_path, &report).expect("write report");
        assert!(report_path.exists());

        let text_path = root.join("nested").join("note.txt");
        write_text_file(&text_path, "hello").expect("write text");
        assert!(text_path.exists());
    }
}
