//! Stable identifiers for checks and finding codes.
//!
//! `check_id` is a dotted namespace. `code` is a short snake_case discriminator.

// Checks
pub const CHECK_DEPS_NO_WILDCARDS: &str = "deps.no_wildcards";
pub const CHECK_DEPS_PATH_REQUIRES_VERSION: &str = "deps.path_requires_version";
pub const CHECK_DEPS_PATH_SAFETY: &str = "deps.path_safety";
pub const CHECK_DEPS_WORKSPACE_INHERITANCE: &str = "deps.workspace_inheritance";

// Codes: deps.no_wildcards
pub const CODE_WILDCARD_VERSION: &str = "wildcard_version";

// Codes: deps.path_requires_version
pub const CODE_PATH_WITHOUT_VERSION: &str = "path_without_version";

// Codes: deps.path_safety
pub const CODE_ABSOLUTE_PATH: &str = "absolute_path";
pub const CODE_PARENT_ESCAPE: &str = "parent_escape";

// Codes: deps.workspace_inheritance
pub const CODE_MISSING_WORKSPACE_TRUE: &str = "missing_workspace_true";

// Tool-level
pub const CHECK_TOOL_RUNTIME: &str = "tool.runtime";
pub const CODE_RUNTIME_ERROR: &str = "runtime_error";
