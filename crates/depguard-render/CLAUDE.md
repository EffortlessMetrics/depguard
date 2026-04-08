# CLAUDE.md — depguard-render

## Purpose

Output formatters for CI surfaces. Renders reports to Markdown, GitHub Actions annotations, SARIF, JUnit XML, and JSON Lines.

## Key Modules

| Module | Contents |
|--------|----------|
| [`markdown.rs`] | `render_markdown()` — human-readable Markdown |
| [`gha.rs`] | `render_github_annotations()` — GHA workflow commands |
| [`sarif.rs`] | `render_sarif()` — SARIF format for security tools |
| [`junit.rs`] | `render_junit()` — JUnit XML for test runners |
| [`jsonl.rs`] | `render_jsonl()` — JSON Lines for log aggregation |
| [`model.rs`] | Renderable view models (`RenderableReport`, etc.) |

## Public API

```rust
// Render report as Markdown
pub fn render_markdown(report: &RenderableReport) -> String

// Render report as GitHub Actions annotation lines
pub fn render_github_annotations(report: &RenderableReport) -> Vec<String>

// Render report as SARIF
pub fn render_sarif(report: &RenderableReport) -> String

// Render report as JUnit XML
pub fn render_junit(report: &RenderableReport) -> String

// Render report as JSON Lines
pub fn render_jsonl(report: &RenderableReport) -> String

// View models for rendering
pub struct RenderableReport { ... }
pub struct RenderableFinding { ... }
pub struct RenderableLocation { ... }
pub enum RenderableVerdictStatus { ... }
```

## Markdown Output

Includes:
- Verdict badge (✅ Pass, ⚠️ Warn, ❌ Fail)
- Finding counts by severity
- Per-finding details: severity, check_id, code, message, location
- Truncation notice if findings were limited

## GitHub Actions Format

Each finding becomes a workflow command:

```
::error file=path/to/Cargo.toml,line=15,col=1::message text here
::warning file=path/to/Cargo.toml,line=20::another message
```

Special characters are escaped:
- `%` → `%25`
- `\r` → `%0D`
- `\n` → `%0A`

## SARIF Format

Produces SARIF 2.1.0 compatible output for:
- GitHub Advanced Security
- Azure DevOps
- Other security analysis tools

## JUnit Format

Standard JUnit XML for CI test reporting:
- Each manifest as a test suite
- Each finding as a test case (failure)

## Design Constraints

- **No I/O**: Functions take data, return strings
- **Deterministic**: Same input → same output
- **One finding per line** for GHA (enables proper annotation display)
- **Byte-stable**: Output should be identical across runs

## Dependencies

| Dependency | Purpose |
|------------|---------|
| `depguard-types` | `DepguardReport`, `Finding`, `Severity` |
| `serde_json` | JSON serialization for SARIF/JSONL |

Dev dependencies:
- `insta` — Snapshot testing
- `time` — Timestamp fixtures

## Testing

```bash
cargo test -p depguard-render
```

Uses insta for snapshot testing of rendered output. Golden files are stored in `snapshots/` directory.

## Adding a New Renderer

1. Create new module file (e.g., `newformat.rs`)
2. Implement `render_newformat(report: &RenderableReport) -> String`
3. Add module and export in `lib.rs`
4. Add CLI subcommand in `depguard-cli`
5. Add tests with insta snapshots
