Title: feat(v2): v2 report format + enhanced dependency checks

Summary

This branch (feat/v2-report-and-checks) implements the v2 report format and a set of refactors and feature improvements across the domain, repo discovery, renderers, CLI and test fixtures. Main goals: introduce a stable v2 report contract, tighten and extend dependency checks, add test coverage and update renderers and docs to consume the new contract.

Motivation

- Produce a more robust and versioned reporting envelope (depguard.report.v2) so downstream consumers can migrate safely.
- Improve detection and reporting of dependency hygiene violations (wildcards, paths, workspace inheritance, optional/unused, etc.).
- Provide fix/action tokens in reports for automated remediation flows.

What changed (high level)

- Contracts & docs
  - Added contracts/docs/finding-payload.md (new documentation for finding payloads)
  - Updated contract schemas under contracts/schemas (sensor.report, cockpit.report, buildfix.plan)
  - Added docs/output-contract.md to describe the v2 output contract

- Domain changes (crates/depguard-domain)
  - Added and refactored checks (no_wildcards, path_requires_version, path_safety, workspace_inheritance, optional_unused, dev_only_in_normal, no_multiple_versions, git_requires_version, default_features_explicit)
  - Reworked engine/fingerprint/report modules to include section names, current specs and fix-action tokens
  - Added test support helpers and a new proptest harness
  - New unit and integration tests added in crates/depguard-domain (including tests.rs and test_support.rs)

- Repo parsing and discovery (crates/depguard-repo)
  - Improved manifest parsing and workspace discovery
  - Added repository-level tests and a build_workspace_model test

- Rendering & CLI
  - Updated depguard-render to support GHA and Markdown for the new report format
  - Updated depguard-app and depguard-cli to emit the new v2 receipt and wire up check/explain/render flows
  - Updated CLI tests and fixtures

- Tests & fixtures
  - Updated many expected.*.json fixtures under tests/fixtures to reflect v2 output
  - Added tests/features/checks.feature updates

Key files touched (representative)

- contracts/docs/finding-payload.md (A)
- contracts/schemas/{buildfix.plan.v1.json,cockpit.report.v1.json,sensor.report.v1.json} (M)
- crates/depguard-domain/src/checks/* (M/A), proptest.rs (M), report.rs (M)
- crates/depguard-repo/src/{discover.rs,parse.rs} (M)
- crates/depguard-app/src/{check.rs,explain.rs,render.rs,report.rs} (M)
- crates/depguard-render/src/{markdown.rs,gha.rs} (M)
- crates/depguard-cli/{src,tests} (M)
- docs/{checks.md,output-contract.md} (M/A)
- tests/fixtures/** (many updated)

Notable commit messages in branch (recent)

- feat: enhance dependency declaration in proptest with wildcards and locations
- style: fix formatting and collapse nested if statements
- Add tests for dependency checks and enhance validation logic
- docs: define depguard finding payload contract and output contract
- feat: add fix action tokens for dependency checks and update expected reports

Testing performed / required

- New and updated unit and integration tests are included in this branch. CI is expected to run the full matrix.
- Locally suggested commands to validate changes before merge:
  - cargo fmt --all && cargo clippy --all-targets --all-features
  - cargo test --workspace
  - cargo test -p depguard-domain
  - (Optional) cargo build --release

Migration & compatibility notes

- This introduces/updates the v2 report schema (depguard.report.v2). Consumers that read receipts must be updated to accept the v2 schema/URN.
- The sensor/report schema IDs were tightened; tools that parse the older envelope may need adjustments.
- No runtime behavioral changes for end users other than the new report envelope, but automation consuming receipts should be validated.

Breaking changes

- Schema ID/URN changes: update any downstream contract validators.
- Some render output and fixture shapes changed - this is intentional (v2).

Suggested reviewers

- depguard-domain maintainers and authors of checks (review domain and test changes)
- depguard-repo maintainers (workspace discovery + parsing)
- depguard-render / depguard-app maintainers (rendering & CLI changes)

Checklist before merge

- [ ] Run cargo fmt and clippy and fix any warnings
- [ ] Run the full test suite (cargo test --workspace)
- [ ] Ensure CI is green on all platforms
- [ ] Confirm downstream consumers have been notified about the v2 contract
- [ ] Add an entry to CHANGELOG or release notes summarizing the v2 contract and feature scope

How to open the PR (recommended)

If you have the GitHub CLI installed and the branch is pushed:

  git push --set-upstream origin feat/v2-report-and-checks
  gh pr create --base main --title "feat(v2): v2 report format + enhanced dependency checks" --body-file PR_BODY.md --reviewer "<team-or-user>" --label "semver:minor","type:feature"

If you don't have gh installed:

  1. Push the branch: git push --set-upstream origin feat/v2-report-and-checks
  2. Open a new PR on GitHub and copy/paste the contents of PR_BODY.md into the PR body.

Notes / next steps

- Recommend running the domain crate's mutation tests for extra confidence (cargo mutants --package depguard-domain) if available.
- Consider adding an integration smoke test that validates a generated v2 receipt against the schema in CI.


