Feature: deps.workspace_inheritance rule

  The deps.workspace_inheritance check ensures that when a dependency is defined in
  [workspace.dependencies], member crates use `workspace = true` to inherit it rather
  than duplicating the version specification. This prevents version drift and ensures
  consistent dependency versions across the workspace.

  Background:
    Given the deps.workspace_inheritance check is enabled

  # ===========================================================================
  # Detection of missing workspace inheritance
  # ===========================================================================

  @detection @fail
  Scenario: Member not using workspace inheritance is flagged
    Given a workspace Cargo.toml with:
      """
      [workspace.dependencies]
      serde = "1.0"
      """
    And a member Cargo.toml with:
      """
      [dependencies]
      serde = "1.0"
      """
    When I run the check
    Then a finding is emitted with check_id "deps.workspace_inheritance" and code "missing_workspace_true"
    And the finding message mentions "serde"
    And the finding message mentions "workspace = true"

  @detection @fail
  Scenario: Member using different version than workspace is flagged
    Given a workspace Cargo.toml with:
      """
      [workspace.dependencies]
      tokio = "1.0"
      """
    And a member Cargo.toml with:
      """
      [dependencies]
      tokio = "1.25"
      """
    When I run the check
    Then a finding is emitted with check_id "deps.workspace_inheritance" and code "missing_workspace_true"

  @detection @fail
  Scenario: Multiple members with missing inheritance produce findings
    Given a workspace with multiple members not using inheritance
    When I run the check
    Then multiple findings are emitted for "deps.workspace_inheritance"

  @detection @fail
  Scenario: Missing inheritance in dev-dependencies
    Given a workspace Cargo.toml with:
      """
      [workspace.dependencies]
      criterion = "0.5"
      """
    And a member Cargo.toml with:
      """
      [dev-dependencies]
      criterion = "0.5"
      """
    When I run the check
    Then a finding is emitted with check_id "deps.workspace_inheritance" and code "missing_workspace_true"

  @detection @fail
  Scenario: Missing inheritance in build-dependencies
    Given a workspace Cargo.toml with:
      """
      [workspace.dependencies]
      cc = "1.0"
      """
    And a member Cargo.toml with:
      """
      [build-dependencies]
      cc = "1.0"
      """
    When I run the check
    Then a finding is emitted with check_id "deps.workspace_inheritance" and code "missing_workspace_true"

  @detection @fail
  Scenario: Member using inline table without workspace = true
    Given a workspace Cargo.toml with:
      """
      [workspace.dependencies]
      serde = { version = "1.0", features = ["derive"] }
      """
    And a member Cargo.toml with:
      """
      [dependencies]
      serde = { version = "1.0", features = ["derive"] }
      """
    When I run the check
    Then a finding is emitted with check_id "deps.workspace_inheritance" and code "missing_workspace_true"

  # ===========================================================================
  # Correct workspace inheritance (pass cases)
  # ===========================================================================

  @pass
  Scenario: Member using workspace inheritance passes
    Given a workspace Cargo.toml with:
      """
      [workspace.dependencies]
      serde = "1.0"
      """
    And a member Cargo.toml with:
      """
      [dependencies]
      serde = { workspace = true }
      """
    When I run the check
    Then no finding is emitted for "deps.workspace_inheritance"

  @pass
  Scenario: Member using workspace inheritance with features override passes
    Given a workspace Cargo.toml with:
      """
      [workspace.dependencies]
      serde = "1.0"
      """
    And a member Cargo.toml with:
      """
      [dependencies]
      serde = { workspace = true, features = ["derive"] }
      """
    When I run the check
    Then no finding is emitted for "deps.workspace_inheritance"

  @pass
  Scenario: Non-workspace dependency in member passes
    Given a workspace Cargo.toml with:
      """
      [workspace.dependencies]
      serde = "1.0"
      """
    And a member Cargo.toml with:
      """
      [dependencies]
      rand = "0.8"
      """
    When I run the check
    Then no finding is emitted for "deps.workspace_inheritance"

  @pass
  Scenario: No workspace dependencies means no check
    Given a workspace Cargo.toml with:
      """
      [workspace]
      members = ["member"]
      """
    And a member Cargo.toml with:
      """
      [dependencies]
      serde = "1.0"
      """
    When I run the check
    Then no finding is emitted for "deps.workspace_inheritance"

  @pass
  Scenario: Single-crate repo without workspace passes
    Given a Cargo.toml with:
      """
      [dependencies]
      serde = "1.0"
      """
    When I run the check
    Then no finding is emitted for "deps.workspace_inheritance"

  # ===========================================================================
  # Multiple workspace dependencies
  # ===========================================================================

  @multiple
  Scenario: Some dependencies inherited, some not
    Given a workspace Cargo.toml with:
      """
      [workspace.dependencies]
      serde = "1.0"
      tokio = "1.0"
      rand = "0.8"
      """
    And a member Cargo.toml with:
      """
      [dependencies]
      serde = { workspace = true }
      tokio = "1.0"
      rand = { workspace = true }
      """
    When I run the check
    Then a finding is emitted for dependency "tokio"
    And no finding is emitted for dependency "serde"
    And no finding is emitted for dependency "rand"

  @multiple
  Scenario: All dependencies correctly inherited
    Given a workspace Cargo.toml with:
      """
      [workspace.dependencies]
      serde = "1.0"
      tokio = "1.0"
      """
    And a member Cargo.toml with:
      """
      [dependencies]
      serde = { workspace = true }
      tokio = { workspace = true }
      """
    When I run the check
    Then no finding is emitted for "deps.workspace_inheritance"

  # ===========================================================================
  # Configuration options
  # ===========================================================================

  @config
  Scenario: Check can be disabled via config
    Given a workspace Cargo.toml with:
      """
      [workspace.dependencies]
      serde = "1.0"
      """
    And a member Cargo.toml with:
      """
      [dependencies]
      serde = "1.0"
      """
    And a depguard.toml with:
      """
      [checks."deps.workspace_inheritance"]
      enabled = false
      """
    When I run the check
    Then no finding is emitted for "deps.workspace_inheritance"

  @config
  Scenario: Severity can be downgraded to warning
    Given a workspace Cargo.toml with:
      """
      [workspace.dependencies]
      serde = "1.0"
      """
    And a member Cargo.toml with:
      """
      [dependencies]
      serde = "1.0"
      """
    And a depguard.toml with:
      """
      [checks."deps.workspace_inheritance"]
      enabled = true
      severity = "warning"
      """
    When I run the check
    Then a finding is emitted with check_id "deps.workspace_inheritance" and code "missing_workspace_true"
    And the finding severity is "warning"

  @config @allowlist
  Scenario: Allowlist suppresses specific dependencies
    Given a workspace Cargo.toml with:
      """
      [workspace.dependencies]
      serde = "1.0"
      tokio = "1.0"
      """
    And a member Cargo.toml with:
      """
      [dependencies]
      serde = "1.0"
      tokio = "1.0"
      """
    And a depguard.toml with:
      """
      [checks."deps.workspace_inheritance"]
      enabled = true
      allow = ["serde"]
      """
    When I run the check
    Then no finding is emitted for dependency "serde"
    And a finding is emitted for dependency "tokio"

  # ===========================================================================
  # Workspace with path dependencies
  # ===========================================================================

  @workspace-path
  Scenario: Workspace path dependency with version can use inheritance
    Given a workspace Cargo.toml with:
      """
      [workspace.dependencies]
      local-lib = { version = "0.1", path = "./local-lib" }
      """
    And a member Cargo.toml with:
      """
      [dependencies]
      local-lib = { workspace = true }
      """
    When I run the check
    Then no finding is emitted for "deps.workspace_inheritance"

  @workspace-path
  Scenario: Workspace path dependency not inherited is flagged
    Given a workspace Cargo.toml with:
      """
      [workspace.dependencies]
      local-lib = { version = "0.1", path = "./local-lib" }
      """
    And a member Cargo.toml with:
      """
      [dependencies]
      local-lib = { version = "0.1", path = "../local-lib" }
      """
    When I run the check
    Then a finding is emitted with check_id "deps.workspace_inheritance" and code "missing_workspace_true"

  # ===========================================================================
  # Edge cases
  # ===========================================================================

  @edge
  Scenario: Workspace with empty dependencies section
    Given a workspace Cargo.toml with:
      """
      [workspace.dependencies]
      """
    And a member Cargo.toml with:
      """
      [dependencies]
      serde = "1.0"
      """
    When I run the check
    Then no finding is emitted for "deps.workspace_inheritance"

  @edge
  Scenario: Root package in workspace can use inheritance
    Given a workspace Cargo.toml with:
      """
      [package]
      name = "root-crate"
      version = "0.1.0"
      edition = "2021"

      [workspace]
      members = ["member"]

      [workspace.dependencies]
      serde = "1.0"

      [dependencies]
      serde = { workspace = true }
      """
    When I run the check
    Then no finding is emitted for "deps.workspace_inheritance"

  @edge
  Scenario: Root package not using inheritance is flagged
    Given a workspace Cargo.toml with:
      """
      [package]
      name = "root-crate"
      version = "0.1.0"
      edition = "2021"

      [workspace]
      members = ["member"]

      [workspace.dependencies]
      serde = "1.0"

      [dependencies]
      serde = "1.0"
      """
    When I run the check
    Then a finding is emitted with check_id "deps.workspace_inheritance" and code "missing_workspace_true"

  @edge
  Scenario: Optional workspace dependency not inherited
    Given a workspace Cargo.toml with:
      """
      [workspace.dependencies]
      optional-dep = { version = "1.0", optional = true }
      """
    And a member Cargo.toml with:
      """
      [dependencies]
      optional-dep = { version = "1.0", optional = true }
      """
    When I run the check
    Then a finding is emitted with check_id "deps.workspace_inheritance" and code "missing_workspace_true"

