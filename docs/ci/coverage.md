# Coverage

Codecov coverage is Rust execution-surface evidence.

## What it answers

> Did tests execute this Rust surface?

## What it does not answer

- Whether dependency-policy behavior is correct
- Whether yanked-resolution behavior is correct
- Whether baselines and inline suppressions are semantically correct
- Whether GitHub annotations are complete
- Whether SARIF, JUnit, Markdown, JSONL, or report renderers are complete
- Whether `depguard explain` coverage is complete
- Whether mutation adequacy is strong
- Whether release readiness is proven

Those are separate proof lanes, verified by:

- **Dependency-policy correctness**: Policy unit tests, property tests, conformance tests
- **Yanked-resolution correctness**: Conformance tests with offline yanked indexes
- **Baseline/suppression correctness**: Integration tests with fixture validation
- **Renderer completeness**: Golden-file tests for Markdown, SARIF, JUnit, JSONL outputs
- **Explain coverage completeness**: Explain coverage workflow and check-registry audits
- **Mutation adequacy**: Scheduled mutation testing on domain-checks crate
- **Release readiness**: Release workflow validation, schema stability checks

## Workflow triggers

The Coverage workflow runs on:

- **Push to `main`**: Full coverage upload to Codecov (blocking upload)
- **`workflow_dispatch`**: Manual trigger for coverage reports
- **PRs labeled `coverage`, `full-ci`, or `ci:full`**: Advisory coverage reports (non-blocking)

## Artifacts

Durable receipts are generated in order of precedence:

1. **coverage-receipt.json**: Local receipt with schema version and claim boundaries
2. **coverage.json**: LLVM coverage JSON export
3. **coverage.txt**: Human-readable coverage summary
4. **lcov.info**: LCOV format for Codecov upload
5. **GitHub Actions artifact**: 14-day retention on `coverage-report`
6. **Codecov dashboard**: Historical coverage trends (if `CODECOV_TOKEN` configured)

Codecov comments are disabled; rely on artifacts and the Codecov dashboard for reports.
