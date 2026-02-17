# depguard-yanked

Offline yanked-version index parsing and querying for `deps.yanked_versions`.

Input is plain text; output is an in-memory `YankedIndex`. The crate is deterministic and I/O free.

## Supported Input Formats

- JSON map: `{ "serde": ["1.0.188", "1.0.189"] }`
- JSON array: `[{"crate":"serde","version":"1.0.188"}]`
- Line format:
  - `serde 1.0.188`
  - `tokio@1.37.0`
  - `# comments` are ignored

## Public API

- `parse_yanked_index(input: &str) -> anyhow::Result<YankedIndex>`
- `YankedIndex::is_yanked(crate_name: &str, version: &str) -> bool`

## Design Constraints

- No filesystem access
- No network access
- Stable, deterministic parsing behavior
