use anyhow::Context;
use camino::{Utf8Path, Utf8PathBuf};
use clap::{Parser, Subcommand};
use depguard_domain::policy::Scope as DomainScope;
use depguard_repo::ScopeInput;
use depguard_types::{DepguardReport, ReportEnvelope, ToolMeta, Verdict};
use std::process::Command;
use time::OffsetDateTime;

#[derive(Parser, Debug)]
#[command(name = "depguard", version, about = "Dependency policy guard for Rust workspaces")]
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

        /// Write a Markdown report alongside the JSON.
        #[arg(long)]
        write_markdown: bool,

        /// Where to write the Markdown report (if enabled).
        #[arg(long, default_value = "artifacts/depguard/report.md")]
        markdown_out: Utf8PathBuf,
    },

    /// Render markdown from an existing JSON report (future).
    Md {},

    /// Render GitHub Actions annotations from an existing JSON report (future).
    Annotations {},

    /// Explain a check_id (future).
    Explain {
        check_id: String,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.cmd {
        Commands::Check { base, head, report_out, write_markdown, markdown_out } => {
            run_check(&cli, base, head, report_out, write_markdown, markdown_out)
        }
        Commands::Md {} => {
            anyhow::bail!("md subcommand is a scaffold placeholder (not implemented yet)")
        }
        Commands::Annotations {} => {
            anyhow::bail!("annotations subcommand is a scaffold placeholder (not implemented yet)")
        }
        Commands::Explain { check_id } => {
            anyhow::bail!("explain subcommand is a scaffold placeholder: {check_id}")
        }
    }
}

fn run_check(
    cli: &Cli,
    base: Option<String>,
    head: Option<String>,
    report_out: Utf8PathBuf,
    write_markdown: bool,
    markdown_out: Utf8PathBuf,
) -> anyhow::Result<()> {
    let started_at = OffsetDateTime::now_utc();

    let repo_root = cli.repo_root.canonicalize_utf8().unwrap_or(cli.repo_root.clone());

    // Load config if present; missing file is allowed (defaults apply).
    let cfg_text = match std::fs::read_to_string(repo_root.join(&cli.config)) {
        Ok(s) => s,
        Err(_) => String::new(),
    };

    let cfg = if cfg_text.trim().is_empty() {
        depguard_settings::DepguardConfigV1::default()
    } else {
        depguard_settings::parse_config_toml(&cfg_text).context("parse config")?
    };

    let resolved = depguard_settings::resolve_config(
        cfg,
        depguard_settings::Overrides {
            profile: cli.profile.clone(),
            scope: cli.scope.clone(),
            max_findings: cli.max_findings,
        },
    )
    .context("resolve config")?;

    let scope_input = match resolved.effective.scope {
        DomainScope::Repo => ScopeInput::Repo,
        DomainScope::Diff => {
            let base = base.context("diff scope requires --base")?;
            let head = head.context("diff scope requires --head")?;
            let changed_files = git_changed_files(&repo_root, &base, &head)
                .context("git diff --name-only failed")?;
            ScopeInput::Diff { changed_files }
        }
    };

    let model = depguard_repo::build_workspace_model(&repo_root, scope_input)
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

    write_json(&report_out, &report).context("write report json")?;

    if write_markdown {
        let md = depguard_render::render_markdown(&report);
        write_text(&markdown_out, &md).context("write markdown")?;
    }

    // Exit codes: 0 pass, 1 warn, 2 fail.
    match report.verdict {
        Verdict::Pass => Ok(()),
        Verdict::Warn => std::process::exit(1),
        Verdict::Fail => std::process::exit(2),
    }
}

fn write_json(path: &Utf8Path, report: &DepguardReport) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let data = serde_json::to_vec_pretty(report)?;
    std::fs::write(path, data)?;
    Ok(())
}

fn write_text(path: &Utf8Path, text: &str) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, text)?;
    Ok(())
}

fn git_changed_files(repo_root: &Utf8Path, base: &str, head: &str) -> anyhow::Result<Vec<depguard_types::RepoPath>> {
    let output = Command::new("git")
        .current_dir(repo_root)
        .args(["diff", "--name-only", &format!("{base}..{head}")])
        .output()
        .context("spawn git")?;

    if !output.status.success() {
        anyhow::bail!("git diff returned non-zero exit status");
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let paths = stdout
        .lines()
        .map(|l| depguard_types::RepoPath::new(l.trim()))
        .collect::<Vec<_>>();

    Ok(paths)
}
