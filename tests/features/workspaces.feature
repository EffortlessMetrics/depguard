Feature: Workspace handling

  depguard correctly discovers and analyzes Cargo workspaces.

  # ===========================================================================
  # Workspace discovery
  # ===========================================================================

  Scenario: Single-crate repository is analyzed
    Given a repository with a single Cargo.toml (no workspace)
    When I run "depguard check --repo-root ."
    Then the manifest is analyzed
    And findings reference the root Cargo.toml

  Scenario: Workspace with members is fully analyzed
    Given a workspace with members: "crate-a", "crate-b", "crate-c"
    When I run "depguard check --repo-root ."
    Then all member manifests are analyzed
    And findings may reference any member path

  Scenario: Virtual workspace (no root package) is analyzed
    Given a virtual workspace Cargo.toml:
      """
      [workspace]
      members = ["crates/*"]
      """
    When I run "depguard check --repo-root ."
    Then all member manifests are analyzed

  Scenario: Nested workspaces are handled correctly
    Given a workspace with a nested workspace in "tools/"
    When I run "depguard check --repo-root ."
    Then only the top-level workspace is analyzed
    And nested workspace members are excluded

  # ===========================================================================
  # Path resolution
  # ===========================================================================

  Scenario: Findings include relative paths from repo root
    Given a workspace fixture "workspace_inheritance"
    When I run "depguard check --repo-root ."
    Then finding paths are relative to repo root
    And paths use forward slashes (portable)

  Scenario: Line numbers point to the dependency declaration
    Given a workspace fixture "wildcards"
    When I run "depguard check --repo-root ."
    Then finding line numbers point to the dependency line in Cargo.toml

  # ===========================================================================
  # Glob patterns in workspace members
  # ===========================================================================

  Scenario: Glob patterns in members are expanded
    Given a workspace Cargo.toml with:
      """
      [workspace]
      members = ["crates/*", "tools/*"]
      """
    And directories: "crates/a", "crates/b", "tools/x"
    When I run "depguard check --repo-root ."
    Then all matched directories are analyzed

  Scenario: Excluded members are skipped
    Given a workspace fixture "workspace_members_exclude"
    When I run "depguard check --repo-root ."
    Then "crates/legacy" is not analyzed
