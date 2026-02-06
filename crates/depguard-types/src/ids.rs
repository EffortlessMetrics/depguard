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
