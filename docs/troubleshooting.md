# Troubleshooting

Common issues and solutions when using depguard.

## Exit codes

| Code | Meaning | Action |
|------|---------|--------|
| `0` | Pass | Nothing to do |
| `1` | Tool error | Check config, paths, git setup |
| `2` | Policy failure | Fix findings or adjust config |

---

## Configuration Issues

### Config File Not Found

**Symptom**: Default config used even though file exists, or warning about missing config.

**Common Causes**:
- File not in the expected location
- File naming issue (case sensitivity)
- Running from wrong directory

**Solutions**:

1. Check file location (must be in repo root by default):
   ```bash
   ls -la depguard.toml
   ```

2. Specify config file explicitly:
   ```bash
   depguard check --config ./path/to/depguard.toml
   ```

3. Verify file name is exactly `depguard.toml` (case-sensitive on Linux/macOS):
   ```bash
   # Correct
   depguard.toml
   
   # Wrong - won't be found
   Depguard.toml
   DEPGUARD.toml
   depguard.TOML
   ```

4. Check you're running from the correct directory:
   ```bash
   pwd  # Should be workspace root
   depguard check --repo-root .
   ```

### Invalid TOML Syntax

**Symptom**: Exit code 1 with config parsing error like "expected equals" or "invalid character".

**Common Causes**:
- Missing quotes around strings with special characters
- Incorrect array syntax
- Mixed table and inline table syntax
- Trailing commas (not allowed in TOML)

**Solutions**:

1. Validate TOML syntax with a linter:
   ```bash
   # Using taplo
   taplo check depguard.toml
   
   # Or using Python
   python -c "import tomllib; tomllib.load(open('depguard.toml', 'rb'))"
   ```

2. Common syntax fixes:
   ```toml
   # WRONG: Missing quotes for dotted keys
   checks.deps.no_wildcards.severity = "error"
   
   # CORRECT: Quoted keys
   [checks."deps.no_wildcards"]
   severity = "error"
   
   # WRONG: Trailing comma
   allow = ["crate1", "crate2", ]
   
   # CORRECT: No trailing comma
   allow = ["crate1", "crate2"]
   
   # WRONG: Mixed inline and table
   [checks."deps.no_wildcards"]
   allow = ["crate1"]
   [checks."deps.no_wildcards".options]  # Conflict!
   
   # CORRECT: Use one style
   [checks."deps.no_wildcards"]
   allow = ["crate1"]
   
   [checks."deps.no_wildcards.options"]
   some_option = true
   ```

3. Use a TOML-aware editor with syntax highlighting to catch errors early.

### Unknown Check IDs

**Symptom**: Error like "unknown check ID 'deps.unknown_check'".

**Common Causes**:
- Typo in check ID
- Using outdated check ID from older version
- Check doesn't exist in current version

**Solutions**:

1. List all available checks:
   ```bash
   depguard explain --list
   ```

2. Verify check ID spelling:
   ```toml
   # WRONG: Typo
   [checks."deps.no_widlcard"]  # "widlcard" typo
   
   # CORRECT
   [checks."deps.no_wildcards"]
   ```

3. Check the check catalog:
   ```bash
   depguard explain deps.no_wildcards
   ```

4. Common check ID typos:
   ```toml
   # These are WRONG
   [checks."dep.no_wildcards"]      # "dep" should be "deps"
   [checks."deps.no_wildcard"]       # Should be plural "wildcards"
   [checks."deps.path_requires_ver"] # Truncated
   
   # These are CORRECT
   [checks."deps.no_wildcards"]
   [checks."deps.path_requires_version"]
   [checks."deps.git_requires_version"]
   ```

### Invalid Severity Values

**Symptom**: Error like "invalid severity 'critical', expected one of: info, warning, error".

**Common Causes**:
- Using non-standard severity names
- Case sensitivity issues

**Solutions**:

1. Use only valid severity values:
   ```toml
   # Valid severities (lowercase only)
   severity = "info"
   severity = "warning"
   severity = "error"
   
   # INVALID - will cause errors
   severity = "critical"
   severity = "Error"
   severity = "WARNING"
   severity = "high"
   ```

2. Check profile defaults if you're unsure what severity a check uses:
   ```bash
   depguard explain deps.no_wildcards
   ```

### Profile Resolution Issues

**Symptom**: Settings don't match expected profile defaults, or unexpected check behavior.

**Common Causes**:
- Misunderstanding profile inheritance
- Config values override profile defaults
- CLI flags override everything

**Resolution order** (highest priority first):
1. CLI flags
2. Config file explicit values
3. Profile defaults
4. Built-in defaults

**Solutions**:

1. Understand what each profile enables:
   ```toml
   # "strict" - All checks enabled at error severity
   profile = "strict"
   
   # "warn" - All checks enabled at warning severity
   profile = "warn"
   
   # "compat" - Minimal checks, warning severity
   profile = "compat"
   ```

2. Debug effective configuration:
   ```toml
   # Start with just a profile to see its defaults
   profile = "strict"
   
   # Then add only necessary overrides
   [checks."deps.no_wildcards"]
   severity = "warning"  # Override just this check
   ```

3. Avoid conflicting settings:
   ```toml
   # CONFUSING: Profile says warn, but fail_on says error
   profile = "warn"
   fail_on = "error"  # This may cause unexpected failures
   
   # BETTER: Be explicit about intent
   profile = "strict"
   fail_on = "error"
   ```

---

## Manifest Parsing Issues

### Malformed Cargo.toml

**Symptom**: Error parsing manifest, or "invalid TOML" errors for Cargo.toml files.

**Common Causes**:
- Syntax errors in manifest
- Invalid dependency format
- Corrupted file encoding

**Solutions**:

1. Validate manifest syntax:
   ```bash
   # Cargo can validate manifests
   cargo metadata --format-version 1 --no-deps 2>&1 | head -20
   
   # Or use taplo
   taplo check Cargo.toml
   ```

2. Common manifest syntax errors:
   ```toml
   # WRONG: Missing [package] table
   name = "my-crate"
   version = "0.1.0"
   
   # CORRECT
   [package]
   name = "my-crate"
   version = "0.1.0"
   
   # WRONG: Invalid version string
   version = "1.0"  # Missing patch version
   
   # CORRECT
   version = "1.0.0"
   ```

3. Check file encoding:
   ```bash
   file Cargo.toml  # Should be ASCII or UTF-8
   ```

### Missing Required Fields

**Symptom**: Error about missing package name, version, or edition.

**Common Causes**:
- Incomplete manifest
- Workspace member missing package section
- Copy-paste error

**Solutions**:

1. Ensure required fields exist:
   ```toml
   [package]
   name = "my-crate"      # Required
   version = "0.1.0"      # Required
   edition = "2021"       # Recommended
   ```

2. For workspace members, each needs its own `[package]`:
   ```toml
   # workspace/Cargo.toml
   [workspace]
   members = ["crates/*"]
   
   # workspace/crates/my-crate/Cargo.toml
   [package]
   name = "my-crate"
   version = "0.1.0"
   ```

### Invalid Dependency Specifications

**Symptom**: Parse errors or unexpected behavior with dependencies.

**Common Causes**:
- Invalid version syntax
- Mixed dependency formats
- Invalid feature names

**Solutions**:

1. Use valid version requirements:
   ```toml
   # Valid version specs
   serde = "1.0"
   serde = "1.0.0"
   serde = "^1.0"
   serde = ">=1.0.0, <2.0.0"
   serde = "~1.0.0"
   
   # INVALID - wildcard (depguard will flag this)
   serde = "*"
   serde = "1.*"
   
   # INVALID - not valid semver
   serde = "latest"
   serde = ""
   ```

2. Use consistent dependency format:
   ```toml
   # Simple form
   serde = "1.0"
   
   # Extended form
   [dependencies.serde]
   version = "1.0"
   features = ["derive"]
   
   # DON'T mix both for same crate
   # This is confusing and error-prone
   ```

3. Check feature names:
   ```toml
   # Valid feature names (identifiers)
   features = ["derive", "std", "serde_json"]
   
   # INVALID
   features = ["my-feature"]  # Hyphens not allowed in feature names
   features = [""]            # Empty string
   ```

### Workspace Configuration Problems

**Symptom**: Workspace members not discovered, or "root manifest not found".

**Common Causes**:
- Missing `[workspace]` section in root
- Incorrect member paths
- Nested workspaces

**Solutions**:

1. Verify workspace structure:
   ```toml
   # Root Cargo.toml
   [workspace]
   members = ["crates/*", "apps/*"]
   resolver = "2"
   
   # Or for a package that is also a workspace
   [package]
   name = "workspace-root"
   version = "0.1.0"
   
   [workspace]
   members = ["crates/*"]
   ```

2. Check member glob patterns:
   ```bash
   # Test if glob matches expected directories
   ls -d crates/*/
   ```

3. Handle nested workspaces (virtual workspace inside package):
   ```toml
   # This is supported but can be confusing
   # Root is a package
   [package]
   name = "root"
   
   # And also a workspace
   [workspace]
   members = ["inner"]
   
   # inner/ can have its own workspace
   # but depguard treats it as part of the parent
   ```

4. Use `exclude` to skip problematic directories:
   ```toml
   [workspace]
   members = ["crates/*"]
   exclude = ["crates/experimental", "legacy"]
   ```

---

## Check-Specific Issues

### Wildcard Version False Positives

**Symptom**: `deps.no_wildcards` flags dependencies that seem intentional.

**Common Causes**:
- Legitimate use of `*` for local development
- Git dependencies without version (different check)
- Misunderstanding what constitutes a wildcard

**What counts as a wildcard**:
```toml
# These are wildcards (flagged)
serde = "*"
serde = "1.*"
serde = "*.*"
serde = ""

# These are NOT wildcards (allowed)
serde = "1"        # Means ^1.0.0
serde = "1.0"      # Means ^1.0.0
serde = ">=1.0"    # Range, not wildcard
```

**Solutions**:

1. Use explicit version constraints:
   ```toml
   # Instead of
   serde = "*"
   
   # Use
   serde = "1.0"
   # or
   serde = ">=1.0.0, <2.0.0"
   ```

2. Allow specific crates if wildcard is intentional:
   ```toml
   [checks."deps.no_wildcards"]
   allow = ["my-dev-only-crate"]
   ```

3. Disable for specific dependency types:
   ```toml
   # Only check production dependencies
   [checks."deps.no_wildcards"]
   include_dev = false
   ```

### Path Dependency Edge Cases

**Symptom**: `deps.path_requires_version` or `deps.path_safety` findings unexpectedly.

**Common Causes**:
- Path dependencies without version specification
- Path dependencies pointing outside workspace
- Path dependencies with workspace inheritance

**Understanding the rules**:

1. `deps.path_requires_version`: Path deps should have a version for publishing
   ```toml
   # Flagged - no version
   [dependencies]
   my-crate = { path = "../my-crate" }
   
   # OK - has version
   [dependencies]
   my-crate = { path = "../my-crate", version = "0.1.0" }
   ```

2. `deps.path_safety`: Path deps outside workspace may not work when published
   ```toml
   # Flagged - escapes workspace
   my-crate = { path = "../../other-workspace/crate" }
   
   # OK - within workspace
   my-crate = { path = "../crate" }
   ```

**Solutions**:

1. Add version to path dependencies:
   ```toml
   [dependencies]
   my-crate = { path = "../my-crate", version = "0.1.0" }
   ```

2. For crates that won't be published:
   ```toml
   [package]
   publish = false  # Indicates dev-only crate
   
   [dependencies]
   my-dev-tool = { path = "../tools" }  # OK, won't be published
   ```

3. Allow specific path patterns:
   ```toml
   [checks."deps.path_safety"]
   allow = ["my-monorepo-shared"]
   ```

### Git Dependency Version Requirements

**Symptom**: `deps.git_requires_version` findings for git dependencies.

**Common Causes**:
- Git dependencies without version specification
- Using only branch/tag/rev without version

**Understanding the check**:
Git dependencies should include a version for crates.io compatibility:

```toml
# Flagged - no version
[dependencies]
my-crate = { git = "https://github.com/user/repo" }

# OK - has version
[dependencies]
my-crate = { git = "https://github.com/user/repo", version = "0.1.0" }

# OK - with tag (provides some versioning info)
[dependencies]
my-crate = { git = "https://github.com/user/repo", tag = "v0.1.0" }
```

**Solutions**:

1. Add version to git dependencies:
   ```toml
   [dependencies]
   my-crate = { git = "https://github.com/user/repo", version = "0.1.0" }
   ```

2. Use tags for versioning:
   ```toml
   [dependencies]
   my-crate = { git = "https://github.com/user/repo", tag = "v0.1.0" }
   ```

3. Allow specific git dependencies:
   ```toml
   [checks."deps.git_requires_version"]
   allow = ["my-fork-only"]
   ```

4. Disable if git dependencies are intentional:
   ```toml
   [checks."deps.git_requires_version"]
   enabled = false
   ```

### Workspace Inheritance Confusion

**Symptom**: `deps.workspace_inheritance` findings, or dependencies not resolving correctly.

**Common Causes**:
- Missing `workspace = true` for inherited deps
- Dependency defined in wrong place
- Version mismatch between workspace and crate

**Understanding workspace inheritance**:
```toml
# Workspace root Cargo.toml
[workspace]
members = ["crates/*"]

[workspace.dependencies]
serde = "1.0"
tokio = { version = "1.0", features = ["full"] }

# Member crate Cargo.toml
[package]
name = "my-crate"
version = "0.1.0"

[dependencies]
serde = { workspace = true }  # Inherits from workspace
tokio = { workspace = true }  # Inherits full spec
```

**Common mistakes**:
```toml
# WRONG: Specifying version when inheriting
[dependencies]
serde = { workspace = true, version = "1.0" }  # Can't override

# WRONG: Forgetting workspace = true
[dependencies]
serde = "1.0"  # This is a separate dependency, not inherited

# CORRECT: Just inherit
[dependencies]
serde = { workspace = true }
```

**Solutions**:

1. Ensure workspace defines the dependency:
   ```toml
   # In workspace root
   [workspace.dependencies]
   my-shared-dep = "1.0"
   ```

2. Use correct inheritance syntax:
   ```toml
   # In member crate
   [dependencies]
   my-shared-dep = { workspace = true }
   ```

3. Allow specific crates to not use inheritance:
   ```toml
   [checks."deps.workspace_inheritance"]
   allow = ["legacy-crate"]
   ```

---

## Output and Reporting Issues

### Report Format Questions

**Symptom**: Unclear what fields mean in report.json, or how to parse it.

**Common Causes**:
- Unfamiliarity with schema
- Version mismatch between expected and actual format

**Solutions**:

1. Check the schema version:
   ```bash
   cat artifacts/depguard/report.json | jq '.version'
   ```

2. View the schema definition:
   ```bash
   # Schema files are in the schemas/ directory
   cat schemas/depguard.report.v2.json | jq .
   ```

3. Key report fields:
   ```json
   {
     "version": "2",
     "repo_root": "/path/to/repo",
     "timestamp": "2024-01-15T10:30:00Z",
     "verdict": "fail",
     "findings": [
       {
         "check_id": "deps.no_wildcards",
         "code": "wildcard_version",
         "severity": "error",
         "path": "crates/my-crate/Cargo.toml",
         "line": 12,
         "message": "Dependency 'serde' uses wildcard version"
       }
     ]
   }
   ```

4. Convert to other formats:
   ```bash
   # Markdown
   depguard md --report artifacts/depguard/report.json
   
   # SARIF (for GitHub code scanning)
   depguard sarif --report artifacts/depguard/report.json
   
   # JUnit XML (for CI systems)
   depguard junit --report artifacts/depguard/report.json
   
   # JSON Lines (for log aggregation)
   depguard jsonl --report artifacts/depguard/report.json
   ```

### Exit Code Meanings

**Symptom**: CI fails with unexpected exit code.

**Reference**:

| Code | Meaning | Typical Cause |
|------|---------|---------------|
| 0 | Pass | No findings, or only info-level findings |
| 1 | Tool error | Config error, file not found, parse error |
| 2 | Policy failure | Findings at or above `fail_on` severity |

**Common scenarios**:

```bash
# Exit code 0 - all good
depguard check
echo $?  # 0

# Exit code 1 - something broke
depguard check --config nonexistent.toml
echo $?  # 1

# Exit code 2 - policy violation found
depguard check  # Found error-level findings
echo $?  # 2
```

**In CI**:
```yaml
# GitHub Actions - handle different exit codes
- name: Run depguard
  id: depguard
  run: |
    depguard check || echo "exit_code=$?" >> $GITHUB_OUTPUT
    exit 0  # Don't fail the step

- name: Check results
  run: |
    if [ "${{ steps.depguard.outputs.exit_code }}" == "2" ]; then
      echo "Policy violations found"
      exit 1
    fi
```

### CI Integration Problems

**Symptom**: depguard works locally but fails in CI, or vice versa.

**Common Causes**:
- Different file paths in CI
- Missing dependencies
- Permission issues
- Environment variable differences

**Solutions**:

1. Debug CI environment:
   ```yaml
   - name: Debug
     run: |
       pwd
       ls -la
       which depguard
       depguard --version
   ```

2. Ensure consistent paths:
   ```yaml
   - uses: actions/checkout@v4
   - name: Run from checkout root
     run: |
       cd $GITHUB_WORKSPACE
       depguard check --repo-root .
   ```

3. Handle missing config gracefully:
   ```yaml
   - name: Run depguard
     run: |
       if [ -f "depguard.toml" ]; then
         depguard check --config depguard.toml
       else
         depguard check  # Use defaults
       fi
   ```

4. Capture output for debugging:
   ```yaml
   - name: Run depguard
     run: depguard check 2>&1 | tee depguard-output.txt
     continue-on-error: true
   
   - name: Upload output
     uses: actions/upload-artifact@v4
     with:
       name: depguard-output
       path: depguard-output.txt
   ```

### GitHub Actions Annotations Not Showing

**Symptom**: `depguard annotations` runs but no inline comments appear on PR.

**Common Causes**:
- Output not captured correctly
- File paths don't match checkout path
- GitHub Actions annotation limit (10 per step)
- Workflow permissions

**Solutions**:

1. Verify annotation format:
   ```bash
   depguard annotations --report artifacts/depguard/report.json
   # Should output lines like:
   # ::error file=crates/my-crate/Cargo.toml,line=12,title=deps.no_wildcards::Wildcard version detected
   ```

2. Ensure output is captured:
   ```yaml
   - name: Create annotations
     run: depguard annotations --report artifacts/depguard/report.json
     # IMPORTANT: Don't pipe to file or redirect stdout
   ```

3. Verify file paths are relative:
   ```yaml
   # WRONG - absolute paths won't match
   # ::error file=/home/runner/work/repo/crates/Cargo.toml::...
   
   # CORRECT - relative to workspace
   # ::error file=crates/Cargo.toml::...
   ```

4. Check workflow permissions:
   ```yaml
   permissions:
     checks: write      # Required for check runs
     pull-requests: write  # Required for PR comments
   ```

5. Stay under annotation limits:
   ```yaml
   - name: Create annotations (limited)
     run: |
       # GitHub limits to 10 annotations per step
       depguard annotations --report artifacts/depguard/report.json | head -10
   ```

6. Use SARIF for better GitHub integration:
   ```yaml
   - name: Generate SARIF
     run: depguard sarif --report artifacts/depguard/report.json > results.sarif
   
   - name: Upload SARIF
     uses: github/codeql-action/upload-sarif@v3
     with:
       sarif_file: results.sarif
   ```

---

## Performance Issues

### Large Workspace Performance

**Symptom**: depguard takes a long time to complete on large workspaces.

**Common Causes**:
- Many manifests to parse (100+ crates)
- Slow filesystem (network drives, CI runners)
- Full repo scan when diff would suffice

**Solutions**:

1. Use diff scope in CI (only analyze changed crates):
   ```bash
   # Only check manifests that changed vs main
   depguard check --scope diff --base origin/main
   ```

2. Exclude unnecessary directories:
   ```toml
   # In workspace Cargo.toml
   [workspace]
   members = ["crates/*"]
   exclude = [
     "crates/deprecated/*",
     "crates/experimental/*",
     "examples",
     "tests/fixtures"
   ]
   ```

3. Limit findings output (reduces I/O):
   ```toml
   # In depguard.toml
   max_findings = 100
   ```

4. Run in parallel with other CI jobs:
   ```yaml
   # GitHub Actions
   jobs:
     depguard:
       runs-on: ubuntu-latest
       steps:
         - uses: actions/checkout@v4
         - run: depguard check
     
     test:
       runs-on: ubuntu-latest
       steps:
         - uses: actions/checkout@v4
         - run: cargo test
   ```

### Memory Usage Concerns

**Symptom**: Process uses a lot of memory, or OOM errors.

**Common Causes**:
- Extremely large workspace (500+ crates)
- Very large manifest files
- Memory leak (bug)

**Solutions**:

1. Monitor memory usage:
   ```bash
   # Linux
   /usr/bin/time -v depguard check 2>&1 | grep "Maximum resident"
   
   # macOS
   /usr/bin/time -l depguard check 2>&1 | grep "maximum resident"
   ```

2. Process in batches using diff scope:
   ```bash
   # Only process changed files
   depguard check --scope diff --base origin/main
   ```

3. Report memory issues:
   ```bash
   # If you see unexpected memory usage, please report:
   depguard --version
   # Number of crates in workspace
   find . -name "Cargo.toml" | wc -l
   ```

### Slow Startup Times

**Symptom**: depguard takes several seconds before any output.

**Common Causes**:
- Workspace discovery scanning many directories
- Large git repository (for diff scope)
- Cold filesystem cache

**Solutions**:

1. Warm up filesystem cache:
   ```bash
   # Pre-scan the workspace
   find . -name "Cargo.toml" > /dev/null
   depguard check
   ```

2. Use precomputed file list for diff scope:
   ```bash
   # Generate file list separately
   git diff --name-only origin/main > changed-files.txt
   
   # Use precomputed list
   depguard check --scope diff --diff-file changed-files.txt
   ```

3. For very large repos, consider:
   ```bash
   # Partial clone (if using git)
   git clone --filter=blob:none --depth 1
   
   # Or sparse checkout
   git sparse-checkout init
   ```

---

## Common Errors (Quick Reference)

### "No Cargo.toml found"

**Symptom**: Exit code 1 with message about missing manifest.

**Causes**:
- Running depguard outside a Rust project
- Specifying wrong `--repo-root`

**Solutions**:
```bash
# Run from the workspace root
cd /path/to/your/workspace
depguard check

# Or specify the root explicitly
depguard check --repo-root /path/to/your/workspace
```

### "Invalid configuration"

**Symptom**: Exit code 1 with config parsing error.

**Causes**:
- TOML syntax error in `depguard.toml`
- Unknown check ID in config
- Invalid value for a setting

**Solutions**:

1. Validate TOML syntax:
   ```bash
   taplo check depguard.toml
   ```

2. Check for typos in check IDs:
   ```toml
   # Wrong
   [checks."dep.no_wildcards"]

   # Correct
   [checks."deps.no_wildcards"]
   ```

3. Verify enum values:
   ```toml
   # Valid profiles: strict, warn, compat
   profile = "strict"

   # Valid severities: info, warning, error
   severity = "error"

   # Valid fail_on: error, warning
   fail_on = "error"
   ```

### "Git ref not found" (diff scope)

**Symptom**: Exit code 1 when using `--scope diff`.

**Causes**:
- Base branch not fetched
- Shallow clone missing history
- Typo in ref name

**Solutions**:

1. Fetch the base branch:
   ```bash
   git fetch origin main
   depguard check --scope diff --base origin/main
   ```

2. Use full clone depth in CI:
   ```yaml
   # GitHub Actions
   - uses: actions/checkout@v4
     with:
       fetch-depth: 0
   ```

3. Verify the ref exists:
   ```bash
   git rev-parse origin/main
   ```

4. Use a precomputed file list instead of git refs:
   ```bash
   depguard check --scope diff --diff-file changed-files.txt
   ```

### "Permission denied writing report"

**Symptom**: Exit code 1 when writing output files.

**Causes**:
- `artifacts/depguard/` directory doesn't exist
- No write permission
- Path is a file, not a directory

**Solutions**:

1. Create the directory:
   ```bash
   mkdir -p artifacts/depguard
   depguard check
   ```

2. Specify a different output path:
   ```bash
   depguard check --report-out ./my-report.json
   ```

3. Check permissions:
   ```bash
   ls -la artifacts/depguard/
   ```

---

## Getting More Help

### Generate a Debug Report

```bash
# Include in bug reports:
depguard check 2>&1 | tee depguard-output.txt
cat depguard.toml
cargo --version
git --version
depguard --version
```

### Explain a Finding

```bash
depguard explain deps.no_wildcards
depguard explain wildcard_version
```

### Check All Available Explanations

```bash
# List all check IDs
depguard explain --list
```

---

## FAQ

### Does depguard need network access?

No. Depguard is fully offline. It only reads local files.

### Does depguard run cargo build?

No. Depguard only parses `Cargo.toml` files. It doesn't compile code or resolve the full dependency graph.

### Can I use depguard without a workspace?

Yes. Single-crate projects work fine. Depguard treats them as a workspace with one member.

### How do I ignore a specific dependency?

Use the `allow` list for the relevant check:

```toml
[checks."deps.path_requires_version"]
allow = ["my-special-crate"]
```

### Why is the output non-deterministic?

It shouldn't be. If you see different output for the same input, please file a bug report with:
- Input manifests
- Config file
- Both outputs
- depguard version

### Can I run depguard on a subset of crates?

Use diff scope to limit analysis:

```bash
# Only changed manifests
depguard check --scope diff --base main
```

Or exclude crates from the workspace:

```toml
# Cargo.toml
[workspace]
exclude = ["crates/experimental/*"]
```

---

## See also

- [Configuration](config.md) — Full config reference
- [Checks Catalog](checks.md) — Understanding findings
- [CI Integration](ci-integration.md) — CI setup guides
