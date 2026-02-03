Feature: Deterministic output

  depguard produces byte-stable, deterministic output for reproducibility.

  # ===========================================================================
  # Byte stability
  # ===========================================================================

  Scenario: Same inputs produce identical outputs
    Given a workspace fixture "wildcards"
    When I run the check 3 times
    Then all 3 reports are byte-identical (excluding timestamps)

  Scenario: Finding order is deterministic
    Given a workspace with violations in multiple files
    When I run the check
    Then findings are sorted by:
      | priority | field    |
      | 1        | severity |
      | 2        | path     |
      | 3        | line     |
      | 4        | check_id |
      | 5        | code     |
      | 6        | message  |

  Scenario: JSON keys are consistently ordered
    Given any workspace
    When I run the check
    Then JSON object keys appear in consistent order
    And no random ordering affects output

  # ===========================================================================
  # Timestamp handling
  # ===========================================================================

  Scenario: Timestamps are ISO 8601 format
    Given a workspace fixture "clean"
    When I run the check
    Then "started_at" is ISO 8601 format
    And "finished_at" is ISO 8601 format
    And "finished_at" >= "started_at"

  # ===========================================================================
  # Golden file compatibility
  # ===========================================================================

  Scenario: Output matches golden fixtures
    Given a workspace fixture "<fixture>"
    When I run the check
    Then the output matches "expected.report.json" (ignoring timestamps)

    Examples:
      | fixture               |
      | clean                 |
      | wildcards             |
      | path_missing_version  |
      | path_safety           |
      | workspace_inheritance |
