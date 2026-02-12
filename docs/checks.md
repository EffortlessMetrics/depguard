# Checks Catalog

> **Navigation**: [Quick Start](quickstart.md) | [Configuration](config.md) | Checks | [CI Integration](ci-integration.md) | [Architecture](architecture.md) | [Troubleshooting](troubleshooting.md)

Depguard checks are identified by a stable `check_id` and a stable `code`.

**Naming convention:**
- `check_id` is a dotted namespace (e.g., `deps.no_wildcards`)
- `code` is a short snake_case discriminator (e.g., `wildcard_version`)

The code registry lives in `crates/depguard-types/src/ids.rs`.

For machine-readable finding payload shapes and fix action tokens, see [`contracts/docs/finding-payload.md`](../contracts/docs/finding-payload.md).

Allowlists are **glob patterns** (case-sensitive) across checks unless otherwise noted.

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

By default, this check is skipped for crates with `publish = false`. Set `ignore_publish_false = true` to enforce regardless of publishability.

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
allow = ["internal-*"]  # Glob patterns (case-sensitive)
ignore_publish_false = true  # Enforce even when publish = false
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

> **Note**: This check is **disabled by default** in all profiles. Enable it explicitly if you want enforcement.

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
allow = ["special-*"]  # Glob patterns allowed to override
```

---

## `deps.git_requires_version`

Detects git dependencies without an explicit version, which can cause issues when publishing.

By default, this check is skipped for crates with `publish = false`. Set `ignore_publish_false = true` to enforce regardless of publishability.

> **Note**: This check is **disabled by default**. Enable it explicitly if you want enforcement.

### Codes

| Code | Trigger |
|------|---------|
| `git_without_version` | Dependency has `git = "..."` but no `version = "..."` |

### Examples

```toml
# Bad
my-crate = { git = "https://github.com/org/my-crate" }

# Good
my-crate = { git = "https://github.com/org/my-crate", version = "0.1" }
```

### Remediation

Add a `version` field alongside `git`. This ensures the crate can be published to crates.io and consumers get the right version from the registry.

### Configuration

```toml
[checks."deps.git_requires_version"]
enabled = true
severity = "error"
allow = ["internal-*"]  # Glob patterns (case-sensitive)
ignore_publish_false = true  # Enforce even when publish = false
```

---

## `deps.dev_only_in_normal`

Detects crates that are typically dev-only appearing in `[dependencies]`.

This check flags common test/mock/benchmark crates that should typically be in `[dev-dependencies]`:
- Test frameworks: proptest, quickcheck, rstest, test-case
- Mocking: mockall, mockito, wiremock
- Benchmarking: criterion, divan
- Test utilities: tempfile, assert_cmd, insta

> **Note**: This check is **disabled by default**. Enable it explicitly if you want enforcement.

### Codes

| Code | Trigger |
|------|---------|
| `dev_dep_in_normal` | Dev-only crate found in `[dependencies]` instead of `[dev-dependencies]` |

### Examples

```toml
# Bad - test framework in normal deps
[dependencies]
mockall = "0.11"
proptest = "1.0"

# Good - in dev-dependencies
[dev-dependencies]
mockall = "0.11"
proptest = "1.0"
```

### Remediation

Move the dependency to `[dev-dependencies]` unless it's genuinely needed in production code. If intentional, add to the allowlist.

### Configuration

```toml
[checks."deps.dev_only_in_normal"]
enabled = true
severity = "warning"
allow = ["tempfile"]  # Allow specific crates in normal deps
```

---

## `deps.default_features_explicit`

Detects dependencies with inline options that don't explicitly set `default-features`.

When a dependency has inline options (features, optional, path, git) but doesn't explicitly declare `default-features = true/false`, it can lead to unclear intent.

> **Note**: This check is **disabled by default**. Enable it explicitly if you want enforcement.

### Codes

| Code | Trigger |
|------|---------|
| `default_features_implicit` | Dependency has inline options but no explicit `default-features` declaration |

### Examples

```toml
# Bad - unclear if default features are wanted
serde = { version = "1.0", features = ["derive"] }

# Good - explicit about default features
serde = { version = "1.0", features = ["derive"], default-features = true }
tokio = { version = "1.0", features = ["rt"], default-features = false }
```

### Remediation

Add an explicit `default-features = true` or `default-features = false` to make the intent clear.

### Configuration

```toml
[checks."deps.default_features_explicit"]
enabled = true
severity = "info"
allow = []
```

---

## `deps.no_multiple_versions`

Detects the same crate with different versions across workspace members.

Having multiple versions of the same dependency in a workspace increases binary size and can cause subtle compatibility issues.

> **Note**: This check is **disabled by default**. Enable it explicitly if you want enforcement.

### Codes

| Code | Trigger |
|------|---------|
| `duplicate_different_versions` | Same crate appears with different versions in multiple manifests |

### Examples

```toml
# Bad - crates/a/Cargo.toml
[dependencies]
serde = "1.0.195"

# Bad - crates/b/Cargo.toml
[dependencies]
serde = "1.0.200"

# Good - use workspace inheritance
# Cargo.toml (root)
[workspace.dependencies]
serde = "1.0.200"

# crates/a/Cargo.toml and crates/b/Cargo.toml
[dependencies]
serde.workspace = true
```

### Remediation

Define the dependency in `[workspace.dependencies]` and use workspace inheritance in all members.

### Configuration

```toml
[checks."deps.no_multiple_versions"]
enabled = true
severity = "warning"
allow = ["proc-macro2"]  # Allow specific crates to have multiple versions
```

---

## `deps.optional_unused`

Detects optional dependencies that aren't referenced in any feature.

When a dependency is marked `optional = true`, it should be activated by at least one feature. An optional dependency not referenced in any feature cannot be enabled by users.

> **Note**: This check is **disabled by default**. Enable it explicitly if you want enforcement.

### Codes

| Code | Trigger |
|------|---------|
| `optional_not_in_features` | Optional dependency not referenced in any feature |

### Examples

```toml
# Bad - optional but no feature uses it
[dependencies]
serde = { version = "1.0", optional = true }

[features]
# No feature references serde

# Good - optional and referenced in feature
[dependencies]
serde = { version = "1.0", optional = true }

[features]
serialization = ["dep:serde"]
```

### Remediation

Either reference the optional dependency in a feature, or remove `optional = true` if it should always be included.

### Configuration

```toml
[checks."deps.optional_unused"]
enabled = true
severity = "warning"
allow = []
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
severity = "warning"  # Downgrade from default "error" (alias: "warn")
```

## See also

- [Configuration](config.md) — Full config reference with profiles
- [Quick Start](quickstart.md) — Getting started guide
- [Troubleshooting](troubleshooting.md) — False positives and allowlists
- [Design Notes](design.md) — Check architecture details
