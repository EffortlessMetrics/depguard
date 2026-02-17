# depguard-app

Application-layer use cases for depguard.

This crate orchestrates settings, repository modeling, domain evaluation, baseline handling, and report rendering/parsing.

## Owns

- Primary check flow (`run_check`)
- Baseline parse/generate/apply helpers
- Report parse/serialize/convert helpers for v1, v2, and `sensor.report.v1`
- Explain use case formatting (`run_explain`, `format_explanation`)
- Buildfix plan generation and conservative safe fix application
- Verdict-to-exit-code mapping (`verdict_exit_code`)

## Design Constraints

- Keep orchestration thin; business rules stay in `depguard-domain`
- Keep CLI concerns out (argument parsing belongs in `depguard-cli`)
- Maintain deterministic report production paths
