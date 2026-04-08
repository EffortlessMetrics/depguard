# depguard-render

Deterministic renderers for depguard report outputs.

This crate converts `RenderableReport` data into text formats used by CI systems and developer workflows. All renderers are pure functions with no I/O side effects.

## Purpose

The render crate provides:
- Multiple output format support for CI/CD integration
- Deterministic, stable output for identical inputs
- CI-safe escaping and formatting rules
- No filesystem or subprocess dependencies

## Output Formats

| Format | Function | Use Case |
|--------|----------|----------|
| Markdown | `render_markdown` | Human-readable reports, documentation |
| GitHub Actions | `render_github_annotations` | CI annotations in GitHub Actions |
| SARIF | `render_sarif` | Security tools, GitHub code scanning |
| JUnit XML | `render_junit` | Test runners, CI dashboards |
| JSON Lines | `render_jsonl` | Log aggregation, streaming processing |

## Public API

```rust
use depguard_render::{
    render_markdown,
    render_github_annotations,
    render_sarif,
    render_junit,
    render_jsonl,
    RenderableReport,
};

/// Render report as Markdown
pub fn render_markdown(report: &RenderableReport) -> String;

/// Render report as GitHub Actions annotations
pub fn render_github_annotations(report: &RenderableReport) -> String;

/// Render report as SARIF (Static Analysis Results Interchange Format)
pub fn render_sarif(report: &RenderableReport) -> String;

/// Render report as JUnit XML
pub fn render_junit(report: &RenderableReport) -> String;

/// Render report as JSON Lines (one JSON object per line)
pub fn render_jsonl(report: &RenderableReport) -> String;
```

## Usage Example

```rust
use depguard_render::{render_markdown, render_sarif, RenderableReport};

// Build a renderable report (typically from depguard-app)
let report: RenderableReport = /* ... */;

// Render as Markdown
let markdown = render_markdown(&report);
std::fs::write("DEPENDENCY_REPORT.md", &markdown)?;

// Render as SARIF for GitHub code scanning
let sarif = render_sarif(&report);
std::fs::write("results.sarif.json", &sarif)?;
```

## Renderable Report Model

The `RenderableReport` struct provides a view of the report suitable for rendering:

```rust
pub struct RenderableReport {
    pub schema: String,
    pub tool: ToolMeta,
    pub run: RunMeta,
    pub verdict: RenderableVerdictStatus,
    pub findings: Vec<RenderableFinding>,
}

pub struct RenderableFinding {
    pub check_id: String,
    pub code: String,
    pub severity: RenderableSeverity,
    pub message: String,
    pub location: RenderableLocation,
    pub data: RenderableData,
}

pub struct RenderableLocation {
    pub path: String,
    pub line: Option<u32>,
    pub column: Option<u32>,
}
```

## Output Examples

### Markdown

```markdown
# Dependency Report

**Verdict**: FAIL

## Findings

### Error: Wildcard Version (`deps.no_wildcards`)

- **Location**: `Cargo.toml:15`
- **Message**: Dependency `serde` uses wildcard version `*`

### Warning: Path Without Version (`deps.path_requires_version`)

- **Location**: `crates/my-crate/Cargo.toml:8`
- **Message**: Path dependency `my-other-crate` should declare a version
```

### GitHub Actions

```
::error file=Cargo.toml,line=15::Wildcard version: Dependency `serde` uses wildcard version `*` [deps.no_wildcards]
::warning file=crates/my-crate/Cargo.toml,line=8::Path without version: Path dependency `my-other-crate` should declare a version [deps.path_requires_version]
```

## Design Constraints

- **Pure rendering only**: No file writes, no subprocesses
- **Stable output**: Identical input produces identical output
- **CI-safe**: Proper escaping for each format
- **No external dependencies**: Only uses `serde_json` for JSON-based formats

## Dependencies

| Crate | Purpose |
|-------|---------|
| `depguard-types` | Report types and severity |
| `serde_json` | JSON serialization for SARIF and JSONL |

## Related Crates

- [`depguard-app`](../depguard-app/) - Uses renderers for output
- [`depguard-types`](../depguard-types/) - Report types
- [`depguard-cli`](../depguard-cli/) - CLI commands that invoke rendering
