Feature: deps.no_wildcards rule

  The deps.no_wildcards check detects wildcard version requirements (`*`, `1.*`, `1.2.*`)
  in dependency specifications. Wildcards make builds non-reproducible and should be
  replaced with explicit semver constraints.

  Background:
    Given the deps.no_wildcards check is enabled by default

  # ===========================================================================
  # Detection of wildcard patterns
  # ===========================================================================

  @detection @fail
  Scenario Outline: Wildcard version patterns are detected
    Given a Cargo.toml with dependency '<dependency>'
    When I run the check
    Then a finding is emitted with check_id "deps.no_wildcards" and code "wildcard_version"
    And the finding message mentions the dependency name
    And the finding severity is "error"

    Examples:
      | dependency              | description                    |
      | serde = "*"             | Pure star wildcard             |
      | tokio = "1.*"           | Major-pinned wildcard          |
      | regex = "1.2.*"         | Minor-pinned wildcard          |
      | uuid = "*.*"            | Double wildcard                |
      | rand = "0.*.*"          | Multiple wildcards             |

  @detection @fail
  Scenario: Wildcard in inline table form
    Given a Cargo.toml with:
      """
      [dependencies]
      serde = { version = "*", features = ["derive"] }
      """
    When I run the check
    Then a finding is emitted with check_id "deps.no_wildcards" and code "wildcard_version"

  @detection @fail
  Scenario: Multiple wildcard dependencies produce multiple findings
    Given a Cargo.toml with:
      """
      [dependencies]
      serde = "*"
      tokio = "1.*"
      regex = "2.*"
      """
    When I run the check
    Then the report contains 3 findings for check "deps.no_wildcards"

  @detection @fail
  Scenario: Wildcard in dev-dependencies
    Given a Cargo.toml with:
      """
      [dev-dependencies]
      criterion = "*"
      """
    When I run the check
    Then a finding is emitted with check_id "deps.no_wildcards" and code "wildcard_version"

  @detection @fail
  Scenario: Wildcard in build-dependencies
    Given a Cargo.toml with:
      """
      [build-dependencies]
      cc = "*"
      """
    When I run the check
    Then a finding is emitted with check_id "deps.no_wildcards" and code "wildcard_version"

  @detection @fail
  Scenario: Wildcard in target-specific dependencies
    Given a Cargo.toml with:
      """
      [target.'cfg(unix)'.dependencies]
      libc = "*"
      """
    When I run the check
    Then a finding is emitted with check_id "deps.no_wildcards" and code "wildcard_version"

  # ===========================================================================
  # Valid version constraints (pass cases)
  # ===========================================================================

  @pass
  Scenario Outline: Valid semver constraints pass without findings
    Given a Cargo.toml with dependency '<dependency>'
    When I run the check
    Then no finding is emitted for "deps.no_wildcards"

    Examples:
      | dependency               | description                    |
      | serde = "1.0"            | Exact version                  |
      | tokio = "1.0.0"          | Full semver                    |
      | regex = "^1.0"           | Caret requirement              |
      | uuid = "~1.2"            | Tilde requirement              |
      | rand = ">=1.0, <2.0"     | Range requirement              |
      | chrono = "0.4.31"        | Prerelease-compatible          |
      | log = "=0.4.17"          | Equals requirement             |

  @pass
  Scenario: Workspace inheritance passes
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
    Then no finding is emitted for "deps.no_wildcards"

  @pass
  Scenario: Path dependency without version passes this check
    Given a Cargo.toml with:
      """
      [dependencies]
      local-crate = { path = "../local" }
      """
    When I run the check
    Then no finding is emitted for "deps.no_wildcards"

  # ===========================================================================
  # Configuration options
  # ===========================================================================

  @config
  Scenario: Check can be disabled via config
    Given a Cargo.toml with dependency 'serde = "*"'
    And a depguard.toml with:
      """
      [checks."deps.no_wildcards"]
      enabled = false
      """
    When I run the check
    Then no finding is emitted for "deps.no_wildcards"

  @config
  Scenario: Severity can be downgraded to warning
    Given a Cargo.toml with dependency 'serde = "*"'
    And a depguard.toml with:
      """
      [checks."deps.no_wildcards"]
      severity = "warning"
      """
    When I run the check
    Then a finding is emitted with check_id "deps.no_wildcards" and code "wildcard_version"
    And the finding severity is "warning"

  @config @allowlist
  Scenario: Allowlist suppresses specific dependencies
    Given a Cargo.toml with:
      """
      [dependencies]
      serde = "*"
      tokio = "*"
      """
    And a depguard.toml with:
      """
      [checks."deps.no_wildcards"]
      allow = ["serde"]
      """
    When I run the check
    Then no finding is emitted for dependency "serde"
    And a finding is emitted for dependency "tokio"

  # ===========================================================================
  # Edge cases
  # ===========================================================================

  @edge
  Scenario: Empty dependencies section produces no findings
    Given a Cargo.toml with:
      """
      [dependencies]
      """
    When I run the check
    Then no finding is emitted for "deps.no_wildcards"

  @edge
  Scenario: Crate with no dependencies produces no findings
    Given a Cargo.toml with:
      """
      [package]
      name = "no-deps"
      version = "0.1.0"
      edition = "2021"
      """
    When I run the check
    Then no finding is emitted for "deps.no_wildcards"

  @edge
  Scenario: Git dependency without version passes
    Given a Cargo.toml with:
      """
      [dependencies]
      my-crate = { git = "https://github.com/example/my-crate" }
      """
    When I run the check
    Then no finding is emitted for "deps.no_wildcards"

