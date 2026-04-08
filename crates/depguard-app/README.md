# depguard-app

Application-layer use cases for depguard.

This crate orchestrates settings, repository modeling, domain evaluation, baseline handling, and report rendering/parsing. It serves as the application layer in depguard's hexagonal architecture, coordinating between the CLI, domain, and infrastructure layers.

## Purpose

The app crate is the orchestration layer that:
- Coordinates the primary check flow (`run_check`)
- Bridges CLI concerns with domain logic
- Manages report serialization and parsing across multiple schema versions
- Provides the explain use case for remediation guidance

## Key Features

### Use Cases

- **Check**: Primary analysis flow that discovers manifests, evaluates policies, and produces findings
- **Baseline**: Generate, parse, and apply suppression baselines
- **Explain**: Format remediation guidance for check IDs and codes
- **Fix**: Generate buildfix plans and apply conservative safe fixes
- **Render**: Convert reports to various output formats

### Report Handling

- Parse and serialize reports for v1, v2, and `sensor.report.v1` schemas
- Convert between report variants
- Verdict-to-exit-code mapping for CLI consumption

## Public API

```rust
// Primary check flow
pub fn run_check(input: CheckInput) -> anyhow::Result<CheckOutput>;
pub fn verdict_exit_code(verdict: &VerdictStatus) -> i32;

// Baseline handling
pub fn generate_baseline(findings: &[FindingV2]) -> DepguardBaselineV1;
pub fn parse_baseline_json(input: &str) -> anyhow::Result<DepguardBaselineV1>;
pub fn apply_baseline(findings: &mut Vec<FindingV2>, baseline: &DepguardBaselineV1) -> BaselineApplyResult;

// Explain use case
pub fn run_explain(query: &str) -> ExplainOutput;
pub fn format_explanation(explanation: &Explanation) -> String;

// Buildfix
pub fn generate_buildfix_plan(report: &DepsReport) -> BuildfixPlanV1;
pub fn apply_safe_fixes(plan: &BuildfixPlanV1, repo_root: &Utf8Path) -> anyhow::Result<FixApplyResult>;

// Rendering
pub fn render_markdown(report: &RenderableReport) -> String;
pub fn render_annotations(report: &RenderableReport) -> String;
pub fn render_sarif(report: &RenderableReport) -> String;
pub fn render_junit(report: &RenderableReport) -> String;
pub fn render_jsonl(report: &RenderableReport) -> String;
```

## Design Constraints

- Keep orchestration thin; business rules stay in `depguard-domain`
- Keep CLI concerns out (argument parsing belongs in `depguard-cli`)
- Maintain deterministic report production paths
- No direct filesystem access for check logic (delegated to `depguard-repo`)

## Dependencies

| Crate | Purpose |
|-------|---------|
| `depguard-types` | DTOs, IDs, receipt types |
| `depguard-domain` | Policy evaluation engine |
| `depguard-repo` | Workspace discovery and manifest loading |
| `depguard-settings` | Configuration parsing and resolution |
| `depguard-render` | Output format renderers |
| `depguard-yanked` | Yanked version index |

## Feature Flags

All check features are propagated through from domain and settings:
- `check-no-wildcards`
- `check-path-requires-version`
- `check-path-safety`
- `check-workspace-inheritance`
- `check-git-requires-version`
- `check-dev-only-in-normal`
- `check-default-features-explicit`
- `check-no-multiple-versions`
- `check-optional-unused`
- `check-yanked-versions`

## Related Crates

- [`depguard-cli`](../depguard-cli/) - CLI entry point that consumes this crate
- [`depguard-domain`](../depguard-domain/) - Business logic layer
- [`depguard-render`](../depguard-render/) - Output rendering
- [`depguard-settings`](../depguard-settings/) - Configuration resolution
