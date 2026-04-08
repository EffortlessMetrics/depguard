# depguard-render

## Problem
The same findings are useful in many places (PR comments, GitHub annotations, CI parsers), but each output format has its own schema and conventions.

## What this crate does
`depguard-render` provides deterministic, format-specific renderers that transform report envelopes into downstream artifacts.

## Outputs covered
- Markdown (`md`)
- GitHub annotations (`annotations`)
- SARIF (`sarif`)
- JUnit (`junit`)
- JSONL (`jsonl`)

## How to use
- Consume validated report envelopes from `depguard-types`.
- Choose a renderer by command context (CI, PR, local inspection).
- Expect deterministic text ordering and stable serializations.

## Design constraints
- No policy evaluation logic.
- No direct filesystem behavior by default; renderers return structured output to be written by caller.

## Quality rules
- Output ordering must stay stable for identical input.
- Rendering should be reproducible across platforms when run with same locale and Rust version.

## Related crates
- `depguard-app`
- `depguard-cli`
- `depguard-types`
