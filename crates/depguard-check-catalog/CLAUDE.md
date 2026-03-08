# CLAUDE.md — depguard-check-catalog

## Purpose

Central check catalog metadata and compile-time feature availability. This crate owns the authoritative registry of all checks, their metadata (IDs, codes, severities), and their feature gates.

## Key Types

| Type | Purpose |
|------|---------|
| [`CheckCatalogEntry`] | Metadata for a single check: ID, codes, profile defaults, feature gate, BDD feature file |
| [`CheckFeature`] | Enum mapping each check to its cargo feature gate |
| [`ProfileCheck`] | Check ID with enabled flag and severity for profile construction |

## Public API

```rust
// Access the full catalog
pub const CHECK_CATALOG: &[CheckCatalogEntry];

// Check if a check is available at runtime (feature gate)
pub fn is_check_available(check_id: &str) -> bool;

// Get catalog entry by ID
pub fn get_catalog_entry(check_id: &str) -> Option<&'static CheckCatalogEntry>;

// Get default checks for a profile
pub fn profile_defaults(profile: &str) -> Vec<ProfileCheck>;
```

## Feature Gates

Each check has a corresponding cargo feature:

| Feature | Check ID |
|---------|----------|
| `check-no-wildcards` | `deps.no_wildcards` |
| `check-path-requires-version` | `deps.path_requires_version` |
| `check-path-safety` | `deps.path_safety` |
| `check-workspace-inheritance` | `deps.workspace_inheritance` |
| `check-git-requires-version` | `deps.git_requires_version` |
| `check-dev-only-in-normal` | `deps.dev_only_in_normal` |
| `check-default-features-explicit` | `deps.default_features_explicit` |
| `check-no-multiple-versions` | `deps.no_multiple_versions` |
| `check-optional-unused` | `deps.optional_unused` |
| `check-yanked-versions` | `deps.yanked_versions` |

All features are enabled by default. Disable them to create minimal builds.

## Profile Defaults

| Check | `strict` | `warn`/`compat` |
|-------|----------|-----------------|
| `no_wildcards` | Error | Warning |
| `path_requires_version` | Error | Warning |
| `path_safety` | Error | Warning |
| `workspace_inheritance` | Disabled | Disabled |
| `git_requires_version` | Disabled | Disabled |
| `dev_only_in_normal` | Warning | Warning |
| `default_features_explicit` | Warning | Warning |
| `no_multiple_versions` | Warning | Warning |
| `optional_unused` | Warning | Warning |
| `yanked_versions` | Warning | Warning |

## Design Constraints

- **No I/O**: Pure data lookups
- **Compile-time gating**: Feature flags control availability
- **Single source of truth**: All check metadata lives here
- **Stable IDs**: Check IDs and codes must never change; deprecate via aliases

## Dependencies

- `depguard-types` — `Severity`, check ID and code constants

## Testing

```bash
cargo test -p depguard-check-catalog
```

Tests verify catalog consistency (all entries have valid IDs, codes, and feature mappings).
