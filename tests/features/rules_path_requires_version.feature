Feature: deps.path_requires_version rule

  The deps.path_requires_version check ensures that path dependencies in publishable
  crates also specify an explicit version. This is required for crates.io publishing
  and ensures consumers can use the crate even without access to the local path.

  Background:
    Given the deps.path_requires_version check is enabled by default

  # ===========================================================================
  # Detection of path dependencies without version
  # ===========================================================================

  @detection @fail
  Scenario: Path dependency without version is flagged
    Given a Cargo.toml with:
      """
      [dependencies]
      my-crate = { path = "../my-crate" }
      """
    When I run the check
    Then a finding is emitted with check_id "deps.path_requires_version" and code "path_without_version"
    And the finding message mentions "my-crate"
    And the finding severity is "error"

  @detection @fail
  Scenario: Multiple path dependencies without version
    Given a Cargo.toml with:
      """
      [dependencies]
      crate-a = { path = "../crate-a" }
      crate-b = { path = "../crate-b" }
      """
    When I run the check
    Then the report contains 2 findings for check "deps.path_requires_version"

  @detection @fail
  Scenario: Path dependency in dev-dependencies without version
    Given a Cargo.toml with:
      """
      [dev-dependencies]
      test-utils = { path = "../test-utils" }
      """
    When I run the check
    Then a finding is emitted with check_id "deps.path_requires_version" and code "path_without_version"

  @detection @fail
  Scenario: Path dependency in build-dependencies without version
    Given a Cargo.toml with:
      """
      [build-dependencies]
      build-helper = { path = "../build-helper" }
      """
    When I run the check
    Then a finding is emitted with check_id "deps.path_requires_version" and code "path_without_version"

  @detection @fail
  Scenario: Path dependency in target-specific dependencies without version
    Given a Cargo.toml with:
      """
      [target.'cfg(windows)'.dependencies]
      win-utils = { path = "../win-utils" }
      """
    When I run the check
    Then a finding is emitted with check_id "deps.path_requires_version" and code "path_without_version"

  # ===========================================================================
  # Valid path dependencies (pass cases)
  # ===========================================================================

  @pass
  Scenario: Path dependency with version passes
    Given a Cargo.toml with:
      """
      [dependencies]
      my-crate = { version = "0.1", path = "../my-crate" }
      """
    When I run the check
    Then no finding is emitted for "deps.path_requires_version"

  @pass
  Scenario: Path dependency using workspace inheritance passes
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
    Then no finding is emitted for "deps.path_requires_version"

  @pass
  Scenario: Non-path dependency passes
    Given a Cargo.toml with:
      """
      [dependencies]
      serde = "1.0"
      """
    When I run the check
    Then no finding is emitted for "deps.path_requires_version"

  @pass
  Scenario: Git dependency without version passes this check
    Given a Cargo.toml with:
      """
      [dependencies]
      my-crate = { git = "https://github.com/example/my-crate" }
      """
    When I run the check
    Then no finding is emitted for "deps.path_requires_version"

  # ===========================================================================
  # Unpublishable crates (publish = false)
  # ===========================================================================

  @publish
  Scenario: Unpublishable crates are skipped by default
    Given a Cargo.toml with:
      """
      [package]
      name = "internal-tool"
      version = "0.1.0"
      edition = "2021"
      publish = false

      [dependencies]
      internal-lib = { path = "../internal-lib" }
      """
    When I run the check
    Then no finding is emitted for "deps.path_requires_version"

  @publish
  Scenario: Publishable crates are checked
    Given a Cargo.toml with:
      """
      [package]
      name = "public-crate"
      version = "0.1.0"
      edition = "2021"

      [dependencies]
      internal-lib = { path = "./libs/internal-lib" }
      """
    When I run the check
    Then a finding is emitted with check_id "deps.path_requires_version" and code "path_without_version"

  @publish
  Scenario: Explicit publish = true is checked
    Given a Cargo.toml with:
      """
      [package]
      name = "public-crate"
      version = "0.1.0"
      edition = "2021"
      publish = true

      [dependencies]
      local-dep = { path = "./libs/local-dep" }
      """
    When I run the check
    Then a finding is emitted with check_id "deps.path_requires_version" and code "path_without_version"

  @publish @config
  Scenario: Config option to check unpublishable crates
    Given a Cargo.toml with:
      """
      [package]
      name = "internal-tool"
      version = "0.1.0"
      edition = "2021"
      publish = false

      [dependencies]
      internal-lib = { path = "./libs/internal-lib" }
      """
    And a depguard.toml with:
      """
      [checks."deps.path_requires_version"]
      ignore_publish_false = true
      """
    When I run the check
    Then a finding is emitted with check_id "deps.path_requires_version" and code "path_without_version"

  # ===========================================================================
  # Configuration options
  # ===========================================================================

  @config
  Scenario: Check can be disabled via config
    Given a Cargo.toml with:
      """
      [dependencies]
      local-crate = { path = "../local-crate" }
      """
    And a depguard.toml with:
      """
      [checks."deps.path_requires_version"]
      enabled = false
      """
    When I run the check
    Then no finding is emitted for "deps.path_requires_version"

  @config
  Scenario: Severity can be downgraded to warning
    Given a Cargo.toml with:
      """
      [dependencies]
      local-crate = { path = "./libs/local-crate" }
      """
    And a depguard.toml with:
      """
      [checks."deps.path_requires_version"]
      severity = "warning"
      """
    When I run the check
    Then a finding is emitted with check_id "deps.path_requires_version" and code "path_without_version"
    And the finding severity is "warning"

  @config @allowlist
  Scenario: Allowlist suppresses specific dependencies
    Given a Cargo.toml with:
      """
      [dependencies]
      local-dev = { path = "./libs/local-dev" }
      local-prod = { path = "./libs/local-prod" }
      """
    And a depguard.toml with:
      """
      [checks."deps.path_requires_version"]
      allow = ["local-dev"]
      """
    When I run the check
    Then no finding is emitted for dependency "local-dev"
    And a finding is emitted for dependency "local-prod"

  # ===========================================================================
  # Edge cases
  # ===========================================================================

  @edge
  Scenario: Path with version and features passes
    Given a Cargo.toml with:
      """
      [dependencies]
      my-crate = { version = "1.0", path = "../my-crate", features = ["extra"] }
      """
    When I run the check
    Then no finding is emitted for "deps.path_requires_version"

  @edge
  Scenario: Path dependency with empty version passes (has explicit version key)
    Given a Cargo.toml with:
      """
      [dependencies]
      my-crate = { version = "", path = "../my-crate" }
      """
    When I run the check
    # Note: An empty string is still considered as "having a version" even if invalid.
    # This allows the crate author to catch it via other Cargo validation.
    Then no finding is emitted for "deps.path_requires_version"

