# xtask

Developer automation commands for the depguard workspace.

This binary crate is for contributor workflows and CI validation, not end-user policy checks.

## Common Commands

- `cargo xtask emit-schemas`
- `cargo xtask validate-schemas`
- `cargo xtask fixtures`
- `cargo xtask print-schema-ids`
- `cargo xtask conform`
- `cargo xtask conform-full`
- `cargo xtask explain-coverage`

## Scope

- Schema generation from Rust types
- Fixture regeneration/validation workflows
- Contract and conformance checks

This crate is `publish = false`.
