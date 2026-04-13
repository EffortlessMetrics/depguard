//! CLI entry point for depguard.
//!
//! This module is intentionally thin: it handles argument parsing, I/O, and exit codes.
//! All business logic lives in the `depguard-app` crate.

#![allow(unexpected_cfgs)]

use anyhow::Context;
use camino::Utf8PathBuf;
use clap::{Parser, Subcommand, ValueEnum};
use depguard_app::{
    CheckInput, ExplainOutput, ReportVariant, ReportVersion, add_artifact, apply_baseline,
    apply_safe_fixes, empty_report, generate_baseline, generate_buildfix_plan, parse_baseline_json,
    parse_report_json, render_annotations, render_jsonl, render_junit, render_markdown,
    render_sarif, run_check, run_explain, runtime_error_report, serialize_baseline,
    serialize_buildfix_plan, serialize_report, to_renderable, verdict_exit_code,
};
use depguard_settings::Overrides;
use depguard_types::RepoPath;
use depguard_types::{ArtifactPointer, ArtifactType};
use depguard_yanked::{YankedIndex, parse_yanked_index};
use reqwest::blocking::Client;
use std::collections::BTreeSet;
use std::io::Read;
use std::process::Command;
use std::time::Duration;

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
    diff_file: Option<Utf8PathBuf>,
    yanked_index: Option<Utf8PathBuf>,
    yanked_live: bool,
    yanked_api_base_url: Option<String>,
    incremental: bool,
    cache_dir: Option<Utf8PathBuf>,
    baseline: Option<Utf8PathBuf>,
    out_dir: Option<Utf8PathBuf>,
    report_out: Option<Utf8PathBuf>,
    report_version: String,
    write_markdown: bool,
    markdown_out: Option<Utf8PathBuf>,
    write_junit: bool,
    junit_out: Option<Utf8PathBuf>,
    write_jsonl: bool,
    jsonl_out: Option<Utf8PathBuf>,
    mode: RunMode,
}

/// Options for the baseline command.
struct BaselineOpts {
    base: Option<String>,
    head: Option<String>,
    diff_file: Option<Utf8PathBuf>,
    yanked_index: Option<Utf8PathBuf>,
    yanked_live: bool,
    yanked_api_base_url: Option<String>,
    incremental: bool,
    cache_dir: Option<Utf8PathBuf>,
    output: Utf8PathBuf,
}

#[derive(Parser, Debug, Clone)]
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
        /// In diff scope: read changed file paths from file instead of calling git.
        ///
        /// Accepts plain newline-separated paths and GitHub Actions output formats.
        #[arg(long)]
        diff_file: Option<Utf8PathBuf>,

        /// Offline yanked-version index file used by deps.yanked_versions.
        #[arg(long)]
        yanked_index: Option<Utf8PathBuf>,

        /// Enable live crates.io yanked-version lookup for deps.yanked_versions.
        #[arg(long)]
        yanked_live: bool,

        /// Base URL for the yanked-version API (advanced/testing).
        ///
        /// Defaults to <https://crates.io>.
        #[arg(long)]
        yanked_api_base_url: Option<String>,

        /// Enable incremental mode by caching parsed manifests between runs.
        #[arg(long)]
        incremental: bool,

        /// Directory for incremental cache data.
        ///
        /// Defaults to `.depguard-cache` when --incremental is enabled.
        #[arg(long)]
        cache_dir: Option<Utf8PathBuf>,

        /// Base directory for generated artifacts.
        ///
        /// Defaults to `artifacts/depguard` if not specified.
        #[arg(long)]
        out_dir: Option<Utf8PathBuf>,

        /// Where to write the JSON report.
        ///
        /// Defaults to `<out-dir>/report.json`.
        #[arg(long)]
        report_out: Option<Utf8PathBuf>,

        /// Optional baseline file path to suppress known findings.
        #[arg(long)]
        baseline: Option<Utf8PathBuf>,

        /// Report schema version to emit (v1, v2, or sensor-v1).
        #[arg(long, default_value = "v2")]
        report_version: String,

        /// Write a Markdown report alongside the JSON.
        #[arg(long)]
        write_markdown: bool,

        /// Where to write the Markdown report (if enabled).
        ///
        /// Defaults to `<out-dir>/comment.md`.
        #[arg(long)]
        markdown_out: Option<Utf8PathBuf>,

        /// Write a JUnit XML report alongside the JSON.
        #[arg(long)]
        write_junit: bool,

        /// Where to write the JUnit XML report (if enabled).
        ///
        /// Defaults to `<out-dir>/report.junit.xml`.
        #[arg(long)]
        junit_out: Option<Utf8PathBuf>,

        /// Write newline-delimited JSON (JSONL) findings alongside the JSON report.
        #[arg(long)]
        write_jsonl: bool,

        /// Where to write the JSONL report (if enabled).
        ///
        /// Defaults to `<out-dir>/report.jsonl`.
        #[arg(long)]
        jsonl_out: Option<Utf8PathBuf>,

        /// Run mode: standard (exit 2 on fail) or cockpit (exit 0 if receipt written).
        #[arg(long, value_enum, default_value = "standard")]
        mode: RunMode,
    },

    /// Generate a baseline file from current findings.
    Baseline {
        /// In diff scope: git base revision (e.g. origin/main).
        #[arg(long)]
        base: Option<String>,
        /// In diff scope: git head revision (e.g. HEAD).
        #[arg(long)]
        head: Option<String>,
        /// In diff scope: read changed file paths from file instead of calling git.
        #[arg(long)]
        diff_file: Option<Utf8PathBuf>,
        /// Offline yanked-version index file used by deps.yanked_versions.
        #[arg(long)]
        yanked_index: Option<Utf8PathBuf>,
        /// Enable live crates.io yanked-version lookup for deps.yanked_versions.
        #[arg(long)]
        yanked_live: bool,
        /// Base URL for the yanked-version API (advanced/testing).
        ///
        /// Defaults to <https://crates.io>.
        #[arg(long)]
        yanked_api_base_url: Option<String>,
        /// Enable incremental mode by caching parsed manifests between runs.
        #[arg(long)]
        incremental: bool,
        /// Directory for incremental cache data.
        ///
        /// Defaults to `.depguard-cache` when --incremental is enabled.
        #[arg(long)]
        cache_dir: Option<Utf8PathBuf>,
        /// Where to write the baseline JSON.
        #[arg(long, default_value = ".depguard-baseline.json")]
        output: Utf8PathBuf,
    },

    /// Render report outputs from an existing JSON report.
    Report {
        #[command(subcommand)]
        format: ReportFormat,
    },

    /// Run a CI-native check workflow for a provider.
    Ci {
        /// CI provider adapter.
        #[command(subcommand)]
        provider: CiProvider,
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

    /// Render SARIF from an existing JSON report.
    Sarif {
        /// Path to the JSON report file.
        #[arg(long, default_value = "artifacts/depguard/report.json")]
        report: Utf8PathBuf,

        /// Where to write the SARIF output (if not specified, prints to stdout).
        #[arg(long, short)]
        output: Option<Utf8PathBuf>,
    },

    /// Render JUnit XML from an existing JSON report.
    Junit {
        /// Path to the JSON report file.
        #[arg(long, default_value = "artifacts/depguard/report.json")]
        report: Utf8PathBuf,

        /// Where to write the JUnit XML output (if not specified, prints to stdout).
        #[arg(long, short)]
        output: Option<Utf8PathBuf>,
    },

    /// Render JSON Lines from an existing JSON report.
    Jsonl {
        /// Path to the JSON report file.
        #[arg(long, default_value = "artifacts/depguard/report.json")]
        report: Utf8PathBuf,

        /// Where to write the JSONL output (if not specified, prints to stdout).
        #[arg(long, short)]
        output: Option<Utf8PathBuf>,
    },

    /// Explain a check_id or code with remediation guidance.
    Explain {
        /// The check_id (e.g., "deps.no_wildcards") or code (e.g., "wildcard_version") to explain.
        identifier: String,
    },

    /// Generate a buildfix plan and optionally apply safe fixes.
    Fix {
        /// Path to the source depguard report file.
        #[arg(long, default_value = "artifacts/depguard/report.json")]
        report: Utf8PathBuf,

        /// Where to write the buildfix plan JSON.
        #[arg(long, default_value = "artifacts/buildfix/plan.json")]
        plan_out: Utf8PathBuf,

        /// Apply conservative safe fixes in-place after writing the plan.
        #[arg(long)]
        apply: bool,
    },
}

#[derive(Subcommand, Debug)]
enum CiProvider {
    /// GitHub Actions-oriented CI mode.
    Github {
        /// CI event source to infer scope (auto|pull_request|push|schedule|workflow_call).
        #[arg(long, default_value = "auto", value_enum)]
        event: CiEvent,

        /// In diff scope: git base revision (for example main or origin/main).
        #[arg(long)]
        base: Option<String>,

        /// In diff scope: git head revision (for example HEAD).
        #[arg(long)]
        head: Option<String>,

        /// In diff scope, read changed files from file instead of calling git.
        ///
        /// Accepts plain newline-separated paths and GitHub Actions output formats.
        #[arg(long)]
        diff_file: Option<Utf8PathBuf>,

        /// Write a Markdown report alongside the JSON.
        #[arg(long, default_value_t = true)]
        write_markdown: bool,

        /// Emit GitHub annotations to stdout.
        #[arg(long, default_value_t = true)]
        emit_annotations: bool,

        /// Write a JUnit XML report alongside the JSON.
        #[arg(long)]
        write_junit: bool,

        /// Write JSONL findings alongside the JSON report.
        #[arg(long)]
        write_jsonl: bool,

        /// Write SARIF report alongside the JSON.
        #[arg(long)]
        write_sarif: bool,

        /// Maximum annotations emitted when `emit-annotations` is true.
        #[arg(long, default_value = "10")]
        max_annotations: usize,

        /// Base directory for generated artifacts.
        ///
        /// Defaults to `artifacts/depguard` if not specified.
        #[arg(long)]
        out_dir: Option<Utf8PathBuf>,

        /// Where to write the JSON report.
        ///
        /// Defaults to `<out-dir>/report.json`.
        #[arg(long)]
        report_out: Option<Utf8PathBuf>,
    },
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, ValueEnum)]
enum CiEvent {
    /// Infer CI event from GitHub Actions environment variables.
    #[default]
    #[value(rename = "auto")]
    Auto,
    /// Pull request event; run `diff` scope.
    #[value(rename = "pull_request", alias = "pull-request")]
    PullRequest,
    /// Push or direct branch events; run `repo` scope.
    #[value(rename = "push")]
    Push,
    /// Scheduled jobs; run `repo` scope.
    #[value(rename = "schedule")]
    Schedule,
    /// Reusable workflow call; run `repo` scope unless a diff file is provided.
    #[value(rename = "workflow_call", alias = "workflow-call")]
    WorkflowCall,
}

#[derive(Subcommand, Debug)]
enum ReportFormat {
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

    /// Render SARIF from an existing JSON report.
    Sarif {
        /// Path to the JSON report file.
        #[arg(long, default_value = "artifacts/depguard/report.json")]
        report: Utf8PathBuf,

        /// Where to write the SARIF output (if not specified, prints to stdout).
        #[arg(long, short)]
        output: Option<Utf8PathBuf>,
    },

    /// Render JUnit XML from an existing JSON report.
    Junit {
        /// Path to the JSON report file.
        #[arg(long, default_value = "artifacts/depguard/report.json")]
        report: Utf8PathBuf,

        /// Where to write the JUnit XML output (if not specified, prints to stdout).
        #[arg(long, short)]
        output: Option<Utf8PathBuf>,
    },

    /// Render JSON Lines from an existing JSON report.
    Jsonl {
        /// Path to the JSON report file.
        #[arg(long, default_value = "artifacts/depguard/report.json")]
        report: Utf8PathBuf,

        /// Where to write the JSONL output (if not specified, prints to stdout).
        #[arg(long, short)]
        output: Option<Utf8PathBuf>,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.cmd {
        Commands::Check {
            ref base,
            ref head,
            ref diff_file,
            ref yanked_index,
            yanked_live,
            ref yanked_api_base_url,
            incremental,
            ref cache_dir,
            ref out_dir,
            ref baseline,
            ref report_out,
            ref report_version,
            write_markdown,
            ref markdown_out,
            write_junit,
            ref junit_out,
            write_jsonl,
            ref jsonl_out,
            mode,
        } => cmd_check(
            &cli,
            CheckOpts {
                base: base.clone(),
                head: head.clone(),
                diff_file: diff_file.clone(),
                yanked_index: yanked_index.clone(),
                yanked_live,
                yanked_api_base_url: yanked_api_base_url.clone(),
                incremental,
                cache_dir: cache_dir.clone(),
                baseline: baseline.clone(),
                out_dir: out_dir.clone(),
                report_out: report_out.clone(),
                report_version: report_version.clone(),
                write_markdown,
                markdown_out: markdown_out.clone(),
                write_junit,
                junit_out: junit_out.clone(),
                write_jsonl,
                jsonl_out: jsonl_out.clone(),
                mode,
            },
        ),
        Commands::Baseline {
            ref base,
            ref head,
            ref diff_file,
            ref yanked_index,
            yanked_live,
            ref yanked_api_base_url,
            incremental,
            ref cache_dir,
            ref output,
        } => cmd_baseline(
            &cli,
            BaselineOpts {
                base: base.clone(),
                head: head.clone(),
                diff_file: diff_file.clone(),
                yanked_index: yanked_index.clone(),
                yanked_live,
                yanked_api_base_url: yanked_api_base_url.clone(),
                incremental,
                cache_dir: cache_dir.clone(),
                output: output.clone(),
            },
        ),
        Commands::Md { report, output } => cmd_md(report, output),
        Commands::Annotations { report, max } => cmd_annotations(report, max),
        Commands::Sarif { report, output } => cmd_sarif(report, output),
        Commands::Junit { report, output } => cmd_junit(report, output),
        Commands::Jsonl { report, output } => cmd_jsonl(report, output),
        Commands::Explain { identifier } => cmd_explain(&identifier),
        Commands::Fix {
            report,
            plan_out,
            apply,
        } => cmd_fix(&cli.repo_root, report, plan_out, apply),
        Commands::Ci { provider } => match provider {
            CiProvider::Github {
                event,
                base,
                head,
                diff_file,
                write_markdown,
                emit_annotations,
                write_junit,
                write_jsonl,
                write_sarif,
                max_annotations,
                out_dir,
                report_out,
            } => cmd_ci_github(
                &cli,
                event,
                base,
                head,
                diff_file,
                write_markdown,
                emit_annotations,
                write_junit,
                write_jsonl,
                write_sarif,
                max_annotations,
                out_dir,
                report_out,
            ),
        },
        Commands::Report { format } => match format {
            ReportFormat::Md { report, output } => cmd_md(report, output),
            ReportFormat::Annotations { report, max } => cmd_annotations(report, max),
            ReportFormat::Sarif { report, output } => cmd_sarif(report, output),
            ReportFormat::Junit { report, output } => cmd_junit(report, output),
            ReportFormat::Jsonl { report, output } => cmd_jsonl(report, output),
        },
    }
}

#[derive(Clone, Debug)]
struct OutputPaths {
    report_out: Utf8PathBuf,
    markdown_out: Utf8PathBuf,
    junit_out: Utf8PathBuf,
    jsonl_out: Utf8PathBuf,
}

fn resolve_output_paths(opts: &CheckOpts) -> OutputPaths {
    let out_dir = opts
        .out_dir
        .clone()
        .unwrap_or_else(|| Utf8PathBuf::from("artifacts/depguard"));
    OutputPaths {
        report_out: opts
            .report_out
            .clone()
            .unwrap_or_else(|| out_dir.join("report.json")),
        markdown_out: opts
            .markdown_out
            .clone()
            .unwrap_or_else(|| out_dir.join("comment.md")),
        junit_out: opts
            .junit_out
            .clone()
            .unwrap_or_else(|| out_dir.join("report.junit.xml")),
        jsonl_out: opts
            .jsonl_out
            .clone()
            .unwrap_or_else(|| out_dir.join("report.jsonl")),
    }
}

fn write_optional_artifacts(
    report: &mut ReportVariant,
    opts: &CheckOpts,
    paths: &OutputPaths,
) -> anyhow::Result<()> {
    if !(opts.write_markdown || opts.write_junit || opts.write_jsonl) {
        return Ok(());
    }

    let renderable = to_renderable(report);

    if opts.write_markdown {
        let markdown = render_markdown(&renderable);
        write_text_file(&paths.markdown_out, &markdown).context("write markdown")?;
        add_artifact(
            report,
            ArtifactPointer {
                artifact_type: ArtifactType::Comment,
                path: paths.markdown_out.to_string(),
                format: Some("text/markdown".to_string()),
            },
        );
    }

    if opts.write_junit {
        let junit = render_junit(&renderable);
        write_text_file(&paths.junit_out, &junit).context("write junit")?;
        add_artifact(
            report,
            ArtifactPointer {
                artifact_type: ArtifactType::Extra,
                path: paths.junit_out.to_string(),
                format: Some("application/junit+xml".to_string()),
            },
        );
    }

    if opts.write_jsonl {
        let jsonl = render_jsonl(&renderable);
        write_text_file(&paths.jsonl_out, &jsonl).context("write jsonl")?;
        add_artifact(
            report,
            ArtifactPointer {
                artifact_type: ArtifactType::Extra,
                path: paths.jsonl_out.to_string(),
                format: Some("application/x-ndjson".to_string()),
            },
        );
    }

    Ok(())
}

fn cmd_check(cli: &Cli, opts: CheckOpts) -> anyhow::Result<()> {
    let repo_root = cli
        .repo_root
        .canonicalize_utf8()
        .unwrap_or_else(|_| cli.repo_root.clone());
    let paths = resolve_output_paths(&opts);

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
            baseline: opts.baseline.as_ref().map(|p| p.to_string()),
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
                depguard::policy::Scope::Repo => "repo",
                depguard::policy::Scope::Diff => "diff",
            };
            let mut report = empty_report(report_version, scope, &resolved.effective.profile);
            write_optional_artifacts(&mut report, &opts, &paths)?;
            write_report_file(&paths.report_out, &report).context("write report json")?;
            eprintln!(
                "depguard: no Cargo.toml found at {}; emitting empty report",
                root_manifest
            );
            return Ok(0);
        }

        let changed_files = resolve_changed_files(
            &repo_root,
            &cfg_text,
            cli.scope.as_deref(),
            opts.base.as_deref(),
            opts.head.as_deref(),
            opts.diff_file.as_deref(),
        )
        .context("resolve diff scope inputs")?;
        let scope_input = scope_input_from_changed_files(changed_files.as_ref());
        let yanked_index = load_yanked_index(
            &repo_root,
            opts.yanked_index.as_ref(),
            opts.yanked_live,
            opts.yanked_api_base_url.as_deref(),
            &scope_input,
        )?;
        let manifest_cache_dir =
            effective_cache_dir(opts.incremental, opts.cache_dir.as_ref().cloned());

        let input = CheckInput {
            repo_root: &repo_root,
            config_text: &cfg_text,
            overrides,
            changed_files,
            report_version,
            yanked_index,
            manifest_cache_dir: manifest_cache_dir.as_deref(),
        };

        let mut output = run_check(input)?;

        if let Some(baseline_path) = output.resolved_config.baseline_path.as_deref() {
            let baseline_path = normalize_input_path(&repo_root, baseline_path);
            let baseline_text = std::fs::read_to_string(&baseline_path)
                .with_context(|| format!("read baseline file: {}", baseline_path))?;
            let baseline = parse_baseline_json(&baseline_text).context("parse baseline file")?;
            let stats = apply_baseline(
                &mut output.report,
                &baseline,
                output.resolved_config.effective.fail_on,
            );
            if stats.suppressed > 0 {
                eprintln!(
                    "depguard: suppressed {} findings using baseline {}",
                    stats.suppressed, baseline_path
                );
            }
        }

        write_optional_artifacts(&mut output.report, &opts, &paths)?;

        write_report_file(&paths.report_out, &output.report).context("write report json")?;

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
            let receipt_written = write_report_file(&paths.report_out, &report).is_ok();
            eprintln!("depguard error: {err:#}");

            // In cockpit mode, exit 0 if we successfully wrote an error receipt.
            match (opts.mode, receipt_written) {
                (RunMode::Cockpit, true) => Ok(()),
                _ => terminate(1),
            }
        }
    }
}

fn cmd_baseline(cli: &Cli, opts: BaselineOpts) -> anyhow::Result<()> {
    let repo_root = cli
        .repo_root
        .canonicalize_utf8()
        .unwrap_or_else(|_| cli.repo_root.clone());
    if !repo_root.exists() {
        anyhow::bail!("repo root does not exist: {}", repo_root);
    }

    let cfg_path = repo_root.join(&cli.config);
    let cfg_text = std::fs::read_to_string(&cfg_path).unwrap_or_default();

    let overrides = Overrides {
        profile: cli.profile.clone(),
        scope: cli.scope.clone(),
        max_findings: cli.max_findings,
        baseline: None,
    };

    let changed_files = resolve_changed_files(
        &repo_root,
        &cfg_text,
        cli.scope.as_deref(),
        opts.base.as_deref(),
        opts.head.as_deref(),
        opts.diff_file.as_deref(),
    )
    .context("resolve diff scope inputs")?;
    let scope_input = scope_input_from_changed_files(changed_files.as_ref());
    let yanked_index = load_yanked_index(
        &repo_root,
        opts.yanked_index.as_ref(),
        opts.yanked_live,
        opts.yanked_api_base_url.as_deref(),
        &scope_input,
    )?;
    let manifest_cache_dir =
        effective_cache_dir(opts.incremental, opts.cache_dir.as_ref().cloned());

    let input = CheckInput {
        repo_root: &repo_root,
        config_text: &cfg_text,
        overrides,
        changed_files,
        report_version: ReportVersion::V2,
        yanked_index,
        manifest_cache_dir: manifest_cache_dir.as_deref(),
    };

    let output = run_check(input).context("run check for baseline generation")?;
    let baseline = generate_baseline(&output.report);
    write_baseline_file(&opts.output, &baseline)?;

    eprintln!(
        "depguard: wrote baseline with {} fingerprints to {}",
        baseline.fingerprints.len(),
        opts.output
    );
    Ok(())
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

fn resolve_changed_files(
    repo_root: &camino::Utf8Path,
    cfg_text: &str,
    cli_scope: Option<&str>,
    base: Option<&str>,
    head: Option<&str>,
    diff_file: Option<&camino::Utf8Path>,
) -> anyhow::Result<Option<Vec<RepoPath>>> {
    let diff_scope_enabled = cli_scope == Some("diff")
        || (cli_scope.is_none() && scope_from_config(cfg_text) == Some("diff"));

    if !diff_scope_enabled {
        if diff_file.is_some() {
            anyhow::bail!("--diff-file requires --scope diff");
        }
        return Ok(None);
    }

    if let Some(diff_file) = diff_file {
        let paths = read_changed_files_from_file(repo_root, diff_file)?;
        return Ok(Some(paths));
    }

    let base = base.context("diff scope requires --base (or --diff-file)")?;
    let head = head.context("diff scope requires --head (or --diff-file)")?;

    let changed =
        git_changed_files(repo_root, base, head).context("git diff --name-only failed")?;
    Ok(Some(changed))
}

fn read_changed_files_from_file(
    repo_root: &camino::Utf8Path,
    diff_file: &camino::Utf8Path,
) -> anyhow::Result<Vec<RepoPath>> {
    let content = if diff_file.as_str() == "-" {
        let mut buf = String::new();
        std::io::stdin()
            .read_to_string(&mut buf)
            .context("read --diff-file from stdin")?;
        buf
    } else {
        let path = if diff_file.is_absolute() {
            diff_file.to_path_buf()
        } else {
            repo_root.join(diff_file)
        };
        std::fs::read_to_string(&path).with_context(|| format!("read diff file: {path}"))?
    };

    Ok(parse_changed_files_input(&content))
}

fn parse_changed_files_input(input: &str) -> Vec<RepoPath> {
    let input = input.trim_start_matches('\u{feff}');
    let chunks = extract_changed_files_chunks(input);

    let mut seen = std::collections::BTreeSet::new();
    let mut paths = Vec::new();

    for chunk in chunks {
        for token in parse_changed_files_chunk(&chunk) {
            let token = token.trim();
            if token.is_empty() {
                continue;
            }
            let normalized = RepoPath::new(token);
            if seen.insert(normalized.as_str().to_string()) {
                paths.push(normalized);
            }
        }
    }

    paths
}

fn parse_changed_files_chunk(chunk: &str) -> Vec<String> {
    let chunk = chunk.trim();
    if chunk.is_empty() {
        return Vec::new();
    }

    if let Ok(values) = serde_json::from_str::<Vec<String>>(chunk) {
        return values;
    }

    tokenize_paths(chunk)
}

fn extract_changed_files_chunks(input: &str) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut lines = input.lines();

    while let Some(line) = lines.next() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some((key, delimiter)) = line.split_once("<<")
            && is_gha_output_key(key)
            && !delimiter.is_empty()
        {
            let mut block = String::new();
            for block_line in lines.by_ref() {
                if block_line.trim() == delimiter {
                    break;
                }
                if !block.is_empty() {
                    block.push('\n');
                }
                block.push_str(block_line.trim_end());
            }
            chunks.push(block);
            continue;
        }

        if let Some((key, value)) = line.split_once('=')
            && is_gha_output_key(key)
            && !value.trim().is_empty()
        {
            chunks.push(value.trim().to_string());
            continue;
        }

        chunks.push(line.to_string());
    }

    if chunks.is_empty() && !input.trim().is_empty() {
        chunks.push(input.trim().to_string());
    }

    chunks
}

fn is_gha_output_key(key: &str) -> bool {
    let key = key.trim();
    !key.is_empty()
        && !key.contains('/')
        && !key.contains('\\')
        && key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.')
}

fn tokenize_paths(input: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;

    for ch in input.chars() {
        match quote {
            Some(q) => {
                if ch == q {
                    quote = None;
                } else {
                    current.push(ch);
                }
            }
            None => {
                if ch == '"' || ch == '\'' {
                    quote = Some(ch);
                } else if ch == ',' || ch.is_whitespace() {
                    if !current.trim().is_empty() {
                        tokens.push(current.trim().to_string());
                    }
                    current.clear();
                } else {
                    current.push(ch);
                }
            }
        }
    }

    if !current.trim().is_empty() {
        tokens.push(current.trim().to_string());
    }

    tokens
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

fn normalize_input_path(repo_root: &camino::Utf8Path, path: &str) -> Utf8PathBuf {
    let path = Utf8PathBuf::from(path);
    if path.is_absolute() {
        path
    } else {
        repo_root.join(path)
    }
}

fn effective_cache_dir(enabled: bool, cache_dir: Option<Utf8PathBuf>) -> Option<Utf8PathBuf> {
    if !enabled {
        return None;
    }
    Some(cache_dir.unwrap_or_else(|| Utf8PathBuf::from(".depguard-cache")))
}

fn scope_input_from_changed_files(
    changed_files: Option<&Vec<RepoPath>>,
) -> depguard_repo::ScopeInput {
    match changed_files {
        Some(paths) => depguard_repo::ScopeInput::Diff {
            changed_files: paths.clone(),
        },
        None => depguard_repo::ScopeInput::Repo,
    }
}

fn load_yanked_index(
    repo_root: &camino::Utf8Path,
    yanked_index_path: Option<&Utf8PathBuf>,
    yanked_live: bool,
    yanked_api_base_url: Option<&str>,
    scope_input: &depguard_repo::ScopeInput,
) -> anyhow::Result<Option<YankedIndex>> {
    let mut merged = if let Some(path) = yanked_index_path {
        let abs_path = if path.is_absolute() {
            path.clone()
        } else {
            repo_root.join(path)
        };

        let text = std::fs::read_to_string(&abs_path)
            .with_context(|| format!("read yanked index file: {}", abs_path))?;
        let index = parse_yanked_index(&text)
            .with_context(|| format!("parse yanked index file: {}", abs_path))?;
        Some(index)
    } else {
        None
    };

    if yanked_live {
        let model = depguard_repo::build_workspace_model(repo_root, scope_input.clone())
            .context("build workspace model for live yanked lookup")?;
        let pins = collect_exact_pins(&model);
        let live = fetch_live_yanked_index(&pins, yanked_api_base_url)?;
        match merged.as_mut() {
            Some(existing) => existing.merge(live),
            None => merged = Some(live),
        }
    }

    Ok(merged)
}

fn fetch_live_yanked_index(
    pins: &BTreeSet<(String, String)>,
    yanked_api_base_url: Option<&str>,
) -> anyhow::Result<YankedIndex> {
    if pins.is_empty() {
        return Ok(YankedIndex::default());
    }

    let base_url = yanked_api_base_url
        .map(str::to_string)
        .or_else(|| std::env::var("DEPGUARD_YANKED_API_BASE_URL").ok())
        .unwrap_or_else(|| "https://crates.io".to_string());
    let base_url = base_url.trim_end_matches('/').to_string();

    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .user_agent(format!("depguard/{}", env!("CARGO_PKG_VERSION")))
        .build()
        .context("build HTTP client for yanked lookup")?;

    let mut index = YankedIndex::default();
    for (crate_name, version) in pins {
        let url = format!("{base_url}/api/v1/crates/{crate_name}/{version}");
        let response = client
            .get(&url)
            .send()
            .with_context(|| format!("request yanked status for {crate_name} {version}"))?;

        let status = response.status();
        if status == reqwest::StatusCode::NOT_FOUND {
            continue;
        }
        if !status.is_success() {
            anyhow::bail!(
                "yanked lookup failed for {crate_name} {version}: HTTP {} from {}",
                status,
                url
            );
        }

        let body: serde_json::Value = response
            .json()
            .with_context(|| format!("parse yanked API response from {url}"))?;
        let yanked = body
            .get("version")
            .and_then(|v| v.get("yanked"))
            .and_then(|v| v.as_bool())
            .context(format!(
                "yanked API response missing version.yanked for {crate_name} {version}"
            ))?;
        if yanked {
            index.insert(crate_name, version);
        }
    }

    Ok(index)
}

fn collect_exact_pins(model: &depguard::model::WorkspaceModel) -> BTreeSet<(String, String)> {
    let mut pins = BTreeSet::new();
    for manifest in &model.manifests {
        for dep in &manifest.dependencies {
            if dep.spec.workspace {
                continue;
            }
            let Some(version_req) = dep.spec.version.as_deref() else {
                continue;
            };
            let Some(pinned) = pinned_version(version_req) else {
                continue;
            };
            let canonical_name = dep.spec.package.as_deref().unwrap_or(&dep.name);
            pins.insert((canonical_name.to_string(), pinned.to_string()));
        }
    }
    pins
}

fn pinned_version(version_req: &str) -> Option<&str> {
    let trimmed = version_req.trim();
    let rest = trimmed.strip_prefix('=')?.trim();
    if rest.is_empty() { None } else { Some(rest) }
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

fn write_baseline_file(
    path: &camino::Utf8Path,
    baseline: &depguard_types::DepguardBaselineV1,
) -> anyhow::Result<()> {
    if let Some(parent) = path.parent().filter(|p| !p.as_str().is_empty()) {
        std::fs::create_dir_all(parent).with_context(|| format!("create directory: {}", parent))?;
    }
    let data = serialize_baseline(baseline).context("serialize baseline")?;
    std::fs::write(path, data).with_context(|| format!("write baseline: {}", path))?;
    Ok(())
}

fn write_buildfix_plan_file(
    path: &camino::Utf8Path,
    plan: &depguard_types::BuildfixPlanV1,
) -> anyhow::Result<()> {
    if let Some(parent) = path.parent().filter(|p| !p.as_str().is_empty()) {
        std::fs::create_dir_all(parent).with_context(|| format!("create directory: {}", parent))?;
    }
    let data = serialize_buildfix_plan(plan).context("serialize buildfix plan")?;
    std::fs::write(path, data).with_context(|| format!("write buildfix plan: {}", path))?;
    Ok(())
}

fn cmd_fix(
    repo_root_arg: &Utf8PathBuf,
    report_path: Utf8PathBuf,
    plan_out: Utf8PathBuf,
    apply: bool,
) -> anyhow::Result<()> {
    let report_text = std::fs::read_to_string(&report_path)
        .with_context(|| format!("read report: {}", report_path))?;
    let report = parse_report_json(&report_text)?;

    let plan = generate_buildfix_plan(&report, report_path.as_str(), !apply);
    write_buildfix_plan_file(&plan_out, &plan)?;

    eprintln!(
        "depguard: wrote buildfix plan with {} safe fix actions to {}",
        plan.fixes.len(),
        plan_out
    );

    if !apply {
        return Ok(());
    }

    let repo_root = repo_root_arg
        .canonicalize_utf8()
        .unwrap_or_else(|_| repo_root_arg.clone());
    let result = apply_safe_fixes(&repo_root, &report);

    eprintln!(
        "depguard: applied {} of {} planned safe fixes ({} skipped, {} failed)",
        result.applied, result.planned, result.skipped, result.failed
    );

    if result.failed > 0 {
        anyhow::bail!(
            "failed to apply {} planned fixes; see prior output",
            result.failed
        );
    }

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

fn cmd_sarif(report_path: Utf8PathBuf, output: Option<Utf8PathBuf>) -> anyhow::Result<()> {
    let report_text = std::fs::read_to_string(&report_path)
        .with_context(|| format!("read report: {}", report_path))?;
    let report = parse_report_json(&report_text)?;
    let renderable = to_renderable(&report);
    let sarif = render_sarif(&renderable);

    if let Some(out_path) = output {
        write_text_file(&out_path, &sarif).context("write sarif output")?;
    } else {
        print!("{}", sarif);
    }

    Ok(())
}

fn cmd_junit(report_path: Utf8PathBuf, output: Option<Utf8PathBuf>) -> anyhow::Result<()> {
    let report_text = std::fs::read_to_string(&report_path)
        .with_context(|| format!("read report: {}", report_path))?;
    let report = parse_report_json(&report_text)?;
    let renderable = to_renderable(&report);
    let junit = render_junit(&renderable);

    if let Some(out_path) = output {
        write_text_file(&out_path, &junit).context("write junit output")?;
    } else {
        print!("{}", junit);
    }

    Ok(())
}

fn cmd_jsonl(report_path: Utf8PathBuf, output: Option<Utf8PathBuf>) -> anyhow::Result<()> {
    let report_text = std::fs::read_to_string(&report_path)
        .with_context(|| format!("read report: {}", report_path))?;
    let report = parse_report_json(&report_text)?;
    let renderable = to_renderable(&report);
    let jsonl = render_jsonl(&renderable);

    if let Some(out_path) = output {
        write_text_file(&out_path, &jsonl).context("write jsonl output")?;
    } else {
        print!("{}", jsonl);
    }

    Ok(())
}

fn cmd_annotations(report_path: Utf8PathBuf, max: usize) -> anyhow::Result<()> {
    let annotations = render_annotations_text(&report_path, max)?;
    print!("{}", annotations);

    Ok(())
}

fn cmd_ci_github(
    cli: &Cli,
    event: CiEvent,
    base: Option<String>,
    head: Option<String>,
    diff_file: Option<Utf8PathBuf>,
    write_markdown: bool,
    emit_annotations: bool,
    write_junit: bool,
    write_jsonl: bool,
    write_sarif: bool,
    max_annotations: usize,
    out_dir: Option<Utf8PathBuf>,
    report_out: Option<Utf8PathBuf>,
) -> anyhow::Result<()> {
    let resolved_event = resolve_ci_github_event(event)?;
    let mut scope = match resolved_event {
        CiEvent::PullRequest => "diff",
        _ => "repo",
    };

    let mut base = base;
    let mut head = head;
    let diff_file = diff_file;

    if diff_file.is_some() {
        scope = "diff";
    }

    if scope == "diff" && diff_file.is_none() {
        if base.is_none() {
            base = Some(ci_default_base_ref(&resolved_event)?);
        }
        if head.is_none() {
            head = Some(ci_default_head_ref());
        }
    }

    let mut ci_cli = cli.clone();
    ci_cli.scope = Some(scope.to_string());

    let run_opts = CheckOpts {
        base,
        head,
        diff_file,
        yanked_index: None,
        yanked_live: false,
        yanked_api_base_url: None,
        incremental: false,
        cache_dir: None,
        baseline: None,
        out_dir,
        report_out,
        report_version: "v2".to_string(),
        write_markdown,
        markdown_out: None,
        write_junit,
        junit_out: None,
        write_jsonl,
        jsonl_out: None,
        mode: RunMode::Cockpit,
    };

    let run_paths = resolve_output_paths(&run_opts);
    cmd_check(&ci_cli, run_opts)?;

    let report_path = run_paths.report_out;
    let report_text = std::fs::read_to_string(&report_path)
        .context("read report after ci run")?;
    let report = parse_report_json(&report_text)?;
    let code = report_exit_code(&report);

    if emit_annotations {
        cmd_annotations(report_path, max_annotations)?;
    }

    if write_sarif {
        let sarif_out = run_paths
            .report_out
            .parent()
            .unwrap_or_else(|| std::path::Path::new("artifacts/depguard"))
            .join("report.sarif.json");
        cmd_sarif(
            report_path.clone(),
            Some(Utf8PathBuf::from_path_buf(sarif_out).expect("valid utf8 path")),
        )?;
    }

    match code {
        0 => Ok(()),
        2 => terminate(2),
        1 => terminate(1),
        other => terminate(other),
    }
}

fn render_annotations_text(report_path: &Utf8PathBuf, max: usize) -> anyhow::Result<String> {
    let report_text = std::fs::read_to_string(report_path)
        .with_context(|| format!("read report: {report_path}"))?;
    let report = parse_report_json(&report_text)?;
    let renderable = to_renderable(&report);
    let annotations = render_annotations(&renderable, max);

    let mut out = String::new();
    for annotation in annotations {
        out.push_str(&annotation);
        out.push('\n');
    }

    Ok(out)
}

fn resolve_ci_github_event(event: CiEvent) -> anyhow::Result<CiEvent> {
    if event != CiEvent::Auto {
        return Ok(event);
    }

    let detected = std::env::var("GITHUB_EVENT_NAME")
        .map(|e| e.to_lowercase())
        .unwrap_or_else(|_| {
            if std::env::var("CI").is_ok() {
                "workflow_call".to_string()
            } else {
                String::new()
            }
        });

    match detected.as_str() {
        "pull_request" | "pull_request_target" | "pull_request_review" | "pull_request_review_comment" => {
            Ok(CiEvent::PullRequest)
        }
        "workflow_call" => Ok(CiEvent::WorkflowCall),
        "schedule" => Ok(CiEvent::Schedule),
        "push" | "workflow_dispatch" => Ok(CiEvent::Push),
        "" => anyhow::bail!(
            "unable to resolve CI event for depguard ci github; pass --event explicitly"
        ),
        unknown => {
            eprintln!("depguard: unrecognized event '{unknown}', defaulting to repo scope");
            Ok(CiEvent::Push)
        }
    }
}

fn ci_default_base_ref(event: &CiEvent) -> anyhow::Result<String> {
    match event {
        CiEvent::PullRequest => {
            let base = std::env::var("GITHUB_BASE_REF")
                .or_else(|_| std::env::var("GITHUB_BASE_SHA"))
                .unwrap_or_else(|| "main".to_string());
            Ok(normalize_ci_ref_base(&base))
        }
        _ => Ok("origin/main".to_string()),
    }
}

fn ci_default_head_ref() -> String {
    std::env::var("GITHUB_SHA").unwrap_or_else(|_| "HEAD".to_string())
}

fn normalize_ci_ref_base(base: &str) -> String {
    if base.contains('/') || base.is_empty() || is_hex_sha(base) {
        base.to_string()
    } else {
        format!("origin/{base}")
    }
}

fn is_hex_sha(value: &str) -> bool {
    value.len() == 40 && value.chars().all(|c| c.is_ascii_hexdigit())
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
    use assert_cmd::Command;
    use std::any::Any;
    use std::panic::{AssertUnwindSafe, catch_unwind};
    use tempfile::TempDir;

    #[test]
    fn legacy_renderer_command_outputs_match_report_renderer_outputs() {
        let tmp = TempDir::new().expect("temp dir");
        let root = Utf8PathBuf::from_path_buf(tmp.path().to_path_buf()).expect("utf8 path");
        let report_path = root.join("artifacts").join("report.json");

        write_sample_report_with_finding(&report_path, "Cargo.toml");

        let legacy_md_out = root.join("legacy.md");
        let report_md_out = root.join("report.md");
        Command::cargo_bin("depguard")
            .unwrap()
            .args([
                "md",
                "--report",
                report_path.as_str(),
                "--output",
                legacy_md_out.as_str(),
            ])
            .assert()
            .success();
        Command::cargo_bin("depguard")
            .unwrap()
            .args([
                "report",
                "md",
                "--report",
                report_path.as_str(),
                "--output",
                report_md_out.as_str(),
            ])
            .assert()
            .success();
        assert_eq!(
            std::fs::read_to_string(legacy_md_out).expect("read legacy md"),
            std::fs::read_to_string(report_md_out).expect("read canonical md"),
        );

        let legacy_sarif_out = root.join("legacy.sarif");
        let report_sarif_out = root.join("report.sarif");
        Command::cargo_bin("depguard")
            .unwrap()
            .args([
                "sarif",
                "--report",
                report_path.as_str(),
                "--output",
                legacy_sarif_out.as_str(),
            ])
            .assert()
            .success();
        Command::cargo_bin("depguard")
            .unwrap()
            .args([
                "report",
                "sarif",
                "--report",
                report_path.as_str(),
                "--output",
                report_sarif_out.as_str(),
            ])
            .assert()
            .success();
        assert_eq!(
            std::fs::read_to_string(legacy_sarif_out).expect("read legacy sarif"),
            std::fs::read_to_string(report_sarif_out).expect("read canonical sarif"),
        );

        let legacy_junit_out = root.join("legacy.junit.xml");
        let report_junit_out = root.join("report.junit.xml");
        Command::cargo_bin("depguard")
            .unwrap()
            .args([
                "junit",
                "--report",
                report_path.as_str(),
                "--output",
                legacy_junit_out.as_str(),
            ])
            .assert()
            .success();
        Command::cargo_bin("depguard")
            .unwrap()
            .args([
                "report",
                "junit",
                "--report",
                report_path.as_str(),
                "--output",
                report_junit_out.as_str(),
            ])
            .assert()
            .success();
        assert_eq!(
            std::fs::read_to_string(legacy_junit_out).expect("read legacy junit"),
            std::fs::read_to_string(report_junit_out).expect("read canonical junit"),
        );

        let legacy_jsonl_out = root.join("legacy.jsonl");
        let report_jsonl_out = root.join("report.jsonl");
        Command::cargo_bin("depguard")
            .unwrap()
            .args([
                "jsonl",
                "--report",
                report_path.as_str(),
                "--output",
                legacy_jsonl_out.as_str(),
            ])
            .assert()
            .success();
        Command::cargo_bin("depguard")
            .unwrap()
            .args([
                "report",
                "jsonl",
                "--report",
                report_path.as_str(),
                "--output",
                report_jsonl_out.as_str(),
            ])
            .assert()
            .success();
        assert_eq!(
            std::fs::read_to_string(legacy_jsonl_out).expect("read legacy jsonl"),
            std::fs::read_to_string(report_jsonl_out).expect("read canonical jsonl"),
        );
    }

    #[test]
    fn legacy_annotations_matches_report_annotations_bytes() {
        let tmp = TempDir::new().expect("temp dir");
        let root = Utf8PathBuf::from_path_buf(tmp.path().to_path_buf()).expect("utf8 path");
        let report_path = root.join("artifacts").join("report.json");

        write_sample_report_with_finding(&report_path, "Cargo.toml");

        let legacy = Command::cargo_bin("depguard")
            .unwrap()
            .args(["annotations", "--report", report_path.as_str(), "--max", "10"])
            .output()
            .expect("run legacy annotations");
        assert!(legacy.status.success());

        let canonical = Command::cargo_bin("depguard")
            .unwrap()
            .args([
                "report",
                "annotations",
                "--report",
                report_path.as_str(),
                "--max",
                "10",
            ])
            .output()
            .expect("run canonical annotations");
        assert!(canonical.status.success());

        assert_eq!(legacy.stdout, canonical.stdout);
    }

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
    fn parse_changed_files_input_supports_plain_and_csv_lists() {
        let input = r#"
            crates/a/Cargo.toml
            crates/b/Cargo.toml,crates/c/Cargo.toml
            crates/a/Cargo.toml
        "#;
        let paths = parse_changed_files_input(input);
        let got: Vec<String> = paths.iter().map(|p| p.as_str().to_string()).collect();
        assert_eq!(
            got,
            vec![
                "crates/a/Cargo.toml".to_string(),
                "crates/b/Cargo.toml".to_string(),
                "crates/c/Cargo.toml".to_string(),
            ]
        );
    }

    #[test]
    fn parse_changed_files_input_supports_json_array() {
        let input = r#"["crates/a/Cargo.toml","crates/b/Cargo.toml"]"#;
        let paths = parse_changed_files_input(input);
        let got: Vec<String> = paths.iter().map(|p| p.as_str().to_string()).collect();
        assert_eq!(
            got,
            vec![
                "crates/a/Cargo.toml".to_string(),
                "crates/b/Cargo.toml".to_string(),
            ]
        );
    }

    #[test]
    fn parse_changed_files_input_supports_github_output_assignment() {
        let input = "all_changed_files=crates/a/Cargo.toml crates/b/Cargo.toml";
        let paths = parse_changed_files_input(input);
        let got: Vec<String> = paths.iter().map(|p| p.as_str().to_string()).collect();
        assert_eq!(
            got,
            vec![
                "crates/a/Cargo.toml".to_string(),
                "crates/b/Cargo.toml".to_string(),
            ]
        );
    }

    #[test]
    fn parse_changed_files_input_supports_github_output_multiline_block() {
        let input = r#"
            all_changed_files<<EOF
            crates/a/Cargo.toml
            crates/b/Cargo.toml
            EOF
        "#;
        let paths = parse_changed_files_input(input);
        let got: Vec<String> = paths.iter().map(|p| p.as_str().to_string()).collect();
        assert_eq!(
            got,
            vec![
                "crates/a/Cargo.toml".to_string(),
                "crates/b/Cargo.toml".to_string(),
            ]
        );
    }

    #[test]
    fn resolve_changed_files_rejects_diff_file_without_diff_scope() {
        let err = resolve_changed_files(
            camino::Utf8Path::new("."),
            "",
            Some("repo"),
            None,
            None,
            Some(camino::Utf8Path::new("changed-files.txt")),
        )
        .expect_err("expected error");
        assert!(
            err.to_string()
                .contains("--diff-file requires --scope diff")
        );
    }

    #[test]
    fn cli_parses_diff_file_for_check_subcommand() {
        let cli = Cli::parse_from(["depguard", "check", "--diff-file", "changed-files.txt"]);
        let Commands::Check { diff_file, .. } = cli.cmd else {
            panic!("expected check command");
        };
        assert_eq!(diff_file, Some(Utf8PathBuf::from("changed-files.txt")));
    }

    #[test]
    fn cli_parses_depguard_ci_github_defaults() {
        let cli = Cli::parse_from(["depguard", "ci", "github"]);
        let Commands::Ci { provider } = cli.cmd else {
            panic!("expected ci command");
        };
        let CiProvider::Github {
            event,
            write_markdown,
            emit_annotations,
            write_junit,
            write_jsonl,
            write_sarif,
            max_annotations,
            ..
        } = provider
        else {
            panic!("expected github ci provider");
        };

        assert!(matches!(event, CiEvent::Auto));
        assert!(write_markdown);
        assert!(emit_annotations);
        assert!(!write_junit);
        assert!(!write_jsonl);
        assert!(!write_sarif);
        assert_eq!(max_annotations, 10);
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
    fn ci_base_ref_defaults_prefix_origin() {
        assert_eq!(normalize_ci_ref_base("main"), "origin/main");
        assert_eq!(normalize_ci_ref_base("origin/main"), "origin/main");
        assert_eq!(normalize_ci_ref_base("refs/heads/main"), "refs/heads/main");
        assert_eq!(normalize_ci_ref_base("c0ff33be0a6fd4d5f9f8e5c9f1b2e6d7f8a9b0c1d"), "c0ff33be0a6fd4d5f9f8e5c9f1b2e6d7f8a9b0c1d");
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

    fn write_sample_report_with_finding(report_path: &Utf8PathBuf, manifest_path: &str) {
        let mut report_variant = empty_report(ReportVersion::V2, "repo", "strict");
        let ReportVariant::V2(ref mut report) = report_variant else {
            panic!("expected v2 report");
        };

        report.findings.push(depguard_types::FindingV2 {
            severity: depguard_types::SeverityV2::Error,
            check_id: depguard_types::ids::CHECK_DEPS_NO_WILDCARDS.to_string(),
            code: depguard_types::ids::CODE_WILDCARD_DEPENDENCY.to_string(),
            message: "Test wildcard dependency".to_string(),
            location: Some(depguard_types::Location {
                path: RepoPath::new(manifest_path),
                line: Some(1),
                col: Some(1),
            }),
            help: Some("pin version requirements".to_string()),
            url: Some("https://example.com/depguard/example".to_string()),
            fingerprint: Some("test-fingerprint".to_string()),
            data: serde_json::json!({
                "dependency": "serde",
                "manifest": manifest_path,
            }),
        });

        write_report_file(report_path, &report_variant).expect("write report fixture");
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
                diff_file: None,
                yanked_index: None,
                yanked_live: false,
                yanked_api_base_url: None,
                incremental: false,
                cache_dir: None,
                out_dir: None,
                report_out: Some(Utf8PathBuf::from("report.json")),
                report_version: "v2".to_string(),
                write_markdown: false,
                markdown_out: Some(Utf8PathBuf::from("comment.md")),
                write_junit: false,
                junit_out: None,
                write_jsonl: false,
                jsonl_out: None,
                mode: RunMode::Standard,
                baseline: None,
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

        std::fs::write(root.join("depguard.toml"), "scope = \"diff\"").expect("write config");

        let cli = cli_for_root(&root);
        let report_out = root.join("out").join("report.json");
        let markdown_out = root.join("out").join("comment.md");
        let opts = CheckOpts {
            base: None,
            head: None,
            diff_file: None,
            yanked_index: None,
            yanked_live: false,
            yanked_api_base_url: None,
            incremental: false,
            cache_dir: None,
            baseline: None,
            out_dir: None,
            report_out: Some(report_out.clone()),
            report_version: "v2".to_string(),
            write_markdown: true,
            markdown_out: Some(markdown_out.clone()),
            write_junit: false,
            junit_out: None,
            write_jsonl: false,
            jsonl_out: None,
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
            diff_file: None,
            yanked_index: None,
            yanked_live: false,
            yanked_api_base_url: None,
            incremental: false,
            cache_dir: None,
            baseline: None,
            out_dir: None,
            report_out: Some(report_out.clone()),
            report_version: "v2".to_string(),
            write_markdown: true,
            markdown_out: Some(markdown_out.clone()),
            write_junit: false,
            junit_out: None,
            write_jsonl: false,
            jsonl_out: None,
            mode: RunMode::Standard,
        };

        cmd_check(&cli, opts).expect("cmd_check");
        assert!(report_out.exists());
        assert!(markdown_out.exists());
    }

    #[test]
    fn cmd_check_diff_scope_uses_diff_file_without_git() {
        let tmp = TempDir::new().expect("temp dir");
        let root = Utf8PathBuf::from_path_buf(tmp.path().to_path_buf()).expect("utf8 path");
        write_manifest(&root, r#"serde = "*""#);

        std::fs::write(
            root.join("changed-files.txt"),
            "all_changed_files=Cargo.toml",
        )
        .expect("write diff file");

        let mut cli = cli_for_root(&root);
        cli.scope = Some("diff".to_string());

        let report_out = root.join("diff-file-report.json");
        let opts = CheckOpts {
            base: None,
            head: None,
            diff_file: Some(Utf8PathBuf::from("changed-files.txt")),
            yanked_index: None,
            yanked_live: false,
            yanked_api_base_url: None,
            incremental: false,
            cache_dir: None,
            baseline: None,
            out_dir: None,
            report_out: Some(report_out.clone()),
            report_version: "v2".to_string(),
            write_markdown: false,
            markdown_out: Some(root.join("comment.md")),
            write_junit: false,
            junit_out: None,
            write_jsonl: false,
            jsonl_out: None,
            mode: RunMode::Cockpit,
        };

        cmd_check(&cli, opts).expect("cmd_check");
        assert!(report_out.exists());

        let report_text = std::fs::read_to_string(report_out).expect("read report");
        let report = parse_report_json(&report_text).expect("parse report");
        let ReportVariant::V2(report) = report else {
            panic!("expected v2 report");
        };
        assert_eq!(report.data.scope, "diff");
        assert!(
            !report.findings.is_empty(),
            "expected wildcard finding from changed manifest"
        );
    }

    #[test]
    fn cmd_check_with_yanked_index_flags_pinned_yanked_versions() {
        let tmp = TempDir::new().expect("temp dir");
        let root = Utf8PathBuf::from_path_buf(tmp.path().to_path_buf()).expect("utf8 path");
        write_manifest(&root, r#"serde = "=1.0.188""#);
        std::fs::write(
            root.join("depguard.toml"),
            r#"[checks."deps.yanked_versions"]
enabled = true
severity = "error"
"#,
        )
        .expect("write depguard.toml");
        std::fs::write(root.join("yanked-index.txt"), "serde 1.0.188\n")
            .expect("write yanked index");

        let cli = cli_for_root(&root);
        let report_out = root.join("yanked-report.json");
        let opts = CheckOpts {
            base: None,
            head: None,
            diff_file: None,
            yanked_index: Some(Utf8PathBuf::from("yanked-index.txt")),
            yanked_live: false,
            yanked_api_base_url: None,
            incremental: false,
            cache_dir: None,
            baseline: None,
            out_dir: None,
            report_out: Some(report_out.clone()),
            report_version: "v2".to_string(),
            write_markdown: false,
            markdown_out: Some(root.join("comment.md")),
            write_junit: false,
            junit_out: None,
            write_jsonl: false,
            jsonl_out: None,
            mode: RunMode::Cockpit,
        };

        cmd_check(&cli, opts).expect("cmd_check");
        let report_text = std::fs::read_to_string(report_out).expect("read report");
        let report = parse_report_json(&report_text).expect("parse report");
        let ReportVariant::V2(report) = report else {
            panic!("expected v2 report");
        };
        assert!(
            report
                .findings
                .iter()
                .any(|f| f.code == depguard_types::ids::CODE_VERSION_YANKED),
            "expected version_yanked finding"
        );
    }

    #[test]
    fn cmd_baseline_writes_file() {
        let tmp = TempDir::new().expect("temp dir");
        let root = Utf8PathBuf::from_path_buf(tmp.path().to_path_buf()).expect("utf8 path");
        write_manifest(&root, r#"serde = "*""#);

        let cli = cli_for_root(&root);
        let output = root.join(".depguard-baseline.json");
        let opts = BaselineOpts {
            base: None,
            head: None,
            diff_file: None,
            yanked_index: None,
            yanked_live: false,
            yanked_api_base_url: None,
            incremental: false,
            cache_dir: None,
            output: output.clone(),
        };

        cmd_baseline(&cli, opts).expect("cmd_baseline");
        assert!(output.exists());

        let text = std::fs::read_to_string(output).expect("read baseline");
        let baseline = parse_baseline_json(&text).expect("parse baseline");
        assert!(
            !baseline.fingerprints.is_empty(),
            "expected at least one finding fingerprint"
        );
    }

    #[test]
    fn cmd_check_with_baseline_suppresses_failures() {
        let tmp = TempDir::new().expect("temp dir");
        let root = Utf8PathBuf::from_path_buf(tmp.path().to_path_buf()).expect("utf8 path");
        write_manifest(&root, r#"serde = "*""#);

        let cli = cli_for_root(&root);
        let baseline_path = root.join(".depguard-baseline.json");
        cmd_baseline(
            &cli,
            BaselineOpts {
                base: None,
                head: None,
                diff_file: None,
                yanked_index: None,
                yanked_live: false,
                yanked_api_base_url: None,
                incremental: false,
                cache_dir: None,
                output: baseline_path.clone(),
            },
        )
        .expect("generate baseline");

        let report_out = root.join("report.json");
        let opts = CheckOpts {
            base: None,
            head: None,
            diff_file: None,
            yanked_index: None,
            yanked_live: false,
            yanked_api_base_url: None,
            incremental: false,
            cache_dir: None,
            baseline: Some(Utf8PathBuf::from(".depguard-baseline.json")),
            out_dir: None,
            report_out: Some(report_out.clone()),
            report_version: "v2".to_string(),
            write_markdown: false,
            markdown_out: Some(root.join("comment.md")),
            write_junit: false,
            junit_out: None,
            write_jsonl: false,
            jsonl_out: None,
            mode: RunMode::Standard,
        };

        cmd_check(&cli, opts).expect("cmd_check");

        let report_text = std::fs::read_to_string(report_out).expect("read report");
        let report = parse_report_json(&report_text).expect("parse report");
        let ReportVariant::V2(report) = report else {
            panic!("expected v2 report");
        };
        assert!(report.findings.is_empty(), "expected suppressed findings");
        assert_eq!(report.verdict.status, depguard_types::VerdictStatus::Pass);
        assert_eq!(report.verdict.counts.suppressed, 1);
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
            diff_file: None,
            yanked_index: None,
            yanked_live: false,
            yanked_api_base_url: None,
            incremental: false,
            cache_dir: None,
            baseline: None,
            out_dir: None,
            report_out: Some(report_out),
            report_version: "v2".to_string(),
            write_markdown: false,
            markdown_out: Some(root.join("comment.md")),
            write_junit: false,
            junit_out: None,
            write_jsonl: false,
            jsonl_out: None,
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
            diff_file: None,
            yanked_index: None,
            yanked_live: false,
            yanked_api_base_url: None,
            incremental: false,
            cache_dir: None,
            baseline: None,
            out_dir: None,
            report_out: Some(report_out.clone()),
            report_version: "v2".to_string(),
            write_markdown: false,
            markdown_out: Some(root.join("comment.md")),
            write_junit: false,
            junit_out: None,
            write_jsonl: false,
            jsonl_out: None,
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
            diff_file: None,
            yanked_index: None,
            yanked_live: false,
            yanked_api_base_url: None,
            incremental: false,
            cache_dir: None,
            baseline: None,
            out_dir: None,
            report_out: Some(report_out.clone()),
            report_version: "v2".to_string(),
            write_markdown: false,
            markdown_out: Some(root.join("comment.md")),
            write_junit: false,
            junit_out: None,
            write_jsonl: false,
            jsonl_out: None,
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
    fn cmd_sarif_writes_output_file() {
        let tmp = TempDir::new().expect("temp dir");
        let root = Utf8PathBuf::from_path_buf(tmp.path().to_path_buf()).expect("utf8 path");

        let report = empty_report(ReportVersion::V2, "repo", "strict");
        let data = serialize_report(&report).expect("serialize report");
        let report_path = root.join("report.json");
        std::fs::write(&report_path, data).expect("write report");

        let output_path = root.join("report.sarif");
        cmd_sarif(report_path, Some(output_path.clone())).expect("cmd_sarif");
        assert!(output_path.exists());

        let sarif_text = std::fs::read_to_string(output_path).expect("read sarif");
        assert!(sarif_text.contains("\"version\": \"2.1.0\""));
    }

    #[test]
    fn cmd_junit_writes_output_file() {
        let tmp = TempDir::new().expect("temp dir");
        let root = Utf8PathBuf::from_path_buf(tmp.path().to_path_buf()).expect("utf8 path");

        let report = empty_report(ReportVersion::V2, "repo", "strict");
        let data = serialize_report(&report).expect("serialize report");
        let report_path = root.join("report.json");
        std::fs::write(&report_path, data).expect("write report");

        let output_path = root.join("report.junit.xml");
        cmd_junit(report_path, Some(output_path.clone())).expect("cmd_junit");
        assert!(output_path.exists());

        let junit_text = std::fs::read_to_string(output_path).expect("read junit");
        assert!(junit_text.contains("<testsuite"));
    }

    #[test]
    fn cmd_jsonl_writes_output_file() {
        let tmp = TempDir::new().expect("temp dir");
        let root = Utf8PathBuf::from_path_buf(tmp.path().to_path_buf()).expect("utf8 path");

        let report = empty_report(ReportVersion::V2, "repo", "strict");
        let data = serialize_report(&report).expect("serialize report");
        let report_path = root.join("report.json");
        std::fs::write(&report_path, data).expect("write report");

        let output_path = root.join("report.jsonl");
        cmd_jsonl(report_path, Some(output_path.clone())).expect("cmd_jsonl");
        assert!(output_path.exists());

        let jsonl_text = std::fs::read_to_string(output_path).expect("read jsonl");
        assert!(jsonl_text.contains("\"kind\":\"summary\""));
    }

    #[test]
    fn cmd_fix_writes_buildfix_plan_without_applying() {
        let tmp = TempDir::new().expect("temp dir");
        let root = Utf8PathBuf::from_path_buf(tmp.path().to_path_buf()).expect("utf8 path");

        std::fs::write(
            root.join("Cargo.toml"),
            r#"[package]
name = "demo"
version = "0.1.0"

[dependencies]
serde = { version = "1.0", optional = true }
"#,
        )
        .expect("write manifest");

        let mut report_variant = empty_report(ReportVersion::V2, "repo", "strict");
        let ReportVariant::V2(ref mut report) = report_variant else {
            panic!("expected v2 report")
        };
        report.findings.push(depguard_types::FindingV2 {
            severity: depguard_types::SeverityV2::Warn,
            check_id: depguard_types::ids::CHECK_DEPS_DEFAULT_FEATURES_EXPLICIT.to_string(),
            code: depguard_types::ids::CODE_DEFAULT_FEATURES_IMPLICIT.to_string(),
            message: "missing default-features".to_string(),
            location: Some(depguard_types::Location {
                path: RepoPath::new("Cargo.toml"),
                line: Some(6),
                col: None,
            }),
            help: None,
            url: None,
            fingerprint: Some("fp-default-features".to_string()),
            data: serde_json::json!({
                "dependency": "serde",
                "manifest": "Cargo.toml",
                "section": "dependencies",
                "fix_action": depguard_types::ids::FIX_ACTION_ADD_DEFAULT_FEATURES,
            }),
        });

        let report_path = root.join("report.json");
        write_report_file(&report_path, &report_variant).expect("write report");
        let plan_path = root.join("artifacts").join("buildfix").join("plan.json");

        cmd_fix(&root, report_path, plan_path.clone(), false).expect("cmd_fix");
        assert!(plan_path.exists());

        let plan_text = std::fs::read_to_string(&plan_path).expect("read plan");
        assert!(plan_text.contains("buildfix.plan.v1"));
        assert!(plan_text.contains("default-features = true"));
        let plan_value: serde_json::Value = serde_json::from_str(&plan_text).expect("parse plan");

        let manifest_dir = Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let repo_root = manifest_dir
            .parent()
            .and_then(|p| p.parent())
            .expect("repo root");
        let schema_text =
            std::fs::read_to_string(repo_root.join("contracts/schemas/buildfix.plan.v1.json"))
                .expect("read buildfix schema");
        let schema_value: serde_json::Value =
            serde_json::from_str(&schema_text).expect("parse buildfix schema");
        let compiled = jsonschema::draft7::new(&schema_value).expect("compile buildfix schema");
        let errors: Vec<_> = compiled.iter_errors(&plan_value).collect();
        assert!(
            errors.is_empty(),
            "buildfix plan should validate, errors: {:?}",
            errors
        );

        let manifest = std::fs::read_to_string(root.join("Cargo.toml")).expect("read manifest");
        assert!(!manifest.contains("default-features = true"));
    }

    #[test]
    fn cmd_fix_can_apply_safe_fixes() {
        let tmp = TempDir::new().expect("temp dir");
        let root = Utf8PathBuf::from_path_buf(tmp.path().to_path_buf()).expect("utf8 path");

        std::fs::write(
            root.join("Cargo.toml"),
            r#"[package]
name = "demo"
version = "0.1.0"

[dependencies]
serde = { version = "1.0", optional = true }
"#,
        )
        .expect("write manifest");

        let mut report_variant = empty_report(ReportVersion::V2, "repo", "strict");
        let ReportVariant::V2(ref mut report) = report_variant else {
            panic!("expected v2 report")
        };
        report.findings.push(depguard_types::FindingV2 {
            severity: depguard_types::SeverityV2::Warn,
            check_id: depguard_types::ids::CHECK_DEPS_DEFAULT_FEATURES_EXPLICIT.to_string(),
            code: depguard_types::ids::CODE_DEFAULT_FEATURES_IMPLICIT.to_string(),
            message: "missing default-features".to_string(),
            location: Some(depguard_types::Location {
                path: RepoPath::new("Cargo.toml"),
                line: Some(6),
                col: None,
            }),
            help: None,
            url: None,
            fingerprint: Some("fp-default-features".to_string()),
            data: serde_json::json!({
                "dependency": "serde",
                "manifest": "Cargo.toml",
                "section": "dependencies",
                "fix_action": depguard_types::ids::FIX_ACTION_ADD_DEFAULT_FEATURES,
            }),
        });

        let report_path = root.join("report.json");
        write_report_file(&report_path, &report_variant).expect("write report");
        let plan_path = root.join("artifacts").join("buildfix").join("plan.json");

        cmd_fix(&root, report_path, plan_path, true).expect("cmd_fix");

        let manifest = std::fs::read_to_string(root.join("Cargo.toml")).expect("read manifest");
        assert!(manifest.contains("default-features = true"));
    }

    #[test]
    fn resolve_output_paths_uses_out_dir_defaults() {
        let opts = CheckOpts {
            base: None,
            head: None,
            diff_file: None,
            yanked_index: None,
            yanked_live: false,
            yanked_api_base_url: None,
            incremental: false,
            cache_dir: None,
            baseline: None,
            out_dir: Some(Utf8PathBuf::from("custom-artifacts")),
            report_out: None,
            report_version: "v2".to_string(),
            write_markdown: false,
            markdown_out: None,
            write_junit: false,
            junit_out: None,
            write_jsonl: false,
            jsonl_out: None,
            mode: RunMode::Standard,
        };

        let paths = resolve_output_paths(&opts);
        assert_eq!(
            paths.report_out,
            Utf8PathBuf::from("custom-artifacts/report.json")
        );
        assert_eq!(
            paths.markdown_out,
            Utf8PathBuf::from("custom-artifacts/comment.md")
        );
        assert_eq!(
            paths.junit_out,
            Utf8PathBuf::from("custom-artifacts/report.junit.xml")
        );
        assert_eq!(
            paths.jsonl_out,
            Utf8PathBuf::from("custom-artifacts/report.jsonl")
        );
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
                diff_file: None,
                yanked_index: None,
                yanked_live: false,
                yanked_api_base_url: None,
                incremental: false,
                cache_dir: None,
                baseline: None,
                out_dir: None,
                report_out: Some(Utf8PathBuf::from("report.json")),
                report_version: "v2".to_string(),
                write_markdown: false,
                markdown_out: Some(Utf8PathBuf::from("comment.md")),
                write_junit: false,
                junit_out: None,
                write_jsonl: false,
                jsonl_out: None,
                mode: RunMode::Standard,
            },
        };

        let report_out = Utf8PathBuf::from_path_buf(tmp.path().join("out").join("report.json"))
            .expect("utf8 report path");
        let opts = CheckOpts {
            base: None,
            head: None,
            diff_file: None,
            yanked_index: None,
            yanked_live: false,
            yanked_api_base_url: None,
            incremental: false,
            cache_dir: None,
            baseline: None,
            out_dir: None,
            report_out: Some(report_out.clone()),
            report_version: "v2".to_string(),
            write_markdown: false,
            markdown_out: Some(Utf8PathBuf::from("comment.md")),
            write_junit: false,
            junit_out: None,
            write_jsonl: false,
            jsonl_out: None,
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
