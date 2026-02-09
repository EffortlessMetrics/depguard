//! Stable identifiers for checks and finding codes.
//!
//! `check_id` is a dotted namespace. `code` is a short snake_case discriminator.

// Checks
pub const CHECK_DEPS_NO_WILDCARDS: &str = "deps.no_wildcards";
pub const CHECK_DEPS_PATH_REQUIRES_VERSION: &str = "deps.path_requires_version";
pub const CHECK_DEPS_PATH_SAFETY: &str = "deps.path_safety";
pub const CHECK_DEPS_WORKSPACE_INHERITANCE: &str = "deps.workspace_inheritance";
pub const CHECK_DEPS_GIT_REQUIRES_VERSION: &str = "deps.git_requires_version";
pub const CHECK_DEPS_DEV_ONLY_IN_NORMAL: &str = "deps.dev_only_in_normal";
pub const CHECK_DEPS_DEFAULT_FEATURES_EXPLICIT: &str = "deps.default_features_explicit";
pub const CHECK_DEPS_NO_MULTIPLE_VERSIONS: &str = "deps.no_multiple_versions";
pub const CHECK_DEPS_OPTIONAL_UNUSED: &str = "deps.optional_unused";

// Codes: deps.no_wildcards
pub const CODE_WILDCARD_VERSION: &str = "wildcard_version";

// Codes: deps.path_requires_version
pub const CODE_PATH_WITHOUT_VERSION: &str = "path_without_version";

// Codes: deps.path_safety
pub const CODE_ABSOLUTE_PATH: &str = "absolute_path";
pub const CODE_PARENT_ESCAPE: &str = "parent_escape";

// Codes: deps.workspace_inheritance
pub const CODE_MISSING_WORKSPACE_TRUE: &str = "missing_workspace_true";

// Codes: deps.git_requires_version
pub const CODE_GIT_WITHOUT_VERSION: &str = "git_without_version";

// Codes: deps.dev_only_in_normal
pub const CODE_DEV_DEP_IN_NORMAL: &str = "dev_dep_in_normal";

// Codes: deps.default_features_explicit
pub const CODE_DEFAULT_FEATURES_IMPLICIT: &str = "default_features_implicit";

// Codes: deps.no_multiple_versions
pub const CODE_DUPLICATE_DIFFERENT_VERSIONS: &str = "duplicate_different_versions";

// Codes: deps.optional_unused
pub const CODE_OPTIONAL_NOT_IN_FEATURES: &str = "optional_not_in_features";

// Tool-level
pub const CHECK_TOOL_RUNTIME: &str = "tool.runtime";
pub const CODE_RUNTIME_ERROR: &str = "runtime_error";

// Capability reason tokens (snake_case, for No Green By Omission reporting)
pub const REASON_DIFF_SCOPE_DISABLED: &str = "diff_scope_disabled";
pub const REASON_CONFIG_MISSING_DEFAULTED: &str = "config_missing_defaulted";
pub const REASON_RUNTIME_ERROR: &str = "runtime_error";
pub const REASON_NO_MANIFEST_FOUND: &str = "no_manifest_found";

// Fix action tokens (stable machine-readable routing for actuators)
pub const FIX_ACTION_PIN_VERSION: &str = "pin_version";
pub const FIX_ACTION_ADD_VERSION: &str = "add_version";
pub const FIX_ACTION_USE_REPO_RELATIVE_PATH: &str = "use_repo_relative_path";
pub const FIX_ACTION_REMOVE_PARENT_ESCAPE: &str = "remove_parent_escape";
pub const FIX_ACTION_USE_WORKSPACE_TRUE: &str = "use_workspace_true";
pub const FIX_ACTION_ADD_VERSION_WITH_GIT: &str = "add_version_with_git";
pub const FIX_ACTION_MOVE_TO_DEV_DEPS: &str = "move_to_dev_deps";
pub const FIX_ACTION_ADD_DEFAULT_FEATURES: &str = "add_default_features";
pub const FIX_ACTION_ALIGN_WORKSPACE_VERSIONS: &str = "align_workspace_versions";
pub const FIX_ACTION_RESOLVE_OPTIONAL_FEATURE: &str = "resolve_optional_feature";

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn ids_are_non_empty_and_unique_within_groups() {
        let check_ids = vec![
            CHECK_DEPS_NO_WILDCARDS,
            CHECK_DEPS_PATH_REQUIRES_VERSION,
            CHECK_DEPS_PATH_SAFETY,
            CHECK_DEPS_WORKSPACE_INHERITANCE,
            CHECK_DEPS_GIT_REQUIRES_VERSION,
            CHECK_DEPS_DEV_ONLY_IN_NORMAL,
            CHECK_DEPS_DEFAULT_FEATURES_EXPLICIT,
            CHECK_DEPS_NO_MULTIPLE_VERSIONS,
            CHECK_DEPS_OPTIONAL_UNUSED,
            CHECK_TOOL_RUNTIME,
        ];
        let codes = vec![
            CODE_WILDCARD_VERSION,
            CODE_PATH_WITHOUT_VERSION,
            CODE_ABSOLUTE_PATH,
            CODE_PARENT_ESCAPE,
            CODE_MISSING_WORKSPACE_TRUE,
            CODE_GIT_WITHOUT_VERSION,
            CODE_DEV_DEP_IN_NORMAL,
            CODE_DEFAULT_FEATURES_IMPLICIT,
            CODE_DUPLICATE_DIFFERENT_VERSIONS,
            CODE_OPTIONAL_NOT_IN_FEATURES,
            CODE_RUNTIME_ERROR,
        ];
        let reasons = vec![
            REASON_DIFF_SCOPE_DISABLED,
            REASON_CONFIG_MISSING_DEFAULTED,
            REASON_RUNTIME_ERROR,
            REASON_NO_MANIFEST_FOUND,
        ];
        let fix_actions = vec![
            FIX_ACTION_PIN_VERSION,
            FIX_ACTION_ADD_VERSION,
            FIX_ACTION_USE_REPO_RELATIVE_PATH,
            FIX_ACTION_REMOVE_PARENT_ESCAPE,
            FIX_ACTION_USE_WORKSPACE_TRUE,
            FIX_ACTION_ADD_VERSION_WITH_GIT,
            FIX_ACTION_MOVE_TO_DEV_DEPS,
            FIX_ACTION_ADD_DEFAULT_FEATURES,
            FIX_ACTION_ALIGN_WORKSPACE_VERSIONS,
            FIX_ACTION_RESOLVE_OPTIONAL_FEATURE,
        ];

        for id in check_ids
            .iter()
            .chain(codes.iter())
            .chain(reasons.iter())
            .chain(fix_actions.iter())
        {
            assert!(!id.is_empty());
        }

        fn assert_unique(values: &[&str]) {
            let unique: HashSet<&str> = values.iter().copied().collect();
            assert_eq!(unique.len(), values.len());
        }

        assert_unique(&check_ids);
        assert_unique(&codes);
        assert_unique(&reasons);
        assert_unique(&fix_actions);
    }
}
