Feature: Individual check behaviors

  Detailed scenarios for each check's detection logic.

  # ===========================================================================
  # deps.no_wildcards
  # ===========================================================================

  Scenario Outline: Wildcard version patterns are detected
    Given a Cargo.toml with dependency '<dependency>'
    When I run the check
    Then a finding is emitted with check_id "deps.no_wildcards" and code "wildcard_version"

    Examples:
      | dependency           | description              |
      | serde = "*"          | Star wildcard            |
      | tokio = "1.*"        | Major-pinned wildcard    |
      | regex = "1.2.*"      | Minor-pinned wildcard    |

  Scenario Outline: Valid version constraints pass
    Given a Cargo.toml with dependency '<dependency>'
    When I run the check
    Then no finding is emitted for "deps.no_wildcards"

    Examples:
      | dependency           | description              |
      | serde = "1.0"        | Exact version            |
      | tokio = "^1.0"       | Caret requirement        |
      | regex = "~1.2"       | Tilde requirement        |
      | uuid = ">=1.0, <2.0" | Range requirement        |

  # ===========================================================================
  # deps.path_requires_version
  # ===========================================================================

  Scenario: Path dependency without version is flagged
    Given a Cargo.toml with:
      """
      [dependencies]
      my-crate = { path = "../my-crate" }
      """
    When I run the check
    Then a finding is emitted with check_id "deps.path_requires_version" and code "path_without_version"

  Scenario: Path dependency with version passes
    Given a Cargo.toml with:
      """
      [dependencies]
      my-crate = { version = "0.1", path = "../my-crate" }
      """
    When I run the check
    Then no finding is emitted for "deps.path_requires_version"

  # ===========================================================================
  # deps.path_safety
  # ===========================================================================

  Scenario Outline: Absolute paths are flagged
    Given a Cargo.toml with dependency path "<path>"
    When I run the check
    Then a finding is emitted with check_id "deps.path_safety" and code "absolute_path"

    Examples:
      | path                      | description        |
      | /opt/libs/my-crate        | Unix absolute      |
      | C:\\libs\\my-crate        | Windows drive      |
      | D:/projects/my-crate      | Windows forward    |

  Scenario: Parent traversal escaping workspace is flagged
    Given a workspace at "/repo"
    And a Cargo.toml with:
      """
      [dependencies]
      outside = { path = "../../../outside-workspace" }
      """
    When I run the check
    Then a finding is emitted with check_id "deps.path_safety" and code "parent_escape"

  Scenario: Relative path within workspace passes
    Given a workspace with member crates
    And a Cargo.toml with:
      """
      [dependencies]
      sibling = { path = "../sibling-crate" }
      """
    When I run the check
    Then no finding is emitted for "deps.path_safety"

  # ===========================================================================
  # deps.workspace_inheritance
  # ===========================================================================

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
    And a depguard.toml with:
      """
      [checks."deps.workspace_inheritance"]
      enabled = true
      """
    When I run the check
    Then a finding is emitted with check_id "deps.workspace_inheritance" and code "missing_workspace_true"

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
    And a depguard.toml with:
      """
      [checks."deps.workspace_inheritance"]
      enabled = true
      """
    When I run the check
    Then no finding is emitted for "deps.workspace_inheritance"

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
    And a depguard.toml with:
      """
      [checks."deps.workspace_inheritance"]
      enabled = true
      """
    When I run the check
    Then no finding is emitted for "deps.workspace_inheritance"
