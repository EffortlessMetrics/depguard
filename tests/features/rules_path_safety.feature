Feature: deps.path_safety rule

  The deps.path_safety check detects unsafe path dependencies:
  - Absolute paths (not portable across machines)
  - Parent traversal that escapes the repository root

  Background:
    Given the deps.path_safety check is enabled by default

  # ===========================================================================
  # Absolute path detection
  # ===========================================================================

  @detection @fail @absolute
  Scenario Outline: Absolute paths are flagged
    Given a Cargo.toml with dependency path "<path>"
    When I run the check
    Then a finding is emitted with check_id "deps.path_safety" and code "absolute_path"
    And the finding message mentions "absolute path"

    Examples:
      | path                       | description          |
      | /opt/libs/my-crate         | Unix root path       |
      | /home/user/project/crate   | Unix home path       |
      | /usr/local/lib/my-crate    | Unix usr path        |
      | C:\\libs\\my-crate         | Windows C drive      |
      | D:\\projects\\my-crate     | Windows D drive      |
      | C:/libs/my-crate           | Windows forward slash|
      | E:\\work\\dependencies     | Windows E drive      |

  @detection @fail @absolute
  Scenario: Absolute path with version is still flagged
    Given a Cargo.toml with:
      """
      [dependencies]
      my-crate = { version = "1.0", path = "/opt/libs/my-crate" }
      """
    When I run the check
    Then a finding is emitted with check_id "deps.path_safety" and code "absolute_path"

  @detection @fail @absolute
  Scenario: Multiple absolute paths produce multiple findings
    Given a Cargo.toml with:
      """
      [dependencies]
      crate-a = { path = "/opt/crate-a", version = "1.0" }
      crate-b = { path = "C:\\crate-b", version = "1.0" }
      """
    When I run the check
    Then the report contains 2 findings with code "absolute_path"

  # ===========================================================================
  # Parent escape detection
  # ===========================================================================

  @detection @fail @escape
  Scenario: Parent traversal escaping workspace is flagged
    Given a workspace at "/repo"
    And a Cargo.toml with:
      """
      [dependencies]
      outside = { path = "../../../outside-workspace", version = "1.0" }
      """
    When I run the check
    Then a finding is emitted with check_id "deps.path_safety" and code "parent_escape"
    And the finding message mentions "escapes"

  @detection @fail @escape
  Scenario: Deep parent traversal from root manifest
    Given a Cargo.toml at the root with:
      """
      [dependencies]
      outside = { path = "../outside", version = "1.0" }
      """
    When I run the check
    Then a finding is emitted with check_id "deps.path_safety" and code "parent_escape"

  @detection @fail @escape
  Scenario: Parent traversal from nested crate escapes workspace
    Given a workspace Cargo.toml with:
      """
      [workspace]
      members = ["crates/nested"]
      """
    And a nested crate at "crates/nested" with:
      """
      [dependencies]
      outside = { path = "../../../outside", version = "1.0" }
      """
    When I run the check
    Then a finding is emitted with check_id "deps.path_safety" and code "parent_escape"

  @detection @fail @escape
  Scenario: Mixed forward and back traversal that escapes
    Given a Cargo.toml with:
      """
      [dependencies]
      sneaky = { path = "./subdir/../../..", version = "1.0" }
      """
    When I run the check
    Then a finding is emitted with check_id "deps.path_safety" and code "parent_escape"

  @detection @fail
  Scenario: Both absolute and escape violations reported
    Given a Cargo.toml with:
      """
      [dependencies]
      abs-dep = { path = "/opt/libs/abs-dep", version = "1.0" }
      escape-dep = { path = "../../../outside", version = "1.0" }
      """
    When I run the check
    Then a finding is emitted with check_id "deps.path_safety" and code "absolute_path"
    And a finding is emitted with check_id "deps.path_safety" and code "parent_escape"

  # ===========================================================================
  # Valid relative paths (pass cases)
  # ===========================================================================

  @pass
  Scenario: Relative path within workspace passes
    Given a workspace with member crates
    And a Cargo.toml with:
      """
      [dependencies]
      sibling-crate = { path = "../sibling-crate", version = "1.0" }
      """
    When I run the check
    Then no finding is emitted for "deps.path_safety"

  @pass
  Scenario: Simple relative path passes
    Given a Cargo.toml with:
      """
      [dependencies]
      local-crate = { path = "./local-crate", version = "1.0" }
      """
    When I run the check
    Then no finding is emitted for "deps.path_safety"

  @pass
  Scenario: Subdirectory path passes
    Given a Cargo.toml with:
      """
      [dependencies]
      nested = { path = "libs/nested", version = "1.0" }
      """
    When I run the check
    Then no finding is emitted for "deps.path_safety"

  @pass
  Scenario: Parent traversal within workspace bounds passes
    Given a workspace Cargo.toml with:
      """
      [workspace]
      members = ["crates/a", "crates/b"]
      """
    And a crate at "crates/a" with:
      """
      [dependencies]
      crate-b = { path = "../b", version = "1.0" }
      """
    When I run the check
    Then no finding is emitted for "deps.path_safety"

  @pass
  Scenario: Non-path dependency passes
    Given a Cargo.toml with:
      """
      [dependencies]
      serde = "1.0"
      """
    When I run the check
    Then no finding is emitted for "deps.path_safety"

  @pass
  Scenario: Git dependency passes
    Given a Cargo.toml with:
      """
      [dependencies]
      my-crate = { git = "https://github.com/example/my-crate" }
      """
    When I run the check
    Then no finding is emitted for "deps.path_safety"

  # ===========================================================================
  # Configuration options
  # ===========================================================================

  @config
  Scenario: Check can be disabled via config
    Given a Cargo.toml with:
      """
      [dependencies]
      my-crate = { path = "/opt/my-crate", version = "1.0" }
      """
    And a depguard.toml with:
      """
      [checks."deps.path_safety"]
      enabled = false
      """
    When I run the check
    Then no finding is emitted for "deps.path_safety"

  @config
  Scenario: Severity can be downgraded to warning
    Given a Cargo.toml with:
      """
      [dependencies]
      my-crate = { path = "/opt/my-crate", version = "1.0" }
      """
    And a depguard.toml with:
      """
      [checks."deps.path_safety"]
      severity = "warning"
      """
    When I run the check
    Then a finding is emitted with check_id "deps.path_safety" and code "absolute_path"
    And the finding severity is "warning"

  @config @allowlist
  Scenario: Allowlist suppresses specific paths
    Given a Cargo.toml with:
      """
      [dependencies]
      allowed-dep = { path = "/opt/allowed-dep", version = "1.0" }
      blocked-dep = { path = "/opt/blocked-dep", version = "1.0" }
      """
    And a depguard.toml with:
      """
      [checks."deps.path_safety"]
      allow = ["/opt/allowed-dep"]
      """
    When I run the check
    Then no finding is emitted for path "/opt/allowed-dep"
    And a finding is emitted with code "absolute_path"

  # ===========================================================================
  # Dependency sections
  # ===========================================================================

  @sections
  Scenario: Absolute path in dev-dependencies
    Given a Cargo.toml with:
      """
      [dev-dependencies]
      test-utils = { path = "/opt/test-utils", version = "1.0" }
      """
    When I run the check
    Then a finding is emitted with check_id "deps.path_safety" and code "absolute_path"

  @sections
  Scenario: Absolute path in build-dependencies
    Given a Cargo.toml with:
      """
      [build-dependencies]
      build-helper = { path = "/opt/build-helper", version = "1.0" }
      """
    When I run the check
    Then a finding is emitted with check_id "deps.path_safety" and code "absolute_path"

  @sections
  Scenario: Absolute path in target-specific dependencies
    Given a Cargo.toml with:
      """
      [target.'cfg(unix)'.dependencies]
      unix-only = { path = "/opt/unix-only", version = "1.0" }
      """
    When I run the check
    Then a finding is emitted with check_id "deps.path_safety" and code "absolute_path"

  # ===========================================================================
  # Edge cases
  # ===========================================================================

  @edge
  Scenario: Path with current directory prefix passes
    Given a Cargo.toml with:
      """
      [dependencies]
      local = { path = "./local", version = "1.0" }
      """
    When I run the check
    Then no finding is emitted for "deps.path_safety"

  @edge
  Scenario: Empty path string passes this check
    Given a Cargo.toml with:
      """
      [dependencies]
      empty-path = { path = "", version = "1.0" }
      """
    When I run the check
    Then no finding is emitted for "deps.path_safety"

  @edge
  Scenario: Path with multiple consecutive slashes normalizes
    Given a Cargo.toml with:
      """
      [dependencies]
      double-slash = { path = "./libs//nested", version = "1.0" }
      """
    When I run the check
    Then no finding is emitted for "deps.path_safety"

