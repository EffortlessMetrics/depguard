//! Explain registry for checks and codes.
//!
//! Maps check IDs and codes to human-readable explanations with remediation guidance.

use crate::ids;

/// Explanation entry for a check or code.
#[derive(Debug, Clone)]
pub struct Explanation {
    /// Short description of the check/code.
    pub title: &'static str,
    /// What the check does and why it exists.
    pub description: &'static str,
    /// How to fix violations.
    pub remediation: &'static str,
    /// Before/after code examples.
    pub examples: ExamplePair,
}

/// Before and after code examples.
#[derive(Debug, Clone)]
pub struct ExamplePair {
    /// Code that would trigger a finding.
    pub before: &'static str,
    /// Code that passes the check.
    pub after: &'static str,
}

/// Look up an explanation by check_id or code.
///
/// Returns `None` if the identifier is not recognized.
pub fn lookup_explanation(identifier: &str) -> Option<Explanation> {
    // Try check_id first, then code
    match identifier {
        // Check IDs
        ids::CHECK_DEPS_NO_WILDCARDS => Some(explain_no_wildcards()),
        ids::CHECK_DEPS_PATH_REQUIRES_VERSION => Some(explain_path_requires_version()),
        ids::CHECK_DEPS_PATH_SAFETY => Some(explain_path_safety()),
        ids::CHECK_DEPS_WORKSPACE_INHERITANCE => Some(explain_workspace_inheritance()),

        // Codes
        ids::CODE_WILDCARD_VERSION => Some(explain_wildcard_version()),
        ids::CODE_PATH_WITHOUT_VERSION => Some(explain_path_without_version()),
        ids::CODE_ABSOLUTE_PATH => Some(explain_absolute_path()),
        ids::CODE_PARENT_ESCAPE => Some(explain_parent_escape()),
        ids::CODE_MISSING_WORKSPACE_TRUE => Some(explain_missing_workspace_true()),

        _ => None,
    }
}

/// List all known check IDs.
pub fn all_check_ids() -> &'static [&'static str] {
    &[
        ids::CHECK_DEPS_NO_WILDCARDS,
        ids::CHECK_DEPS_PATH_REQUIRES_VERSION,
        ids::CHECK_DEPS_PATH_SAFETY,
        ids::CHECK_DEPS_WORKSPACE_INHERITANCE,
    ]
}

/// List all known codes.
pub fn all_codes() -> &'static [&'static str] {
    &[
        ids::CODE_WILDCARD_VERSION,
        ids::CODE_PATH_WITHOUT_VERSION,
        ids::CODE_ABSOLUTE_PATH,
        ids::CODE_PARENT_ESCAPE,
        ids::CODE_MISSING_WORKSPACE_TRUE,
    ]
}

// --- Check-level explanations ---

fn explain_no_wildcards() -> Explanation {
    Explanation {
        title: "No Wildcard Versions",
        description: "\
Detects dependencies declared with wildcard version requirements like `*` or `1.*`.

Wildcard versions are problematic because:
- They allow any version to be selected, including breaking changes
- Builds are not reproducible across different points in time
- Security vulnerabilities in newer versions may be pulled in unknowingly
- cargo publish rejects crates with wildcard dependencies",
        remediation: "\
Replace wildcard versions with explicit semver requirements:
- Use `^1.2.3` (caret, default) for compatible updates within the same major version
- Use `~1.2.3` (tilde) for patch-level updates only
- Use `=1.2.3` for an exact version pin
- Use `>=1.2.0, <2.0.0` for explicit version ranges",
        examples: ExamplePair {
            before: r#"[dependencies]
serde = "*"
tokio = "1.*""#,
            after: r#"[dependencies]
serde = "1.0"
tokio = "1.35""#,
        },
    }
}

fn explain_path_requires_version() -> Explanation {
    Explanation {
        title: "Path Dependencies Require Version",
        description: "\
Detects path dependencies in publishable crates that lack an explicit version.

When publishing a crate to crates.io, Cargo ignores the `path` key and uses only
the version from the registry. If no version is specified:
- The crate cannot be published (cargo publish will fail)
- Users who depend on your crate won't be able to build it

This check only applies to crates that can be published (publish != false).",
        remediation: "\
Add an explicit version alongside the path:

    my-crate = { path = \"../my-crate\", version = \"0.1.0\" }

Alternatively, use workspace inheritance:

    my-crate.workspace = true

Or mark the crate as unpublishable in its Cargo.toml:

    [package]
    publish = false",
        examples: ExamplePair {
            before: r#"[dependencies]
my-lib = { path = "../my-lib" }"#,
            after: r#"[dependencies]
my-lib = { path = "../my-lib", version = "0.1.0" }

# Or use workspace inheritance:
my-lib.workspace = true"#,
        },
    }
}

fn explain_path_safety() -> Explanation {
    Explanation {
        title: "Path Dependency Safety",
        description: "\
Detects path dependencies that use absolute paths or escape the repository root.

This check flags two issues:
1. Absolute paths (e.g., `/home/user/code/lib` or `C:\\Code\\lib`)
2. Parent references (`..`) that escape outside the repository root

Both patterns cause problems:
- Absolute paths are machine-specific and not portable
- Escaping the repo root means the dependency is not version-controlled with the project
- CI/CD builds will fail when paths don't exist on the build machine
- Other contributors cannot build the project without identical directory layouts",
        remediation: "\
Use repo-relative paths that stay within the repository:

    my-crate = { path = \"../sibling-crate\" }  # OK if still in repo
    my-crate = { path = \"crates/my-crate\" }   # Always OK

If you need an external dependency:
- Publish it to crates.io or a private registry
- Use a git dependency with a URL
- Move the dependency into the workspace",
        examples: ExamplePair {
            before: r#"[dependencies]
# Absolute path - not portable
my-lib = { path = "/home/user/code/my-lib" }

# Escapes repo root
other-lib = { path = "../../../outside-repo/lib" }"#,
            after: r#"[dependencies]
# Repo-relative path
my-lib = { path = "../my-lib" }

# Or use a git/registry dependency for external code
other-lib = { git = "https://github.com/org/other-lib" }"#,
        },
    }
}

fn explain_workspace_inheritance() -> Explanation {
    Explanation {
        title: "Workspace Dependency Inheritance",
        description: "\
Detects dependencies that exist in [workspace.dependencies] but are not using
`workspace = true` inheritance.

When a workspace defines shared dependencies in [workspace.dependencies], member
crates should inherit them to ensure:
- Consistent versions across all workspace crates
- Single source of truth for dependency versions
- Easier bulk updates when upgrading dependencies
- Reduced duplication in Cargo.toml files",
        remediation: "\
Change the dependency declaration to use workspace inheritance:

    # In member crate's Cargo.toml
    [dependencies]
    serde.workspace = true

You can still add local features while inheriting the version:

    serde = { workspace = true, features = [\"derive\"] }

If you intentionally need a different version, add the dependency to the
check's allow list in depguard.toml.",
        examples: ExamplePair {
            before: r#"# In Cargo.toml (workspace root)
[workspace.dependencies]
serde = "1.0"

# In crates/my-crate/Cargo.toml
[dependencies]
serde = "1.0"  # Duplicates workspace definition"#,
            after: r#"# In Cargo.toml (workspace root)
[workspace.dependencies]
serde = "1.0"

# In crates/my-crate/Cargo.toml
[dependencies]
serde.workspace = true

# Or with additional features:
serde = { workspace = true, features = ["derive"] }"#,
        },
    }
}

// --- Code-level explanations ---

fn explain_wildcard_version() -> Explanation {
    // Same as the check, but framed as the specific code
    let mut exp = explain_no_wildcards();
    exp.title = "Wildcard Version";
    exp
}

fn explain_path_without_version() -> Explanation {
    let mut exp = explain_path_requires_version();
    exp.title = "Path Without Version";
    exp
}

fn explain_absolute_path() -> Explanation {
    Explanation {
        title: "Absolute Path Dependency",
        description: "\
A dependency is declared with an absolute filesystem path.

Absolute paths like `/home/user/code/lib` or `C:\\Code\\lib` are:
- Machine-specific and not portable across systems
- Not reproducible in CI/CD environments
- Not shareable with other contributors
- A potential security concern (may leak host directory structure)",
        remediation: "\
Convert to a repo-relative path:

    my-crate = { path = \"../my-crate\" }

Or use a published/git dependency:

    my-crate = \"1.0\"
    my-crate = { git = \"https://github.com/org/my-crate\" }",
        examples: ExamplePair {
            before: r#"[dependencies]
my-lib = { path = "/home/user/projects/my-lib" }
win-lib = { path = "C:\\Code\\win-lib" }"#,
            after: r#"[dependencies]
my-lib = { path = "../my-lib" }
win-lib = { path = "../win-lib" }"#,
        },
    }
}

fn explain_parent_escape() -> Explanation {
    Explanation {
        title: "Path Escapes Repository Root",
        description: "\
A path dependency uses `..` segments that navigate outside the repository root.

This typically happens when:
- A dependency lives in a sibling directory outside the repo
- The path was copied from another project with different structure
- A monorepo was split but paths weren't updated

Dependencies outside the repository:
- Are not version-controlled with the project
- Won't exist on CI/CD machines
- Cannot be cloned by other contributors
- Break the principle of self-contained repositories",
        remediation: "\
Move the dependency into the workspace, or use an external reference:

1. Move into workspace:
   mv ../external-lib crates/external-lib
   # Update path to: { path = \"crates/external-lib\" }

2. Use git dependency:
   external-lib = { git = \"https://github.com/org/external-lib\" }

3. Publish to a registry:
   external-lib = \"1.0\"",
        examples: ExamplePair {
            before: r#"# From crates/my-app/Cargo.toml
[dependencies]
# Escapes repo: crates/my-app -> crates -> repo-root -> ??? (outside!)
shared = { path = "../../../shared-libs/common" }"#,
            after: r#"# Move shared into the workspace, then:
[dependencies]
shared = { path = "../shared" }

# Or use a git/registry dependency:
shared = { git = "https://github.com/org/shared-libs", subdirectory = "common" }"#,
        },
    }
}

fn explain_missing_workspace_true() -> Explanation {
    let mut exp = explain_workspace_inheritance();
    exp.title = "Missing workspace = true";
    exp
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_by_check_id() {
        assert!(lookup_explanation(ids::CHECK_DEPS_NO_WILDCARDS).is_some());
        assert!(lookup_explanation(ids::CHECK_DEPS_PATH_REQUIRES_VERSION).is_some());
        assert!(lookup_explanation(ids::CHECK_DEPS_PATH_SAFETY).is_some());
        assert!(lookup_explanation(ids::CHECK_DEPS_WORKSPACE_INHERITANCE).is_some());
    }

    #[test]
    fn lookup_by_code() {
        assert!(lookup_explanation(ids::CODE_WILDCARD_VERSION).is_some());
        assert!(lookup_explanation(ids::CODE_PATH_WITHOUT_VERSION).is_some());
        assert!(lookup_explanation(ids::CODE_ABSOLUTE_PATH).is_some());
        assert!(lookup_explanation(ids::CODE_PARENT_ESCAPE).is_some());
        assert!(lookup_explanation(ids::CODE_MISSING_WORKSPACE_TRUE).is_some());
    }

    #[test]
    fn lookup_unknown_returns_none() {
        assert!(lookup_explanation("unknown.check").is_none());
        assert!(lookup_explanation("unknown_code").is_none());
    }

    #[test]
    fn all_check_ids_are_valid() {
        for id in all_check_ids() {
            assert!(
                lookup_explanation(id).is_some(),
                "check_id {} should be in registry",
                id
            );
        }
    }

    #[test]
    fn all_codes_are_valid() {
        for code in all_codes() {
            assert!(
                lookup_explanation(code).is_some(),
                "code {} should be in registry",
                code
            );
        }
    }
}
