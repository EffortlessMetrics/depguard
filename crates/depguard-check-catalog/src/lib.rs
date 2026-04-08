//! Check catalog metadata and feature-gating metadata.
//!
//! This crate owns the check table used by settings and runtime feature gates.

#![forbid(unsafe_code)]

use depguard_types::{Severity, ids};

#[derive(Clone, Copy, Debug)]
pub struct CheckCatalogEntry {
    pub id: &'static str,
    /// Canonical finding codes emitted by this check.
    pub codes: &'static [&'static str],
    /// Whether the check is on by default for the strict profile.
    pub strict_enabled: bool,
    /// Strict-profile default severity.
    pub strict_severity: Severity,
    /// Whether the check is on by default for the warn/compat profiles.
    pub warn_enabled: bool,
    /// Warn-profile default severity.
    pub warn_severity: Severity,
    /// Owning cargo feature gate for this check.
    pub feature: CheckFeature,
    /// Primary BDD feature file that should exercise this check.
    pub bdd_feature_file: &'static str,
}

#[derive(Clone, Copy, Debug)]
pub struct ProfileCheck {
    pub id: &'static str,
    pub enabled: bool,
    pub severity: Severity,
}

#[derive(Clone, Copy, Debug)]
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

const CHECK_CATALOG: &[CheckCatalogEntry] = &[
    CheckCatalogEntry {
        id: ids::CHECK_DEPS_NO_WILDCARDS,
        codes: &[ids::CODE_WILDCARD_VERSION],
        strict_enabled: true,
        strict_severity: Severity::Error,
        warn_enabled: true,
        warn_severity: Severity::Warning,
        feature: CheckFeature::NoWildcards,
        bdd_feature_file: "rules_no_wildcards.feature",
    },
    CheckCatalogEntry {
        id: ids::CHECK_DEPS_PATH_REQUIRES_VERSION,
        codes: &[ids::CODE_PATH_WITHOUT_VERSION],
        strict_enabled: true,
        strict_severity: Severity::Error,
        warn_enabled: true,
        warn_severity: Severity::Warning,
        feature: CheckFeature::PathRequiresVersion,
        bdd_feature_file: "rules_path_requires_version.feature",
    },
    CheckCatalogEntry {
        id: ids::CHECK_DEPS_PATH_SAFETY,
        codes: &[ids::CODE_ABSOLUTE_PATH, ids::CODE_PARENT_ESCAPE],
        strict_enabled: true,
        strict_severity: Severity::Error,
        warn_enabled: true,
        warn_severity: Severity::Warning,
        feature: CheckFeature::PathSafety,
        bdd_feature_file: "rules_path_safety.feature",
    },
    CheckCatalogEntry {
        id: ids::CHECK_DEPS_WORKSPACE_INHERITANCE,
        codes: &[ids::CODE_MISSING_WORKSPACE_TRUE],
        strict_enabled: false,
        strict_severity: Severity::Error,
        warn_enabled: false,
        warn_severity: Severity::Warning,
        feature: CheckFeature::WorkspaceInheritance,
        bdd_feature_file: "rules_workspace_inheritance.feature",
    },
    CheckCatalogEntry {
        id: ids::CHECK_DEPS_GIT_REQUIRES_VERSION,
        codes: &[ids::CODE_GIT_WITHOUT_VERSION],
        strict_enabled: false,
        strict_severity: Severity::Error,
        warn_enabled: false,
        warn_severity: Severity::Warning,
        feature: CheckFeature::GitRequiresVersion,
        bdd_feature_file: "checks.feature",
    },
    CheckCatalogEntry {
        id: ids::CHECK_DEPS_DEFAULT_FEATURES_EXPLICIT,
        codes: &[ids::CODE_DEFAULT_FEATURES_IMPLICIT],
        strict_enabled: false,
        strict_severity: Severity::Warning,
        warn_enabled: false,
        warn_severity: Severity::Warning,
        feature: CheckFeature::DefaultFeaturesExplicit,
        bdd_feature_file: "checks.feature",
    },
    CheckCatalogEntry {
        id: ids::CHECK_DEPS_NO_MULTIPLE_VERSIONS,
        codes: &[ids::CODE_DUPLICATE_DIFFERENT_VERSIONS],
        strict_enabled: false,
        strict_severity: Severity::Warning,
        warn_enabled: false,
        warn_severity: Severity::Warning,
        feature: CheckFeature::NoMultipleVersions,
        bdd_feature_file: "checks.feature",
    },
    CheckCatalogEntry {
        id: ids::CHECK_DEPS_OPTIONAL_UNUSED,
        codes: &[ids::CODE_OPTIONAL_NOT_IN_FEATURES],
        strict_enabled: false,
        strict_severity: Severity::Warning,
        warn_enabled: false,
        warn_severity: Severity::Warning,
        feature: CheckFeature::OptionalUnused,
        bdd_feature_file: "checks.feature",
    },
    CheckCatalogEntry {
        id: ids::CHECK_DEPS_DEV_ONLY_IN_NORMAL,
        codes: &[ids::CODE_DEV_DEP_IN_NORMAL],
        strict_enabled: false,
        strict_severity: Severity::Warning,
        warn_enabled: false,
        warn_severity: Severity::Warning,
        feature: CheckFeature::DevOnlyInNormal,
        bdd_feature_file: "checks.feature",
    },
    CheckCatalogEntry {
        id: ids::CHECK_DEPS_YANKED_VERSIONS,
        codes: &[ids::CODE_VERSION_YANKED],
        strict_enabled: false,
        strict_severity: Severity::Error,
        warn_enabled: false,
        warn_severity: Severity::Error,
        feature: CheckFeature::YankedVersions,
        bdd_feature_file: "roadmap.feature",
    },
];

impl CheckFeature {
    pub const fn cargo_feature(self) -> &'static str {
        match self {
            Self::NoWildcards => "check-no-wildcards",
            Self::PathRequiresVersion => "check-path-requires-version",
            Self::PathSafety => "check-path-safety",
            Self::WorkspaceInheritance => "check-workspace-inheritance",
            Self::GitRequiresVersion => "check-git-requires-version",
            Self::DevOnlyInNormal => "check-dev-only-in-normal",
            Self::DefaultFeaturesExplicit => "check-default-features-explicit",
            Self::NoMultipleVersions => "check-no-multiple-versions",
            Self::OptionalUnused => "check-optional-unused",
            Self::YankedVersions => "check-yanked-versions",
        }
    }

    pub const fn is_enabled(self) -> bool {
        match self {
            Self::NoWildcards => cfg!(feature = "check-no-wildcards"),
            Self::PathRequiresVersion => cfg!(feature = "check-path-requires-version"),
            Self::PathSafety => cfg!(feature = "check-path-safety"),
            Self::WorkspaceInheritance => cfg!(feature = "check-workspace-inheritance"),
            Self::GitRequiresVersion => cfg!(feature = "check-git-requires-version"),
            Self::DevOnlyInNormal => cfg!(feature = "check-dev-only-in-normal"),
            Self::DefaultFeaturesExplicit => cfg!(feature = "check-default-features-explicit"),
            Self::NoMultipleVersions => cfg!(feature = "check-no-multiple-versions"),
            Self::OptionalUnused => cfg!(feature = "check-optional-unused"),
            Self::YankedVersions => cfg!(feature = "check-yanked-versions"),
        }
    }
}

pub fn catalog() -> &'static [CheckCatalogEntry] {
    CHECK_CATALOG
}

pub fn is_known_check_id(check_id: &str) -> bool {
    CHECK_CATALOG.iter().any(|entry| entry.id == check_id)
}

pub fn all_check_ids() -> Vec<&'static str> {
    CHECK_CATALOG.iter().map(|entry| entry.id).collect()
}

pub fn all_codes() -> Vec<&'static str> {
    CHECK_CATALOG
        .iter()
        .flat_map(|entry| entry.codes.iter().copied())
        .collect()
}

pub fn is_check_available(check_id: &str) -> bool {
    entry(check_id).is_some_and(|entry| entry.feature.is_enabled())
}

pub fn entry(check_id: &str) -> Option<&'static CheckCatalogEntry> {
    CHECK_CATALOG.iter().find(|entry| entry.id == check_id)
}

pub fn feature_name(check_id: &str) -> Option<&'static str> {
    entry(check_id).map(|entry| entry.feature.cargo_feature())
}

pub fn bdd_feature_file(check_id: &str) -> Option<&'static str> {
    entry(check_id).map(|entry| entry.bdd_feature_file)
}

pub fn checks_for_profile(profile: &str) -> Vec<ProfileCheck> {
    let profile_is_warnish = matches!(profile, "warn" | "team" | "compat" | "oss");

    CHECK_CATALOG
        .iter()
        .map(|entry| {
            if profile_is_warnish {
                ProfileCheck {
                    id: entry.id,
                    enabled: entry.warn_enabled,
                    severity: entry.warn_severity,
                }
            } else {
                ProfileCheck {
                    id: entry.id,
                    enabled: entry.strict_enabled,
                    severity: entry.strict_severity,
                }
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_catalog_ids_have_explanations() {
        for entry in catalog() {
            assert!(
                depguard_types::explain::lookup_explanation(entry.id).is_some(),
                "check id {} has explanation",
                entry.id
            );
        }
    }

    #[test]
    fn strict_and_warn_profiles_cover_all_checks() {
        let strict = checks_for_profile("strict");
        let warn = checks_for_profile("warn");
        assert_eq!(strict.len(), catalog().len());
        assert_eq!(warn.len(), catalog().len());
    }

    #[test]
    fn check_catalog_entries_have_bdd_feature_file() {
        for entry in catalog() {
            assert!(
                !entry.bdd_feature_file.is_empty(),
                "{} must define a BDD feature file",
                entry.id
            );
        }
    }

    #[test]
    fn check_features_default_to_enabled() {
        for entry in catalog() {
            assert!(
                entry.feature.is_enabled(),
                "{} feature should be enabled",
                entry.id
            );
        }
    }
}
