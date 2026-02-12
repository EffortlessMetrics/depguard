Feature: Dependency manifest hygiene (depguard)

  depguard is a repo-truth sensor. It scans Cargo.toml files and emits findings with stable codes.

  Exit codes: 0 = pass, 2 = policy failure, 1 = runtime error

  # ===========================================================================
  # Core checks
  # ===========================================================================

  Scenario: Clean workspace passes all checks
    Given a workspace fixture "clean"
    When I run "depguard check --repo-root ."
    Then the exit code is 0
    And the receipt verdict is "pass"
    And the receipt has no findings

  Scenario: Wildcard versions are flagged
    Given a workspace fixture "wildcards"
    When I run "depguard check --repo-root ."
    Then the exit code is 2
    And the receipt verdict is "fail"
    And the receipt has a finding with:
      | check_id | deps.no_wildcards |
      | code     | wildcard_version  |
      | severity | error             |

  Scenario: Path dependency requires version for publishable crates
    Given a workspace fixture "path_missing_version"
    When I run "depguard check --repo-root ."
    Then the exit code is 2
    And the receipt has a finding with:
      | check_id | deps.path_requires_version |
      | code     | path_without_version       |
      | severity | error                      |

  Scenario: Absolute paths in dependencies are flagged
    Given a workspace fixture "path_safety"
    When I run "depguard check --repo-root ."
    Then the exit code is 2
    And the receipt has a finding with:
      | check_id | deps.path_safety |
      | code     | absolute_path    |
      | severity | error            |

  Scenario: Parent-escaping paths in dependencies are flagged
    Given a workspace fixture "path_safety"
    When I run "depguard check --repo-root ."
    Then the exit code is 2
    And the receipt has a finding with:
      | check_id | deps.path_safety |
      | code     | parent_escape    |
      | severity | error            |

  Scenario: Workspace dependency drift is prevented
    Given a workspace fixture "workspace_inheritance"
    When I run "depguard check --repo-root ."
    Then the exit code is 2
    And the receipt has a finding with:
      | check_id | deps.workspace_inheritance |
      | code     | missing_workspace_true     |
      | severity | error                      |

  # ===========================================================================
  # Output rendering
  # ===========================================================================

  Scenario: Check command writes JSON report to specified path
    Given a workspace fixture "clean"
    When I run "depguard check --repo-root . --report-out artifacts/report.json"
    Then the file "artifacts/report.json" exists
    And the file is valid JSON

  Scenario: Check command can write Markdown alongside JSON
    Given a workspace fixture "wildcards"
    When I run "depguard check --repo-root . --report-out report.json --write-markdown --markdown-out report.md"
    Then the file "report.json" exists
    And the file "report.md" exists
    And "report.md" contains "fail"
    And "report.md" contains "wildcard"

  Scenario: Markdown command renders from existing report
    Given a JSON report file "report.json" with findings
    When I run "depguard md --report report.json"
    Then the exit code is 0
    And stdout contains the verdict

  Scenario: Annotations command renders GitHub Actions format
    Given a JSON report file "report.json" with findings
    When I run "depguard annotations --report report.json"
    Then the exit code is 0
    And stdout contains "::error"

  # ===========================================================================
  # Explain command
  # ===========================================================================

  Scenario: Explain command shows info for check ID
    When I run "depguard explain deps.no_wildcards"
    Then the exit code is 0
    And stdout contains remediation guidance

  Scenario: Explain command shows info for code
    When I run "depguard explain wildcard_version"
    Then the exit code is 0
    And stdout contains remediation guidance

  Scenario: Explain command fails for unknown identifier
    When I run "depguard explain nonexistent_check"
    Then the exit code is 1

  # ===========================================================================
  # Error handling
  # ===========================================================================

  Scenario: Missing repo root returns error
    When I run "depguard check --repo-root /nonexistent/path"
    Then the exit code is 1

  Scenario: Version flag shows version
    When I run "depguard --version"
    Then the exit code is 0
    And stdout contains the version number

  # ===========================================================================
  # Receipt structure
  # ===========================================================================

  Scenario: Receipt contains required envelope fields
    Given a workspace fixture "clean"
    When I run "depguard check --repo-root . --report-out report.json"
    Then the receipt has field "schema" with value "depguard.report.v2"
    And the receipt has field "tool_name" with value "depguard"
    And the receipt has field "tool_version"
    And the receipt has field "run.started_at"
    And the receipt has field "run.ended_at"
    And the receipt has field "run.duration_ms"

  Scenario: Findings are deterministically ordered
    Given a workspace with multiple violations
    When I run the check twice
    Then both reports have identical finding order
    And findings are sorted by: severity, path, line, check_id, code, message
