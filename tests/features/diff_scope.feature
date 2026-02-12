Feature: Diff-scoped analysis

  depguard can limit analysis to files changed between git refs.

  Background:
    Given a git repository with history

  # ===========================================================================
  # Basic diff scope
  # ===========================================================================

  Scenario: Diff scope only analyzes changed files
    Given the following files changed between base and head:
      | file                      |
      | crates/changed/Cargo.toml |
    And "crates/unchanged/Cargo.toml" has violations
    When I run "depguard check --scope diff --base main --head HEAD"
    Then only "crates/changed/Cargo.toml" is analyzed
    And no findings are reported for unchanged files

  Scenario: Full repo scope analyzes all files
    Given the following files changed between base and head:
      | file                      |
      | crates/changed/Cargo.toml |
    And "crates/unchanged/Cargo.toml" has violations
    When I run "depguard check --scope repo"
    Then all Cargo.toml files are analyzed
    And findings include violations from unchanged files

  # ===========================================================================
  # Git integration
  # ===========================================================================

  Scenario: Diff scope with branch names
    Given branches "main" and "feature/add-deps"
    When I run "depguard check --scope diff --base main --head feature/add-deps"
    Then the exit code is 0 or 2
    And the receipt shows scope "diff"

  Scenario: Diff scope with commit SHAs
    Given commits "abc1234" and "def5678"
    When I run "depguard check --scope diff --base abc1234 --head def5678"
    Then the exit code is 0 or 2

  Scenario: Missing git returns error
    Given a directory without git initialization
    When I run "depguard check --scope diff --base main --head HEAD"
    Then the exit code is 1
    And stderr mentions git is required

  # ===========================================================================
  # PR workflow scenarios
  # ===========================================================================

  Scenario: New crate added in PR is analyzed
    Given a PR that adds "crates/new-service/Cargo.toml"
    And the new Cargo.toml has a wildcard dependency
    When I run "depguard check --scope diff --base main --head HEAD"
    Then a finding is reported for the new crate

  Scenario: Modified existing crate is analyzed
    Given a PR that modifies "crates/existing/Cargo.toml"
    And the modification adds a path dependency without version
    When I run "depguard check --scope diff --base main --head HEAD"
    Then a finding is reported for the modification

  Scenario: Deleted crate is not analyzed
    Given a PR that deletes "crates/old-crate/Cargo.toml"
    When I run "depguard check --scope diff --base main --head HEAD"
    Then no findings are reported for the deleted crate
