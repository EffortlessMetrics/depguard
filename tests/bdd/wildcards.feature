Feature: Wildcard dependency versions are rejected in strict profile

  Scenario: A dependency uses a wildcard version
    Given a Cargo manifest with dependency "serde = *"
    When depguard evaluates the manifest in "strict" profile
    Then the report contains a finding with check_id "deps.no_wildcards" and code "wildcard_version"
