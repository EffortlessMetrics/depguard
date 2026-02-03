use anyhow::Context;
use camino::{Utf8Path, Utf8PathBuf};
use clap::{Parser, Subcommand};
use depguard_domain::policy::Scope as DomainScope;
use depguard_repo::ScopeInput;
use depguard_types::{DepguardReport, ReportEnvelope, ToolMeta, Verdict};
use std::process::Command;
use time::OffsetDateTime;

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

        /// Write a Markdown report alongside the JSON.
        #[arg(long)]
        write_markdown: bool,

        /// Where to write the Markdown report (if enabled).
        #[arg(long, default_value = "artifacts/depguard/report.md")]
        markdown_out: Utf8PathBuf,
    },

    /// Render markdown from an existing JSON report.
    Md {
        /// Path to the JSON report file.
        #[arg(long)]
        report: Utf8PathBuf,

        /// Where to write the Markdown output (if not specified, prints to stdout).
        #[arg(long, short)]
        output: Option<Utf8PathBuf>,
    },

    /// Render GitHub Actions annotations from an existing JSON report.
    Annotations {
        /// Path to the JSON report file.
        #[arg(long)]
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
            write_markdown,
            ref markdown_out,
        } => run_check(
            &cli,
            base.clone(),
            head.clone(),
            report_out.clone(),
            write_markdown,
            markdown_out.clone(),
        ),
        Commands::Md { report, output } => run_md(report, output),
        Commands::Annotations { report, max } => run_annotations(report, max),
        Commands::Explain { identifier } => run_explain(&identifier),
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

    let repo_root = cli
        .repo_root
        .canonicalize_utf8()
        .unwrap_or(cli.repo_root.clone());

    // Load config if present; missing file is allowed (defaults apply).
    let cfg_text = std::fs::read_to_string(repo_root.join(&cli.config)).unwrap_or_default();

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

fn git_changed_files(
    repo_root: &Utf8Path,
    base: &str,
    head: &str,
) -> anyhow::Result<Vec<depguard_types::RepoPath>> {
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

fn read_report(path: &Utf8Path) -> anyhow::Result<DepguardReport> {
    let data = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read report file: {}", path))?;
    let report: DepguardReport = serde_json::from_str(&data)
        .with_context(|| format!("failed to parse report JSON: {}", path))?;
    Ok(report)
}

fn run_md(report_path: Utf8PathBuf, output: Option<Utf8PathBuf>) -> anyhow::Result<()> {
    let report = read_report(&report_path)?;
    let md = depguard_render::render_markdown(&report);

    if let Some(out_path) = output {
        write_text(&out_path, &md).context("write markdown output")?;
    } else {
        print!("{}", md);
    }

    Ok(())
}

fn run_annotations(report_path: Utf8PathBuf, max: usize) -> anyhow::Result<()> {
    let report = read_report(&report_path)?;
    let annotations = depguard_render::render_github_annotations(&report);

    for annotation in annotations.into_iter().take(max) {
        println!("{}", annotation);
    }

    Ok(())
}

fn run_explain(identifier: &str) -> anyhow::Result<()> {
    use depguard_types::explain;

    let Some(exp) = explain::lookup_explanation(identifier) else {
        eprintln!("Unknown check_id or code: {}", identifier);
        eprintln!();
        eprintln!("Available check_ids:");
        for id in explain::all_check_ids() {
            eprintln!("  - {}", id);
        }
        eprintln!();
        eprintln!("Available codes:");
        for code in explain::all_codes() {
            eprintln!("  - {}", code);
        }
        std::process::exit(1);
    };

    println!("{}", exp.title);
    println!("{}", "=".repeat(exp.title.len()));
    println!();
    println!("{}", exp.description);
    println!();
    println!("Remediation");
    println!("-----------");
    println!("{}", exp.remediation);
    println!();
    println!("Examples");
    println!("--------");
    println!();
    println!("Before (violation):");
    println!("```toml");
    println!("{}", exp.examples.before);
    println!("```");
    println!();
    println!("After (fixed):");
    println!("```toml");
    println!("{}", exp.examples.after);
    println!("```");

    Ok(())
}
