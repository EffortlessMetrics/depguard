# depguard-check-catalog

Central check metadata and feature-gating for depguard checks.

This crate provides a single source of truth for all check definitions, their default behaviors across profiles, and compile-time feature availability.

## Purpose

The check catalog serves as the authoritative registry for:
- Check IDs and their associated finding codes
- Default severity levels per profile (strict vs warn)
- Compile-time feature gating for optional check inclusion
- BDD feature file coverage mapping

## Key Features

### Check Registry

Each check entry contains:
- **ID**: Canonical check identifier (e.g., `deps.no_wildcards`)
- **Codes**: Finding codes this check can emit
- **Profile Defaults**: Enabled status and severity for strict/warn profiles
- **Feature Gate**: Cargo feature that controls availability
- **BDD Coverage**: Link to the feature file that exercises this check

### Available Checks

| Check ID | Feature | Description |
|----------|---------|-------------|
| `deps.no_wildcards` | `check-no-wildcards` | Disallow wildcard version specs |
| `deps.path_requires_version` | `check-path-requires-version` | Path deps should declare version |
| `deps.path_safety` | `check-path-safety` | No absolute paths or parent escapes |
| `deps.workspace_inheritance` | `check-workspace-inheritance` | Enforce workspace inheritance |
| `deps.git_requires_version` | `check-git-requires-version` | Git deps should declare version |
| `deps.dev_only_in_normal` | `check-dev-only-in-normal` | Dev-only crates in normal deps |
| `deps.default_features_explicit` | `check-default-features-explicit` | Explicit default-features setting |
| `deps.no_multiple_versions` | `check-no-multiple-versions` | No duplicate crate versions |
| `deps.optional_unused` | `check-optional-unused` | Optional deps should be used |
| `deps.yanked_versions` | `check-yanked-versions` | No yanked crate versions |

## Public API

```rust
// Catalog entry type
pub struct CheckCatalogEntry {
    pub id: &'static str,
    pub codes: &'static [&'static str],
    pub strict_enabled: bool,
    pub strict_severity: Severity,
    pub warn_enabled: bool,
    pub warn_severity: Severity,
    pub feature: CheckFeature,
    pub bdd_feature_file: &'static str,
}

// Profile-specific check configuration
pub struct ProfileCheck {
    pub id: &'static str,
    pub enabled: bool,
    pub severity: Severity,
}

// Feature enum for compile-time gating
pub enum CheckFeature {
    NoWildcards,
    PathRequiresVersion,
    PathSafety,
    WorkspaceInheritance,
    GitRequiresVersion,
    DevOnlyInNormal,
    DefaultFeaturesExplicit,
    NoMultipleVersions,
    OptionalUnused,
    YankedVersions,
}

// Access the full catalog
pub const CHECK_CATALOG: &[CheckCatalogEntry];
```

## Feature Flags

All checks are enabled by default. Individual checks can be disabled at compile time by disabling their feature:

```toml
[dependencies]
depguard-check-catalog = { version = "0.1.0", default-features = false, features = [
    "check-no-wildcards",
    "check-path-safety",
    # ... only include needed checks
] }
```

## Usage Example

```rust
use depguard_check_catalog::{CHECK_CATALOG, ProfileCheck};

// Iterate all available checks
for entry in CHECK_CATALOG {
    println!("Check: {} -> {:?}", entry.id, entry.codes);
}

// Get strict profile defaults
let strict_defaults: Vec<ProfileCheck> = CHECK_CATALOG
    .iter()
    .map(|e| ProfileCheck {
        id: e.id,
        enabled: e.strict_enabled,
        severity: e.strict_severity,
    })
    .collect();
```

## Dependencies

| Crate | Purpose |
|-------|---------|
| `depguard-types` | Severity enum and ID constants |

## Related Crates

- [`depguard-settings`](../depguard-settings/) - Uses catalog for profile resolution
- [`depguard-domain-checks`](../depguard-domain-checks/) - Implements the check logic
- [`depguard-types`](../depguard-types/) - ID constants and types
