Feature: Configuration and profiles

  depguard supports configuration via depguard.toml and built-in profiles.

  Background:
    Given the default configuration profile is "strict"

  # ===========================================================================
  # Built-in profiles
  # ===========================================================================

  Scenario: Strict profile treats all checks as errors
    Given a workspace with violations
    When I run "depguard check --profile strict"
    Then all findings have severity "error"
    And the verdict is "fail"

  Scenario: Warn profile treats violations as warnings
    Given a workspace with violations
    When I run "depguard check --profile warn"
    Then findings have severity "warning"
    And the verdict is "fail" with exit code 2

  Scenario: Compat profile is permissive for legacy codebases
    Given a workspace with violations
    When I run "depguard check --profile compat"
    Then most checks are disabled or downgraded
    And the verdict is "pass" or "warn"

  # ===========================================================================
  # Custom configuration file
  # ===========================================================================

  Scenario: Config file overrides default profile
    Given a workspace fixture "wildcards"
    And a depguard.toml with:
      """
      profile = "warn"
      """
    When I run "depguard check --repo-root ."
    Then findings have severity "warning"

  Scenario: CLI profile flag overrides config file
    Given a workspace fixture "wildcards"
    And a depguard.toml with:
      """
      profile = "warn"
      """
    When I run "depguard check --repo-root . --profile strict"
    Then findings have severity "error"

  Scenario: Per-check severity override
    Given a workspace fixture "wildcards"
    And a depguard.toml with:
      """
      [checks."deps.no_wildcards"]
      severity = "warning"
      """
    When I run "depguard check --repo-root ."
    Then the wildcard finding has severity "warning"

  Scenario: Check can be disabled
    Given a workspace fixture "wildcards"
    And a depguard.toml with:
      """
      [checks."deps.no_wildcards"]
      enabled = false
      """
    When I run "depguard check --repo-root ."
    Then there are no findings for "deps.no_wildcards"

  # ===========================================================================
  # Allowlists
  # ===========================================================================

  Scenario: Allowlist suppresses specific dependencies
    Given a workspace fixture "path_missing_version"
    And a depguard.toml with:
      """
      [checks."deps.path_requires_version"]
      allow = ["local-dev-crate"]
      """
    When I run "depguard check --repo-root ."
    Then there are no findings for dependency "local-dev-crate"

  # ===========================================================================
  # Fail-on threshold
  # ===========================================================================

  Scenario: fail_on=error passes when only warnings exist
    Given a workspace with warning-level findings
    And a depguard.toml with:
      """
      fail_on = "error"
      """
    When I run "depguard check --repo-root ."
    Then the exit code is 0
    And the verdict is "warn"

  Scenario: fail_on=warning fails when warnings exist
    Given a workspace with warning-level findings
    And a depguard.toml with:
      """
      fail_on = "warning"
      """
    When I run "depguard check --repo-root ."
    Then the exit code is 2
    And the verdict is "fail"

  # ===========================================================================
  # Max findings limit
  # ===========================================================================

  Scenario: Max findings limits output
    Given a workspace with 10 violations
    When I run "depguard check --max-findings 3"
    Then the report contains exactly 3 findings
    And the report indicates findings were truncated
