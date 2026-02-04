Feature: CI/CD integration

  depguard integrates with CI systems via annotations and structured output.

  # ===========================================================================
  # GitHub Actions annotations
  # ===========================================================================

  Scenario: Annotations command outputs GHA workflow commands
    Given a report with findings
    When I run "depguard annotations --report report.json"
    Then stdout contains lines matching "::error file=<path>,line=<n>::<message>"

  Scenario: Error severity produces error annotations
    Given a report with error-level findings
    When I run "depguard annotations --report report.json"
    Then stdout contains "::error"

  Scenario: Warning severity produces warning annotations
    Given a report with warning-level findings
    When I run "depguard annotations --report report.json"
    Then stdout contains "::warning"

  Scenario: Annotation count can be limited
    Given a report with 20 findings
    When I run "depguard annotations --report report.json --max 5"
    Then stdout contains exactly 5 annotation lines

  # ===========================================================================
  # Markdown PR comments
  # ===========================================================================

  Scenario: Markdown includes summary table
    Given a report with findings
    When I run "depguard md --report report.json"
    Then stdout contains a markdown table with columns:
      | column   |
      | Severity |
      | File     |
      | Check    |
      | Message  |

  Scenario: Markdown shows verdict prominently
    Given a report with verdict "fail"
    When I run "depguard md --report report.json"
    Then stdout contains "❌" or "FAIL" indicator

  Scenario: Markdown is collapsible for many findings
    Given a report with 50 findings
    When I run "depguard md --report report.json"
    Then stdout contains "<details>" sections

  # ===========================================================================
  # Exit codes for CI
  # ===========================================================================

  Scenario: Exit code 0 indicates success
    Given a clean workspace
    When I run "depguard check"
    Then the exit code is 0
    And CI interprets this as success

  Scenario: Exit code 2 indicates policy failure
    Given a workspace with violations
    When I run "depguard check"
    Then the exit code is 2
    And CI interprets this as failure

  Scenario: Exit code 1 indicates tool error
    Given invalid inputs (missing repo, bad config)
    When I run "depguard check"
    Then the exit code is 1
    And CI interprets this as infrastructure failure

  # ===========================================================================
  # JSON schema compliance
  # ===========================================================================

  Scenario: Report conforms to JSON schema
    Given any workspace
    When I run "depguard check --report-out report.json"
    Then report.json validates against "schemas/depguard.report.v1.json"

  Scenario: Report contains schema_id for consumers
    Given any workspace
    When I run "depguard check --report-out report.json"
    Then report.json has "schema_id" = "receipt.envelope.v1"
