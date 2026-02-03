# Checks catalog

Depguard checks are identified by a stable `check_id` and a stable `code`.

**Naming convention:**
- `check_id` is a dotted namespace (e.g., `deps.no_wildcards`)
- `code` is a short snake_case discriminator (e.g., `wildcard_version`)

The code registry lives in `crates/depguard-types/src/ids.rs`.

---

## `deps.no_wildcards`

Detects wildcard version specifiers that allow any version.

### Codes

| Code | Trigger |
|------|---------|
| `wildcard_version` | Version is `*` or contains wildcard segments like `1.*` |

### Examples

```toml
# Bad
serde = "*"
tokio = "1.*"

# Good
serde = "1.0"
tokio = "1.35"
```

### Remediation

Pin to a specific version or version range. Use `cargo update` to find the latest compatible version.

---

## `deps.path_requires_version`

Detects path dependencies without an explicit version, which can cause issues when publishing.

### Codes

| Code | Trigger |
|------|---------|
| `path_without_version` | Dependency has `path = "..."` but no `version = "..."` |

### Examples

```toml
# Bad
my-crate = { path = "../my-crate" }

# Good
my-crate = { version = "0.1", path = "../my-crate" }
```

### Remediation

Add a `version` field alongside `path`. This ensures the crate can be published to crates.io and consumers get the right version.

### Configuration

```toml
[checks."deps.path_requires_version"]
enabled = true
severity = "error"
allow = ["internal-dev-tool"]  # Crates that don't need version
```

---

## `deps.path_safety`

Detects path dependencies that could cause portability or security issues.

### Codes

| Code | Trigger |
|------|---------|
| `absolute_path` | Path is absolute (`/abs/path` or `C:\path`) |
| `parent_escape` | Path escapes the workspace root via `..` segments |

### Examples

```toml
# Bad - absolute path
my-crate = { path = "/home/user/my-crate" }
my-crate = { path = "C:\\Users\\dev\\my-crate" }

# Bad - escapes workspace
shared = { path = "../../other-repo/shared" }

# Good - relative within workspace
my-crate = { path = "../my-crate" }
```

### Remediation

- For absolute paths: Convert to workspace-relative paths
- For parent escapes: Move the dependency into the workspace or use a git/crates.io dependency

### Configuration

```toml
[checks."deps.path_safety"]
enabled = true
severity = "error"
allow = []  # No allowlist by default
```

---

## `deps.workspace_inheritance`

Detects dependencies that should use `workspace = true` but don't.

### Codes

| Code | Trigger |
|------|---------|
| `missing_workspace_true` | Dependency exists in `[workspace.dependencies]` but member doesn't use `{ workspace = true }` |

### Examples

```toml
# In workspace Cargo.toml
[workspace.dependencies]
serde = "1.0"

# Bad - member doesn't inherit
[dependencies]
serde = "1.0"

# Good - member inherits
[dependencies]
serde = { workspace = true }
```

### Remediation

Change the dependency to `{ workspace = true }` to inherit the version from the workspace root. This ensures version consistency across the workspace.

### Configuration

```toml
[checks."deps.workspace_inheritance"]
enabled = true
severity = "warning"
allow = ["special-crate"]  # Crates allowed to override
```

---

## Adding a new check

1. **Add IDs** — Add `check_id` and `code` constants to `crates/depguard-types/src/ids.rs`:
   ```rust
   pub const DEPS_MY_CHECK: &str = "deps.my_check";
   pub const MY_CODE: &str = "my_code";
   ```

2. **Add explanation** — Add entry to `crates/depguard-types/src/explain.rs`:
   ```rust
   (DEPS_MY_CHECK, Explanation { title: "...", description: "...", ... })
   ```

3. **Implement check** — Create `crates/depguard-domain/src/checks/my_check.rs`:
   ```rust
   pub fn run(manifest: &ManifestModel, ..., policy: &CheckPolicy) -> Vec<Finding> {
       // Implementation
   }
   ```

4. **Wire into engine** — Update `crates/depguard-domain/src/checks/mod.rs`

5. **Add tests**:
   - Unit tests in the check module
   - BDD scenario in `tests/features/`
   - Golden fixture if output changes

6. **Document** — Add section to this file

## Severity levels

| Level | Meaning | Default fail_on behavior |
|-------|---------|-------------------------|
| `info` | Informational, no action needed | Never fails |
| `warning` | Should be addressed | Fails if `fail_on = "warning"` |
| `error` | Must be addressed | Always fails |

Configure per-check severity in `depguard.toml`:

```toml
[checks."deps.no_wildcards"]
severity = "warning"  # Downgrade from default "error"
```
