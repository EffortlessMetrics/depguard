# CLAUDE.md — depguard-app

## Purpose

Use case orchestration layer. Thin application layer that coordinates domain, repo, settings, and render crates. Includes check execution, baseline generation, buildfix plans, and report rendering.

## Key Modules

| Module | Contents |
|--------|----------|
| [`check.rs`] | `run_check()` — primary analysis use case |
| [`baseline.rs`] | Baseline suppression generation from findings |
| [`fix.rs`] | Buildfix plan generation and safe fix application |
| [`render.rs`] | `run_markdown()`, `run_annotations()`, renderer coordination |
| [`explain.rs`] | `run_explain()` — lookup check/code guidance |
| [`report.rs`] | Report construction and serialization |

## Public API

```rust
// Run analysis
pub fn run_check(input: CheckInput) -> Result<CheckOutput>

// Render from existing report
pub fn run_markdown(report: &RenderableReport) -> String
pub fn run_annotations(report: &RenderableReport) -> Vec<String>
pub fn run_sarif(report: &RenderableReport) -> String
pub fn run_junit(report: &RenderableReport) -> String
pub fn run_jsonl(report: &RenderableReport) -> String

// Serialize report to JSON
pub fn serialize_report(report: &DepguardReport) -> Result<String>

// Lookup explanation
pub fn run_explain(identifier: &str) -> Option<Explanation>

// Baseline generation
pub fn generate_baseline(report: &DepguardReport) -> BaselineV1

// Buildfix plan generation and safe auto-fix
pub fn generate_buildfix_plan(report: &ReportVariant, report_path: &str, dry_run: bool) -> BuildfixPlanV1
pub fn apply_safe_fixes(repo_root: &Utf8Path, report: &ReportVariant) -> FixApplyResult

// Map verdict to exit code
pub fn verdict_exit_code(verdict: Verdict) -> i32
```

## Check Use Case Flow

```
CheckInput { repo_root, config_text, overrides, changed_files }
    → parse_config_toml()
    → resolve_config()
    → build_workspace_model()
    → evaluate()
    → wrap in DepguardReport with timestamps
    → CheckOutput { report, resolved_config }
```

## Exit Code Mapping

| Verdict | Exit Code |
|---------|-----------|
| Pass | 0 |
| Warn | 0 |
| Fail | 2 |

Tool/runtime errors use exit code 1.

## Feature Gates

This crate propagates check feature gates to domain and settings:

```toml
check-no-wildcards = [
    "depguard-domain/check-no-wildcards",
    "depguard-settings/check-no-wildcards",
]
```

All 10 checks have corresponding features.

## Design Constraints

- **Minimal glue code**: Most logic delegated to domain/repo/settings/render
- **No clap**: CLI argument handling belongs in `depguard-cli`
- **Error handling**: Uses anyhow with context for actionable messages
- **Deterministic**: Same inputs → same outputs

## Dependencies

| Dependency | Purpose |
|------------|---------|
| `depguard-types` | DTOs, IDs, explanations |
| `depguard-domain` | Policy evaluation engine |
| `depguard-repo` | Workspace discovery, model building |
| `depguard-settings` | Config parsing, profile resolution |
| `depguard-render` | Output formatters |
| `depguard-yanked` | Yanked version index parsing |
| `anyhow` | Error handling |
| `camino` | UTF-8 paths |
| `serde_json` | Report serialization |
| `time` | Timestamps |
| `toml_edit` | Manifest editing for fixes |

## Testing

```bash
cargo test -p depguard-app
```

Integration tests validate end-to-end use case behavior using fixtures in `tests/fixtures/`.
