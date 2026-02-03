# BDD Scenarios

This directory contains supplementary `.feature` files in Gherkin format.

The primary feature files are in `tests/features/`. This directory holds additional
lower-level scenarios that may be promoted to the main features directory once validated.

## Feature File Structure

Main scenarios (`tests/features/`):

| File                   | Coverage                                      |
|------------------------|-----------------------------------------------|
| `depguard.feature`     | Core checks, output rendering, error handling |
| `checks.feature`       | Detailed per-check detection logic            |
| `configuration.feature`| Profiles, config files, allowlists            |
| `workspaces.feature`   | Workspace discovery and path resolution       |
| `diff_scope.feature`   | Git diff-scoped analysis                      |
| `determinism.feature`  | Byte-stable, reproducible output              |
| `ci_integration.feature`| GitHub Actions, markdown, exit codes         |

## Implementation Approach

The project uses a **hybrid approach**:

1. **Feature files** document behavior in human-readable Gherkin format
2. **Rust integration tests** (`crates/depguard-cli/tests/fixtures.rs`) execute the scenarios
3. **Golden fixtures** (`tests/fixtures/`) provide expected outputs for regression testing

This trades a Gherkin interpreter for simpler Rust-native tests while keeping policy
semantics readable by non-developers.

## Running Tests

```bash
# Run all integration tests (which validate feature scenarios)
cargo test --test '*'

# Run fixture tests specifically
cargo test -p depguard-cli fixture_

# Regenerate golden fixtures after intentional changes
cargo xtask fixtures
```

## Adding New Scenarios

1. Write the scenario in the appropriate `.feature` file
2. Create a fixture directory in `tests/fixtures/<name>/`
3. Add `Cargo.toml` with the test case
4. Run `cargo xtask fixtures` to generate `expected.report.json`
5. Add a test function in `fixtures.rs` to validate the scenario
