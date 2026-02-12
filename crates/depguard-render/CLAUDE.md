# CLAUDE.md — depguard-render

## Purpose

Output formatters for CI surfaces. Renders reports to Markdown and GitHub Actions annotations.

## Key Modules

| Module | Contents |
|--------|----------|
| `markdown.rs` | `render_markdown()` — human-readable Markdown |
| `gha.rs` | `render_github_annotations()` — GHA workflow commands |

## Public API

```rust
// Render report as Markdown
pub fn render_markdown(report: &DepguardReport) -> String

// Render report as GitHub Actions annotation lines
pub fn render_github_annotations(report: &DepguardReport) -> Vec<String>
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

## Design Constraints

- **No I/O**: Functions take data, return strings
- **Deterministic**: Same input → same output
- **One finding per line** for GHA (enables proper annotation display)

## Dependencies

- `depguard-types` — `DepguardReport`, `Finding`, `Severity`

Dev dependencies: `insta`, `time` for snapshot testing

## Testing

```bash
cargo test -p depguard-render
```

Uses insta for snapshot testing of rendered output.
