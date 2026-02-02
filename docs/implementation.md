# Implementation notes

This is a suggested sequencing that minimizes rework:

1. **Types + schema IDs** (`depguard-types`)
   - lock down the envelope/report shape early
   - add ordering helpers + fingerprint strategy

2. **Config model + presets** (`depguard-settings`)
   - parse TOML
   - merge with CLI overrides
   - produce `EffectiveConfig`

3. **Domain checks** (`depguard-domain`)
   - implement checks against an in-memory `WorkspaceModel`
   - unit tests + property tests

4. **Repo adapter** (`depguard-repo`)
   - workspace discovery
   - manifest parse into domain model
   - fixtures + fuzz harnesses

5. **Renderers** (`depguard-render`)
   - Markdown + GitHub annotations
   - golden snapshot tests

6. **CLI glue** (`depguard-cli`)
   - wire everything
   - artifacts + exit codes
   - integration tests

7. **xtask**
   - schema generation (optional)
   - fixture update automation
   - release packaging
