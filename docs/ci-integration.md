# CI Integration

Depguard is designed for CI pipelines. This guide covers setup for popular CI systems.

## Exit codes

Understanding exit codes is essential for CI integration:

| Code | Meaning | CI interpretation |
|------|---------|-------------------|
| `0` | Pass — no policy violations | Success |
| `1` | Tool error — config issues, missing files | Fail (infrastructure problem) |
| `2` | Policy failure — findings exceed threshold | Fail (code problem) |

## GitHub Actions

By default, depguard writes `artifacts/depguard/report.json`. Use `--report-version v1` if you need the legacy schema.

### Basic workflow

```yaml
name: Depguard

on:
  pull_request:
  push:
    branches: [main]

jobs:
  depguard:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0  # Needed for diff scope

      - name: Install Rust
        uses: dtolnay/rust-action@stable

      - name: Install depguard
        run: cargo install --path crates/depguard-cli

      - name: Run depguard
        run: depguard check
```

### Diff-scope for PRs

Only analyze changed manifests:

```yaml
- name: Run depguard (diff scope)
  if: github.event_name == 'pull_request'
  run: |
    depguard check \
      --scope diff \
      --base origin/${{ github.base_ref }} \
      --head HEAD

- name: Run depguard (full)
  if: github.event_name != 'pull_request'
  run: depguard check
```

### PR comments with Job Summary

```yaml
- name: Run depguard
  id: depguard
  continue-on-error: true
  run: |
    depguard check
    echo "exit_code=$?" >> $GITHUB_OUTPUT

- name: Generate summary
  if: always()
  run: |
    echo "## Depguard Report" >> $GITHUB_STEP_SUMMARY
    depguard md --report artifacts/depguard/report.json >> $GITHUB_STEP_SUMMARY

- name: Check result
  if: steps.depguard.outputs.exit_code == '2'
  run: exit 2
```

### Inline annotations

GitHub Actions supports inline annotations on PRs:

```yaml
- name: Run depguard
  run: depguard check

- name: Create annotations
  if: failure()
  run: |
    depguard annotations --report artifacts/depguard/report.json >> $GITHUB_OUTPUT
```

The annotations command outputs GitHub workflow commands:

```
::error file=crates/foo/Cargo.toml,line=12::Wildcard version '*' is not allowed
::warning file=crates/bar/Cargo.toml,line=8::Path dependency missing version
```

### Complete workflow

```yaml
name: Depguard

on:
  pull_request:
  push:
    branches: [main]

jobs:
  depguard:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0

      - name: Install Rust
        uses: dtolnay/rust-action@stable

      - name: Install depguard
        run: cargo install --path crates/depguard-cli

      - name: Run depguard
        id: check
        continue-on-error: true
        run: |
          if [ "${{ github.event_name }}" = "pull_request" ]; then
            depguard check --scope diff --base origin/${{ github.base_ref }}
          else
            depguard check
          fi

      - name: Generate summary
        if: always()
        run: |
          echo "## Depguard Report" >> $GITHUB_STEP_SUMMARY
          echo "" >> $GITHUB_STEP_SUMMARY
          depguard md --report artifacts/depguard/report.json >> $GITHUB_STEP_SUMMARY

      - name: Create annotations
        if: failure()
        run: depguard annotations --report artifacts/depguard/report.json

      - name: Upload report
        if: always()
        uses: actions/upload-artifact@v4
        with:
          name: depguard-report
          path: artifacts/depguard/report.json

      - name: Fail on policy violation
        if: steps.check.outcome == 'failure'
        run: exit 2
```

## GitLab CI

### Basic configuration

```yaml
depguard:
  stage: lint
  image: rust:latest
  script:
    - cargo install --path crates/depguard-cli
    - depguard check
  artifacts:
    paths:
      - artifacts/depguard/report.json
    when: always
```

### Diff-scope for merge requests

```yaml
depguard:
  stage: lint
  image: rust:latest
  script:
    - cargo install --path crates/depguard-cli
    - |
      if [ -n "$CI_MERGE_REQUEST_TARGET_BRANCH_NAME" ]; then
        git fetch origin $CI_MERGE_REQUEST_TARGET_BRANCH_NAME
        depguard check --scope diff --base origin/$CI_MERGE_REQUEST_TARGET_BRANCH_NAME
      else
        depguard check
      fi
  artifacts:
    paths:
      - artifacts/depguard/report.json
    when: always
```

### Merge request comments

```yaml
depguard:
  stage: lint
  image: rust:latest
  script:
    - cargo install --path crates/depguard-cli
    - depguard check || true
    - depguard md --report artifacts/depguard/report.json > artifacts/depguard/comment.md
  artifacts:
    paths:
      - artifacts/depguard/report.json
      - artifacts/depguard/comment.md
    when: always
  rules:
    - if: $CI_PIPELINE_SOURCE == "merge_request_event"
```

## CircleCI

```yaml
version: 2.1

jobs:
  depguard:
    docker:
      - image: rust:latest
    steps:
      - checkout
      - run:
          name: Install depguard
          command: cargo install --path crates/depguard-cli
      - run:
          name: Run depguard
          command: depguard check
      - store_artifacts:
          path: artifacts/depguard/report.json
          destination: depguard-report

workflows:
  lint:
    jobs:
      - depguard
```

## Azure Pipelines

```yaml
trigger:
  - main

pool:
  vmImage: ubuntu-latest

steps:
  - task: RustInstaller@1
    inputs:
      rustVersion: stable

  - script: cargo install --path crates/depguard-cli
    displayName: Install depguard

  - script: depguard check
    displayName: Run depguard

  - task: PublishPipelineArtifact@1
    inputs:
      targetPath: artifacts/depguard/report.json
      artifact: depguard-report
    condition: always()
```

## Jenkins

```groovy
pipeline {
    agent any

    stages {
        stage('Depguard') {
            steps {
                sh 'cargo install --path crates/depguard-cli'
                sh 'depguard check'
            }
            post {
                always {
                    archiveArtifacts artifacts: 'artifacts/depguard/report.json'
                }
            }
        }
    }
}
```

## Configuration tips

### Gradual adoption

Start lenient and tighten over time:

```toml
# depguard.toml - Phase 1: Observe
profile = "compat"
fail_on = "error"  # Only fail on errors, not warnings
```

```toml
# depguard.toml - Phase 2: Enforce
profile = "warn"
fail_on = "error"
```

```toml
# depguard.toml - Phase 3: Strict
profile = "strict"
fail_on = "error"
```

### Allowlists for exceptions

```toml
[checks."deps.path_requires_version"]
allow = ["internal-dev-tool", "build-script-helper"]

[checks."deps.workspace_inheritance"]
allow = ["legacy-crate-with-special-needs"]
```

### Different configs for PR vs main

Use environment variables or config file selection:

```yaml
# GitHub Actions
- name: Run depguard
  run: |
    if [ "${{ github.ref }}" = "refs/heads/main" ]; then
      depguard check --config depguard.strict.toml
    else
      depguard check --config depguard.toml
    fi
```

## Caching

### GitHub Actions

```yaml
- name: Cache depguard
  uses: actions/cache@v4
  with:
    path: ~/.cargo/bin/depguard
    key: depguard-${{ hashFiles('crates/depguard-cli/Cargo.toml') }}
```

### GitLab CI

```yaml
depguard:
  cache:
    key: depguard
    paths:
      - $CARGO_HOME/bin/depguard
```

## Troubleshooting CI

### Shallow clone issues

If you see errors about missing git refs:

```yaml
# GitHub Actions
- uses: actions/checkout@v4
  with:
    fetch-depth: 0  # Full history
```

```yaml
# GitLab CI
variables:
  GIT_DEPTH: 0
```

### Missing base branch

For diff scope, ensure the base branch is fetched:

```bash
git fetch origin main
depguard check --scope diff --base origin/main
```

### Permission issues

Ensure the CI user can write to `artifacts/depguard/`:

```yaml
- name: Create output directory
  run: mkdir -p artifacts/depguard
```

## Artifact management

### Report retention

Keep reports for trend analysis:

```yaml
# GitHub Actions
- uses: actions/upload-artifact@v4
  with:
    name: depguard-report-${{ github.sha }}
    path: artifacts/depguard/report.json
    retention-days: 90
```

### Report aggregation

For monorepos, you may want to aggregate reports:

```bash
# Run on multiple workspaces
for dir in workspace1 workspace2; do
  (cd $dir && depguard check --report-out ../artifacts/depguard/$dir-report.json)
done
```

## See also

- [Configuration](config.md) — Full config reference
- [Troubleshooting](troubleshooting.md) — Common issues
- [Exit codes](#exit-codes) — CI behavior reference

