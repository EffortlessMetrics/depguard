# Troubleshooting

Common issues and solutions when using depguard.

## Exit codes

| Code | Meaning | Action |
|------|---------|--------|
| `0` | Pass | Nothing to do |
| `1` | Tool error | Check config, paths, git setup |
| `2` | Policy failure | Fix findings or adjust config |

## Common errors

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
   # Use a TOML validator
   cat depguard.toml | toml-lint
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

## Finding-specific issues

### False positives

**Symptom**: Depguard reports issues that are intentional.

**Solutions**:

1. Use allowlists in config:
   ```toml
   [checks."deps.path_requires_version"]
   allow = ["my-internal-crate"]
   ```

2. Disable specific checks:
   ```toml
   [checks."deps.workspace_inheritance"]
   enabled = false
   ```

3. Downgrade severity:
   ```toml
   [checks."deps.no_wildcards"]
   severity = "warning"
   ```

### Missing findings

**Symptom**: Expected violations aren't reported.

**Causes**:
- Check is disabled
- Manifest not in scope
- Crate is in allowlist

**Solutions**:

1. Verify check is enabled:
   ```bash
   # Check effective config
   cat depguard.toml
   ```

2. Verify manifest is discovered:
   ```bash
   # Run in verbose mode (if available)
   depguard check --verbose
   ```

3. Use `--scope repo` instead of diff:
   ```bash
   depguard check --scope repo
   ```

### Line numbers are wrong

**Symptom**: Findings point to incorrect lines.

**Causes**:
- File was modified after parsing
- TOML parser limitation with certain constructs

**Solutions**:
- This is usually a limitation of TOML location tracking
- The file path is always correct; line is best-effort
- Open an issue if consistently wrong for specific patterns

## Performance issues

### Slow on large workspaces

**Symptom**: Depguard takes a long time to complete.

**Causes**:
- Many manifests to parse
- Slow filesystem (network drive, etc.)

**Solutions**:

1. Use diff scope in CI:
   ```bash
   depguard check --scope diff --base origin/main
   ```

2. Limit findings output:
   ```toml
   max_findings = 50
   ```

3. Exclude experimental directories:
   ```toml
   # In workspace Cargo.toml
   [workspace]
   exclude = ["experiments/*", "scratch/*"]
   ```

### Out of memory

**Symptom**: Process killed or OOM errors.

**Causes**:
- Extremely large workspace
- Malformed TOML files causing parser issues

**Solutions**:
- Check for malformed TOML files
- File an issue with details about workspace size

## CI-specific issues

### Workflow fails but no findings shown

**Symptom**: CI fails with exit code 2, but no output visible.

**Solutions**:

1. Generate and display the report:
   ```yaml
   - name: Run depguard
     run: depguard check
     continue-on-error: true

   - name: Show report
     if: always()
     run: cat artifacts/depguard/report.json | jq .
   ```

2. Use markdown output:
   ```yaml
   - name: Show findings
     if: always()
     run: depguard md --report artifacts/depguard/report.json
   ```

### Annotations not appearing on PR

**Symptom**: `depguard annotations` runs but no inline comments appear.

**Causes**:
- Output not captured correctly
- File paths don't match checkout path
- GitHub Actions annotation limit reached

**Solutions**:

1. Verify annotation format:
   ```bash
   depguard annotations --report artifacts/depguard/report.json
   # Should output lines like:
   # ::error file=path/to/Cargo.toml,line=12::message
   ```

2. Ensure paths are relative to workspace:
   ```yaml
   - uses: actions/checkout@v4
   - name: Run from checkout root
     run: depguard check
   ```

### Shallow clone errors

**Symptom**: "fatal: ambiguous argument 'origin/main'"

**Solutions**:

```yaml
# GitHub Actions - fetch full history
- uses: actions/checkout@v4
  with:
    fetch-depth: 0

# GitLab CI
variables:
  GIT_DEPTH: 0
  GIT_STRATEGY: clone
```

## Configuration issues

### Config file not found

**Symptom**: Default config used even though file exists.

**Solutions**:

1. Check file location (must be in repo root):
   ```bash
   ls -la depguard.toml
   ```

2. Specify explicitly:
   ```bash
   depguard check --config ./depguard.toml
   ```

3. Check file name (case-sensitive):
   ```bash
   # Correct
   depguard.toml

   # Wrong
   Depguard.toml
   DEPGUARD.toml
   ```

### Profile not applied

**Symptom**: Settings don't match expected profile defaults.

**Causes**:
- Config file overrides profile
- CLI flags override config

**Resolution order** (highest priority first):
1. CLI flags
2. Config file values
3. Profile defaults

To debug:
```toml
# Remove explicit overrides to see profile defaults
profile = "strict"
# Don't set individual check severities if you want profile defaults
```

## Getting more help

### Generate a debug report

```bash
# Include in bug reports:
depguard check 2>&1 | tee depguard-output.txt
cat depguard.toml
cargo --version
git --version
```

### Explain a finding

```bash
depguard explain deps.no_wildcards
depguard explain wildcard_version
```

### Check all available explanations

```bash
# List all check IDs
depguard explain --list
```

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

## See also

- [Configuration](config.md) — Full config reference
- [Checks Catalog](checks.md) — Understanding findings
- [CI Integration](ci-integration.md) — CI setup guides

