Feature: Dependency manifest hygiene (depguard)

  depguard is a repo-truth sensor. It scans Cargo.toml files and emits findings with stable codes.

  Scenario: Wildcard versions are flagged
    Given a workspace fixture "wildcards"
    When I run "depguard check --root . --profile team --scope repo"
    Then the receipt "artifacts/depguard/report.json" has a finding with:
      | check_id | deps.no_wildcards |
      | code     | wildcard_version  |
      | severity | error             |

  Scenario: Path dependency requires version for publishable crates
    Given a workspace fixture "path_missing_version"
    When I run "depguard check --root . --profile strict --scope repo"
    Then the receipt has a finding with:
      | check_id | deps.path_requires_version |
      | code     | missing_version            |
      | severity | error                      |

  Scenario: Workspace dependency drift is prevented in strict mode
    Given a workspace fixture "workspace_inheritance_drift"
    When I run "depguard check --root . --profile strict --scope repo"
    Then the receipt has a finding with:
      | check_id | deps.workspace_inheritance |
      | code     | not_inherited              |
      | severity | error                      |
