# BDD Feature Files

This directory contains Gherkin `.feature` files that document depguard's behavior
in human-readable format.

## Feature Files

| File                    | Description                                     |
|-------------------------|-------------------------------------------------|
| `depguard.feature`      | Core scenarios: checks, output, errors          |
| `checks.feature`        | Per-check detection logic with examples         |
| `configuration.feature` | Profiles, config overrides, allowlists          |
| `workspaces.feature`    | Workspace discovery and member handling         |
| `diff_scope.feature`    | Git diff-scoped analysis for PRs                |
| `roadmap.feature`       | Baseline, yanked index, outputs, and fix flows  |
| `determinism.feature`   | Byte-stable, reproducible output guarantees     |
| `ci_integration.feature`| GitHub Actions annotations, markdown, exit codes|

## Purpose

These files serve as:

1. **Living documentation** — Understand what depguard does without reading code
2. **Test specifications** — Define expected behavior for implementation
3. **Regression anchors** — Ensure behavior doesn't change unexpectedly

## Implementation Status

| Scenario Type              | Implementation               |
|----------------------------|------------------------------|
| Core check detection       | ✓ `fixtures.rs` + fixtures   |
| Output rendering           | ✓ `fixtures.rs`              |
| Configuration/profiles     | ✓ `bdd.rs` steps + fixtures  |
| Workspace handling         | ✓ `bdd.rs` steps + fixtures  |
| Diff scope                 | ✓ git + `--diff-file` modes  |
| Baseline/fix/extra outputs | ✓ `roadmap.feature`          |
| Determinism validation     | ✓ `fixtures.rs` + proptest   |
| CI integration             | ✓ `fixtures.rs`              |

## Test Fixtures

Each scenario backed by a fixture has a corresponding directory in `tests/fixtures/`:

```
tests/fixtures/
├── clean/                       # Passing workspace
│   ├── Cargo.toml
│   └── expected.report.json
├── wildcards/                   # deps.no_wildcards violation
│   ├── Cargo.toml
│   ├── expected.report.json
│   ├── expected.comment.md
│   └── expected.annotations.txt
├── path_missing_version/        # deps.path_requires_version violation
│   ├── Cargo.toml
│   └── expected.report.json
├── path_safety/                 # deps.path_safety violations
│   ├── Cargo.toml
│   └── expected.report.json
├── workspace_inheritance/       # deps.workspace_inheritance violation
│   ├── Cargo.toml
│   ├── member-crate/
│   │   └── Cargo.toml
│   └── expected.report.json
├── workspace_members_exclude/   # workspace members + exclude
│   └── expected.report.json
├── nested_workspace/            # nested workspace exclusion
│   └── expected.report.json
└── target_deps/                 # target-specific deps
    └── expected.report.json
```

## Running Scenario Tests

```bash
# All integration tests
cargo test --test '*'

# Specific fixture
cargo test fixture_wildcards_fails

# Update golden fixtures after intentional changes
# (Run depguard against each fixture and update expected.* files)
```

## Adding New Scenarios

1. **Document** — Add scenario to appropriate `.feature` file
2. **Fixture** — Create `tests/fixtures/<name>/Cargo.toml`
3. **Golden** — Run `cargo xtask fixtures` to generate expected output
4. **Test** — Add test function in `crates/depguard-cli/tests/fixtures.rs`:

```rust
#[test]
fn fixture_<name>_<passes_or_fails>() {
    let (exit_code, report) = run_check_on_fixture("<name>");
    let expected = load_expected_report("<name>");

    assert_eq!(exit_code, <0_or_2>, "<name> fixture should exit with <code>");
    assert_reports_match(report, expected, "<name>");
}
```

## Gherkin Conventions

- **Given** — Set up the test context (fixture, config)
- **When** — Execute the action (run depguard command)
- **Then** — Assert the outcome (exit code, findings, output)
- **And** — Additional assertions or setup in same step type

Example:
```gherkin
Scenario: Wildcard versions are flagged
  Given a workspace fixture "wildcards"
  When I run "depguard check --repo-root ."
  Then the exit code is 2
  And the receipt has a finding with:
    | check_id | deps.no_wildcards |
    | code     | wildcard_version  |
```
