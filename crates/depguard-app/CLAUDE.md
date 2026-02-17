# CLAUDE.md — depguard-app

## Purpose

Use case orchestration layer. Thin application layer that coordinates domain, repo, settings, and render crates, and now includes conservative safe-fix application for buildfix plans.

## Key Modules

| Module | Contents |
|--------|----------|
| `check.rs` | `run_check()` — primary analysis use case |
| `fix.rs` | buildfix plan generation + conservative safe fix application |
| `render.rs` | `run_markdown()`, `run_annotations()`, `serialize_report()` |
| `explain.rs` | `run_explain()` — lookup check/code guidance |

## Public API

```rust
// Run analysis
pub fn run_check(input: CheckInput) -> Result<CheckOutput>

// Render from existing report
pub fn run_markdown(report: &DepguardReport) -> String
pub fn run_annotations(report: &DepguardReport) -> Vec<String>

// Serialize report to JSON
pub fn serialize_report(report: &DepguardReport) -> Result<String>

// Lookup explanation
pub fn run_explain(identifier: &str) -> Option<Explanation>

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

## Design Constraints

- **Minimal glue code**: Most logic delegated to domain/repo/settings/render; fix application remains conservative and explicit
- **No clap**: CLI argument handling belongs in `depguard-cli`
- **Error handling**: Uses anyhow with context for actionable messages

## Dependencies

- All internal crates: `depguard-types`, `depguard-domain`, `depguard-repo`, `depguard-settings`, `depguard-render`
- `anyhow` — Error handling
- `camino` — UTF-8 paths
- `serde_json` — Report serialization
- `time` — Timestamps

## Testing

```bash
cargo test -p depguard-app
```

Integration tests validate end-to-end use case behavior.
