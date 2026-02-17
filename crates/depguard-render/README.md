# depguard-render

Deterministic renderers for depguard report outputs.

This crate converts `RenderableReport` data into text formats used by CI systems and developer workflows.

## Output Formats

- Markdown (`render_markdown`)
- GitHub Actions annotations (`render_github_annotations`)
- SARIF (`render_sarif`)
- JUnit XML (`render_junit`)
- JSON Lines (`render_jsonl`)

## Design Constraints

- Pure rendering only (no file writes, no subprocesses)
- Stable output for identical input
- Keep escaping/output rules CI-safe and explicit
