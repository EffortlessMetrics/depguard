# Depguard Project Context

## Project Overview

**Depguard** is a repo-truth dependency manifest hygiene sensor for Rust workspaces. It inspects `Cargo.toml` manifests against explicit policies to enforce versioning rules, prevent wildcard dependencies, and ensure workspace hygiene. It is designed to be:
- **Offline & Fast:** No network or `cargo` metadata resolution required.
- **Deterministic:** Byte-identical outputs for stable CI diffs.
- **Schema-First:** Output is strictly typed via versioned JSON schemas.
- **CI-Native:** Optimised for GitHub Actions and other CI pipelines.

## Architecture

The project follows a **Hexagonal (Ports & Adapters) Architecture** to ensure the core logic remains pure and testable.

### Key Crates
- **`depguard-cli`**: The binary entry point. Adapts CLI args to the application layer.
- **`depguard-app`**: Orchestrates use cases (check, explain, report) using the domain.
- **`depguard-domain`**: **PURE LOGIC.** Contains policy evaluation, models, and checks.
  - **Constraint:** MUST NOT perform I/O (no file system, no network, no stdout).
- **`depguard-repo`**: Adapter for file system and git operations (discovery, parsing).
- **`depguard-settings`**: Configuration parsing and profile management (`depguard.toml`).
- **`depguard-render`**: Renders outputs (Markdown reports, console output).
- **`depguard-types`**: Stable DTOs and schema definitions.
- **`xtask`**: Developer tooling for schema generation and verification.

## Building and Running

### Basic Commands
- **Build:** `cargo build`
- **Test:** `cargo test`
- **Run:** `cargo run -p depguard-cli -- <args>`
  - Example: `cargo run -p depguard-cli -- check`
- **Lint:** `cargo clippy --all-targets --all-features`
- **Format:** `cargo fmt`

### Developer Tasks (`xtask`)
Run these via `cargo xtask <command>`.

| Command | Description |
| :--- | :--- |
| `emit-schemas` | Regenerate JSON schemas in `schemas/` from Rust types. |
| `validate-schemas` | Verify `schemas/` matches code (used in CI). |
| `conform` | Validate contract fixtures against `sensor.report.v1` schema. |
| `conform-full` | Validate fixtures + binary output against contracts. |
| `explain-coverage` | Ensure all check IDs and codes have documentation. |
| `print-schema-ids` | List all known schema IDs. |

### Testing Strategy
- **Unit Tests:** Co-located in `src/`.
- **Property Tests:** In `depguard-domain` (using `proptest`).
- **Golden/Snapshot Tests:** In `tests/fixtures`.
  - Uses `insta` and custom golden file comparisons.
  - **Update Snapshots:** `cargo insta review` or set `INSTA_UPDATE=always`.
- **BDD Tests:** `tests/features/*.feature` (using `cucumber`).

## Key Files & Directories

- **`Cargo.toml`**: Workspace definition.
- **`depguard.toml`**: Project configuration (if present in root).
- **`schemas/`**: Versioned JSON schemas (`depguard.report.v1.json`, etc.).
- **`contracts/`**: Testing contracts and schemas for output validation.
- **`docs/`**: Extensive documentation (Architecture, Checks, Config).
- **`CONTRIBUTING.md`**: Detailed contribution guide (Note: `cargo xtask fixtures` reference may be outdated; use `insta` or specific `xtask` commands).

## Development Conventions

1.  **Domain Purity:** Never introduce I/O into `depguard-domain`.
2.  **Schema Stability:** `depguard-types` defines the public contract. Do not break existing schemas.
3.  **Check Implementation:**
    - Add ID to `depguard-types/src/ids.rs`.
    - Add explanation to `depguard-types/src/explain.rs`.
    - Implement logic in `depguard-domain/src/checks/`.
    - Wire up in `depguard-domain/src/checks/mod.rs`.
4.  **Error Handling:** Use `anyhow` for app/CLI layers, specific errors for domain.
