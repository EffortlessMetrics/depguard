Feature: Wildcard version detection edge cases

  Edge cases and boundary conditions for wildcard version detection.

  # ===========================================================================
  # Different dependency table locations
  # ===========================================================================

  Scenario: Wildcards in [dependencies] are detected
    Given a Cargo.toml with:
      """
      [dependencies]
      serde = "*"
      """
    When I run the check
    Then a finding is emitted for "serde"

  Scenario: Wildcards in [dev-dependencies] are detected
    Given a Cargo.toml with:
      """
      [dev-dependencies]
      criterion = "*"
      """
    When I run the check
    Then a finding is emitted for "criterion"

  Scenario: Wildcards in [build-dependencies] are detected
    Given a Cargo.toml with:
      """
      [build-dependencies]
      cc = "*"
      """
    When I run the check
    Then a finding is emitted for "cc"

  Scenario: Wildcards in target-specific dependencies are detected
    Given a Cargo.toml with:
      """
      [target.'cfg(unix)'.dependencies]
      nix = "*"
      """
    When I run the check
    Then a finding is emitted for "nix"

  # ===========================================================================
  # Inline table vs separate table
  # ===========================================================================

  Scenario: Wildcard in inline table is detected
    Given a Cargo.toml with:
      """
      [dependencies]
      tokio = { version = "*", features = ["full"] }
      """
    When I run the check
    Then a finding is emitted for "tokio"

  Scenario: Wildcard in expanded table is detected
    Given a Cargo.toml with:
      """
      [dependencies.tokio]
      version = "*"
      features = ["full"]
      """
    When I run the check
    Then a finding is emitted for "tokio"

  # ===========================================================================
  # Workspace inheritance edge cases
  # ===========================================================================

  Scenario: Workspace dependency with wildcard is detected at workspace level
    Given a workspace Cargo.toml with:
      """
      [workspace.dependencies]
      serde = "*"
      """
    When I run the check
    Then a finding is emitted at the workspace root level

  Scenario: Member using workspace=true inherits version silently
    Given a workspace with wildcard in workspace.dependencies
    And a member using:
      """
      [dependencies]
      serde = { workspace = true }
      """
    When I run the check
    Then finding is only at workspace level, not member level

  # ===========================================================================
  # Partial wildcards
  # ===========================================================================

  Scenario Outline: Partial wildcard patterns
    Given a Cargo.toml with dependency "example = \"<version>\""
    When I run the check
    Then <outcome>

    Examples:
      | version | outcome                                |
      | *       | a finding is emitted                   |
      | 1.*     | a finding is emitted                   |
      | 1.2.*   | a finding is emitted                   |
      | 1.2.3   | no finding is emitted                  |
      | ^1.0    | no finding is emitted                  |
      | ~1.2    | no finding is emitted                  |
      | >=1,<2  | no finding is emitted                  |

  # ===========================================================================
  # Optional dependencies
  # ===========================================================================

  Scenario: Wildcard in optional dependency is detected
    Given a Cargo.toml with:
      """
      [dependencies]
      optional-dep = { version = "*", optional = true }
      """
    When I run the check
    Then a finding is emitted for "optional-dep"
