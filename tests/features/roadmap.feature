Feature: Implementation-plan feature lock-in

  These scenarios pin newer implementation-plan capabilities to executable BDD behavior.

  Scenario: Offline yanked index flags pinned yanked versions
    Given a Cargo.toml with:
      """
      [dependencies]
      serde = "=1.0.188"
      """
    And a depguard.toml with:
      """
      [checks."deps.yanked_versions"]
      enabled = true
      severity = "error"
      """
    And a yanked index file "yanked-index.txt" with:
      """
      serde 1.0.188
      """
    When I run "depguard check --repo-root . --yanked-index yanked-index.txt"
    Then a finding is emitted with check_id "deps.yanked_versions" and code "version_yanked"
    And the exit code is 2

  Scenario: Live yanked lookup flags pinned yanked versions
    Given a Cargo.toml with:
      """
      [dependencies]
      serde = "=1.0.188"
      """
    And a depguard.toml with:
      """
      [checks."deps.yanked_versions"]
      enabled = true
      severity = "error"
      """
    And a live yanked API that marks crate "serde" version "1.0.188" as yanked
    When I run "depguard check --repo-root . --yanked-live --yanked-api-base-url __YANKED_API_URL__"
    Then a finding is emitted with check_id "deps.yanked_versions" and code "version_yanked"
    And the exit code is 2

  Scenario: Incremental mode writes and uses a manifest cache
    Given a workspace fixture "wildcards"
    When I run "depguard check --repo-root . --incremental --cache-dir .depguard-cache --report-out report.first.json"
    Then the exit code is 2
    And the file ".depguard-cache/manifests.v1.json" exists
    When I run "depguard check --repo-root . --incremental --cache-dir .depguard-cache --report-out report.second.json"
    Then the exit code is 2
    And the receipt has a finding with:
      | check_id | deps.no_wildcards |
      | code     | wildcard_version  |

  Scenario: Baseline command suppresses known findings
    Given a workspace fixture "wildcards"
    When I run "depguard baseline --repo-root . --output .depguard-baseline.json"
    Then the exit code is 0
    And the file ".depguard-baseline.json" exists
    And ".depguard-baseline.json" contains "depguard.baseline.v1"
    When I run "depguard check --repo-root . --baseline .depguard-baseline.json"
    Then the exit code is 0
    And the receipt has no findings
    And the receipt has integer field "verdict.counts.suppressed" with value 1

  Scenario: Diff scope reads changed files from --diff-file
    Given a workspace with violations in multiple files
    And a diff file "changed-files.txt" with:
      """
      all_changed_files=crates/a/Cargo.toml
      """
    When I run "depguard check --repo-root . --scope diff --diff-file changed-files.txt"
    Then the exit code is 2
    And the receipt shows scope "diff"
    And a violation is detected

  Scenario: Diff scope with --diff-file skips unchanged manifests
    Given a workspace with violations in multiple files
    And a diff file "changed-files.txt" with:
      """
      all_changed_files=README.md
      """
    When I run "depguard check --repo-root . --scope diff --diff-file changed-files.txt"
    Then the exit code is 0
    And the receipt shows scope "diff"
    And the receipt has no findings

  Scenario: Check command can emit JUnit and JSONL artifacts
    Given a workspace fixture "wildcards"
    When I run "depguard check --repo-root . --report-out report.json --write-junit --junit-out report.junit.xml --write-jsonl --jsonl-out report.jsonl"
    Then the exit code is 2
    And the file "report.junit.xml" exists
    And "report.junit.xml" contains "<testsuite"
    And the file "report.jsonl" exists
    And "report.jsonl" contains "summary"

  Scenario: SARIF command renders from an existing report
    Given a JSON report file "report.json" with findings
    When I run "depguard sarif --report report.json"
    Then the exit code is 0
    And stdout contains "2.1.0"

  Scenario: JUnit command renders from an existing report
    Given a JSON report file "report.json" with findings
    When I run "depguard junit --report report.json"
    Then the exit code is 0
    And stdout contains "<testsuite"

  Scenario: JSONL command renders from an existing report
    Given a JSON report file "report.json" with findings
    When I run "depguard jsonl --report report.json"
    Then the exit code is 0
    And stdout contains "summary"

  Scenario: Fix command writes a buildfix plan
    Given a workspace fixture "default_features_explicit"
    When I run "depguard check --repo-root . --report-out report.json"
    Then the exit code is 2
    When I run "depguard fix --report report.json --plan-out plan.json"
    Then the exit code is 0
    And the file "plan.json" exists
    And "plan.json" contains "buildfix.plan.v1"
    And "plan.json" contains "default-features = true"

  Scenario: Fix command can apply safe remediations
    Given a workspace fixture "default_features_explicit"
    When I run "depguard check --repo-root . --report-out report.json"
    Then the exit code is 2
    When I run "depguard fix --report report.json --plan-out plan.json --apply"
    Then the exit code is 0
    And "Cargo.toml" contains "default-features = true"
    When I run "depguard check --repo-root . --report-out report-after.json"
    Then the exit code is 0
    And the receipt has no findings
