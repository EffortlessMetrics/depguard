//! CLI entry point for depguard.
//!
//! This module is intentionally thin: it handles argument parsing, I/O, and exit codes.
//! All business logic lives in the `depguard-app` crate.

use anyhow::Context;
use camino::Utf8PathBuf;
use clap::{Parser, Subcommand};
use depguard_app::{
    CheckInput, ExplainOutput, ReportVariant, ReportVersion, add_artifact, empty_report,
    parse_report_json, render_annotations, render_markdown, run_check, run_explain,
    runtime_error_report, serialize_report, to_renderable, verdict_exit_code,
};
use depguard_types::{ArtifactPointer, ArtifactType};
use depguard_settings::Overrides;
use depguard_types::RepoPath;
use std::process::Command;

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
                std::process::exit(final_code);
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
                _ => std::process::exit(1),
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
        let stderr_lower = stderr.to_lowercase();

        // Detect shallow clone / missing base commit errors.
        // Git produces various error messages depending on the situation:
        // - "fatal: ambiguous argument 'origin/main': unknown revision or path"
        // - "fatal: bad revision 'origin/main..HEAD'"
        // - "fatal: Invalid revision range"
        let is_base_unreachable = stderr_lower.contains("unknown revision")
            || stderr_lower.contains("bad revision")
            || stderr_lower.contains("invalid revision range")
            || (stderr_lower.contains("fatal:")
                && stderr_lower.contains(base.to_lowercase().as_str()));

        // Check if the error specifically mentions the head ref.
        let is_head_unreachable = !is_base_unreachable
            && stderr_lower.contains("fatal:")
            && stderr_lower.contains(head.to_lowercase().as_str());

        let err = if is_base_unreachable {
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
        };

        return Err(err.into());
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
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("create directory: {}", parent))?;
    }
    let data = serialize_report(report).context("serialize report")?;
    std::fs::write(path, data).with_context(|| format!("write report: {}", path))?;
    Ok(())
}

fn write_text_file(path: &camino::Utf8Path, text: &str) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
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
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
