use anyhow::Context;
use camino::{Utf8Path, Utf8PathBuf};
use depguard_types::RepoPath;
use globset::{Glob, GlobSetBuilder};
use std::path::PathBuf;
use toml_edit::DocumentMut;
use walkdir::WalkDir;

/// Represents a parsed member pattern with its exclusion flag.
#[derive(Debug, Clone)]
struct MemberPattern {
    /// The glob pattern (without the `!` prefix if present)
    pattern: String,
    /// Whether this pattern excludes previously matched paths
    is_exclusion: bool,
}

/// Discover Cargo manifests for the workspace rooted at `repo_root`.
///
/// Behavior:
/// - If the root manifest has `[workspace]`, expand `members` (with glob support) and apply `exclude`.
/// - Supports exclusion patterns in members list (patterns starting with `!`).
/// - Otherwise, return only `Cargo.toml` (single crate repository).
///
/// # Cargo-Compatible Glob Semantics
///
/// This implementation follows Cargo's workspace member glob behavior:
///
/// - **Double-star (`**`)**: Matches zero or more directory components.
///   Example: `crates/**` matches `crates/a`, `crates/foo/bar`, etc.
///
/// - **Exclusion patterns**: Patterns starting with `!` in the members list
///   exclude previously matched paths. These are processed in order.
///   Example: `["crates/*", "!crates/excluded"]` includes all crates except `excluded`.
///
/// - **Empty member lists**: When `[workspace]` is present with no members,
///   all `Cargo.toml` files in the repository are included.
///
/// - **Non-existent paths**: Patterns that match no files are silently ignored.
///
/// - **Relative path normalization**: Both `./path` and `path` forms are handled.
///
/// # Known Deviations from Cargo
///
/// - **Circular workspace references**: Not currently detected. Cargo handles these
///   by erroring during resolution. This implementation may include nested workspaces.
///
/// - **Default members**: The `default-members` field is not honored during discovery.
///   All matched members are included regardless of `default-members`.
pub fn discover_manifests(repo_root: &Utf8Path) -> anyhow::Result<Vec<RepoPath>> {
    let root = repo_root.join("Cargo.toml");
    let text = std::fs::read_to_string(&root).with_context(|| format!("read {}", root))?;
    let doc = text
        .parse::<DocumentMut>()
        .context("parse root Cargo.toml")?;

    let workspace = doc.get("workspace");
    if workspace.is_none() {
        return Ok(vec![RepoPath::new("Cargo.toml")]);
    }

    let member_patterns: Vec<MemberPattern> = doc
        .get("workspace")
        .and_then(|w| w.get("members"))
        .and_then(|i| i.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().and_then(parse_member_pattern))
                .collect()
        })
        .unwrap_or_default();

    let excludes: Vec<String> = doc
        .get("workspace")
        .and_then(|w| w.get("exclude"))
        .and_then(|i| i.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|v| {
                    v.as_str()
                        .map(|s| s.trim())
                        .filter(|s| !s.is_empty())
                        .map(normalize_path)
                })
                .collect()
        })
        .unwrap_or_default();

    // Separate inclusion and exclusion patterns from members list
    let (include_patterns, exclude_from_members): (Vec<_>, Vec<_>) =
        member_patterns.iter().partition(|p| !p.is_exclusion);

    let include_patterns: Vec<String> = include_patterns
        .into_iter()
        .map(|p| p.pattern.clone())
        .collect();
    let exclude_from_members: Vec<String> = exclude_from_members
        .into_iter()
        .map(|p| p.pattern.clone())
        .collect();

    // Combine explicit excludes with exclusion patterns from members
    let all_excludes: Vec<String> = excludes.into_iter().chain(exclude_from_members).collect();

    let member_set = build_globset(&include_patterns).context("compile members globset")?;
    let exclude_set = build_globset(&all_excludes).context("compile exclude globset")?;

    let mut out: Vec<RepoPath> = Vec::new();
    out.push(RepoPath::new("Cargo.toml"));

    for abs in WalkDir::new(repo_root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file() && e.file_name() == "Cargo.toml")
        .filter_map(|e| pathbuf_to_utf8(e.path().to_path_buf()))
    {
        let rel = abs
            .strip_prefix(repo_root)
            .unwrap_or(&abs)
            .as_str()
            .replace('\\', "/");
        if rel == "Cargo.toml" {
            continue;
        }

        // Skip virtual workspaces (Cargo behavior)
        // Virtual workspaces have [workspace] but no [package] and represent workspace boundaries
        // Package workspaces (with both [package] and [workspace]) are included as members
        if is_nested_workspace(&abs) {
            continue;
        }

        // Match both the file path and its parent directory against globs.
        let dir_rel = Utf8Path::new(&rel)
            .parent()
            .map(|p| p.as_str())
            .unwrap_or("");

        // Empty member list means include all (Cargo behavior)
        let is_member = include_patterns.is_empty()
            || member_set.is_match(&rel)
            || member_set.is_match(dir_rel);

        let is_excluded = exclude_set.is_match(&rel) || exclude_set.is_match(dir_rel);

        if is_member && !is_excluded {
            out.push(RepoPath::new(&rel));
        }
    }

    // Stable order.
    out.sort();
    out.dedup();

    Ok(out)
}

/// Parse a member pattern, detecting exclusion prefix.
///
/// Cargo supports patterns starting with `!` to exclude previously matched paths.
///
/// # Cargo Edge Cases Handled
///
/// - **Parent directory references**: Patterns containing `../` are rejected
/// - **Absolute paths**: Absolute paths are rejected
/// - **Empty patterns**: Empty patterns after trimming are ignored
fn parse_member_pattern(s: &str) -> Option<MemberPattern> {
    let trimmed = s.trim();

    // Reject empty patterns
    if trimmed.is_empty() {
        return None;
    }

    // Handle exclusion prefix
    let (pattern_str, is_exclusion) = if let Some(stripped) = trimmed.strip_prefix('!') {
        (stripped.trim_start(), true)
    } else {
        (trimmed, false)
    };

    // Reject empty pattern after stripping exclusion prefix
    if pattern_str.is_empty() {
        return None;
    }

    // Reject absolute paths (Cargo behavior)
    // Unix: starts with /
    // Windows: starts with /, \, or drive letter (e.g., C:\)
    if pattern_str.starts_with('/') || pattern_str.starts_with('\\') {
        return None;
    }

    #[cfg(windows)]
    {
        // Check for Windows drive letter paths (e.g., C:\, D:/)
        if pattern_str.len() >= 2 && pattern_str.as_bytes()[1] == b':' {
            let first_char = pattern_str.chars().next().unwrap();
            if first_char.is_ascii_alphabetic() {
                return None;
            }
        }
    }

    // Reject parent directory references (Cargo behavior)
    // Patterns like "../other-crate" or "crates/../../other" are not allowed
    if pattern_str.contains("../") || pattern_str.contains("..\\") {
        return None;
    }

    Some(MemberPattern {
        pattern: normalize_path(pattern_str),
        is_exclusion,
    })
}

/// Normalize a path by removing leading `./` or `.\` prefix.
///
/// Cargo treats `./path` and `path` equivalently in workspace members.
fn normalize_path(s: &str) -> String {
    let trimmed = s.trim();
    if let Some(stripped) = trimmed.strip_prefix("./") {
        stripped.to_string()
    } else if let Some(stripped) = trimmed.strip_prefix(".\\") {
        stripped.to_string()
    } else {
        trimmed.to_string()
    }
}

fn build_globset(patterns: &[String]) -> anyhow::Result<globset::GlobSet> {
    let mut b = GlobSetBuilder::new();
    for p in patterns {
        // Skip empty patterns
        if p.is_empty() {
            continue;
        }
        // Cargo workspace globs are relative paths.
        // The globset crate handles `**` (globstar) patterns correctly.
        // Validate that patterns don't contain parent directory references
        if p.contains("../") || p.contains("..\\") {
            continue;
        }
        b.add(Glob::new(p)?);
    }
    Ok(b.build()?)
}

fn pathbuf_to_utf8(path: PathBuf) -> Option<Utf8PathBuf> {
    Utf8PathBuf::from_path_buf(path).ok()
}

/// Check if a Cargo.toml file defines a nested virtual workspace.
///
/// Cargo only includes members from the root workspace, not from nested workspaces.
/// However, a "package workspace" (a manifest with both [package] and [workspace])
/// is still included as a member. Only virtual workspaces (with [workspace] but no [package])
/// are excluded.
///
/// Returns true if this is a virtual workspace (has [workspace] but no [package]).
fn is_nested_workspace(path: &Utf8Path) -> bool {
    match std::fs::read_to_string(path) {
        Ok(content) => {
            // Parse the TOML and check for workspace section
            if let Ok(doc) = content.parse::<DocumentMut>() {
                let has_workspace = doc.get("workspace").is_some();
                let has_package = doc.get("package").is_some();
                // Virtual workspace: has [workspace] but no [package]
                has_workspace && !has_package
            } else {
                false
            }
        }
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn utf8_root(tmp: &TempDir) -> Utf8PathBuf {
        Utf8PathBuf::from_path_buf(tmp.path().to_path_buf()).expect("utf8 path")
    }

    fn write_file(path: &Utf8Path, contents: &str) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("create parent");
        }
        std::fs::write(path, contents).expect("write file");
    }

    #[test]
    fn discover_no_workspace_returns_root_only() {
        let tmp = TempDir::new().expect("temp dir");
        let root = utf8_root(&tmp);

        write_file(
            &root.join("Cargo.toml"),
            r#"[package]
name = "solo"
version = "0.1.0"
"#,
        );
        write_file(
            &root.join("crates/a/Cargo.toml"),
            r#"[package]
name = "a"
version = "0.1.0"
"#,
        );

        let manifests = discover_manifests(&root).expect("discover");
        let paths: Vec<&str> = manifests.iter().map(|p| p.as_str()).collect();
        assert_eq!(paths, vec!["Cargo.toml"]);
    }

    #[test]
    fn discover_workspace_members_and_excludes() {
        let tmp = TempDir::new().expect("temp dir");
        let root = utf8_root(&tmp);

        write_file(
            &root.join("Cargo.toml"),
            r#"[workspace]
members = ["crates/*", "tools/**"]
exclude = ["crates/excluded", "tools/skip*"]
"#,
        );
        write_file(
            &root.join("crates/a/Cargo.toml"),
            r#"[package]
name = "a"
version = "0.1.0"
"#,
        );
        write_file(
            &root.join("crates/excluded/Cargo.toml"),
            r#"[package]
name = "excluded"
version = "0.1.0"
"#,
        );
        write_file(
            &root.join("tools/util/Cargo.toml"),
            r#"[package]
name = "util"
version = "0.1.0"
"#,
        );
        write_file(
            &root.join("tools/skip-this/Cargo.toml"),
            r#"[package]
name = "skip"
version = "0.1.0"
"#,
        );

        let manifests = discover_manifests(&root).expect("discover");
        let paths: Vec<&str> = manifests.iter().map(|p| p.as_str()).collect();
        assert_eq!(
            paths,
            vec!["Cargo.toml", "crates/a/Cargo.toml", "tools/util/Cargo.toml"]
        );
    }

    #[test]
    fn pathbuf_to_utf8_rejects_invalid() {
        #[cfg(windows)]
        {
            use std::ffi::OsString;
            use std::os::windows::ffi::OsStringExt;
            let invalid = OsString::from_wide(&[0xD800]);
            let path = PathBuf::from(invalid);
            assert!(pathbuf_to_utf8(path).is_none());
        }

        #[cfg(unix)]
        {
            use std::ffi::OsString;
            use std::os::unix::ffi::OsStringExt;
            let invalid = OsString::from_vec(vec![0xFF, 0xFE, 0xFD]);
            let path = PathBuf::from(invalid);
            assert!(pathbuf_to_utf8(path).is_none());
        }
    }

    #[test]
    fn discover_workspace_with_empty_members_includes_all() {
        let tmp = TempDir::new().expect("temp dir");
        let root = utf8_root(&tmp);

        write_file(&root.join("Cargo.toml"), "[workspace]\n");
        write_file(
            &root.join("crates/a/Cargo.toml"),
            r#"[package]
name = "a"
version = "0.1.0"
"#,
        );
        write_file(
            &root.join("crates/b/Cargo.toml"),
            r#"[package]
name = "b"
version = "0.1.0"
"#,
        );

        let manifests = discover_manifests(&root).expect("discover");
        let paths: Vec<&str> = manifests.iter().map(|p| p.as_str()).collect();
        assert_eq!(
            paths,
            vec!["Cargo.toml", "crates/a/Cargo.toml", "crates/b/Cargo.toml"]
        );
    }

    #[test]
    fn discover_invalid_glob_returns_error() {
        let tmp = TempDir::new().expect("temp dir");
        let root = utf8_root(&tmp);

        write_file(
            &root.join("Cargo.toml"),
            r#"[workspace]
members = ["["]
"#,
        );

        let err = discover_manifests(&root).unwrap_err();
        assert!(err.to_string().contains("compile members globset"));
    }

    // =========================================================================
    // Cargo-Compatible Edge Case Tests
    // =========================================================================

    /// Test double-star (globstar) patterns matching nested directories.
    /// `crates/**` should match `crates/a`, `crates/foo/bar`, etc.
    #[test]
    fn discover_double_star_matches_nested() {
        let tmp = TempDir::new().expect("temp dir");
        let root = utf8_root(&tmp);

        write_file(
            &root.join("Cargo.toml"),
            r#"[workspace]
members = ["crates/**"]
"#,
        );
        write_file(
            &root.join("crates/a/Cargo.toml"),
            r#"[package]
name = "a"
version = "0.1.0"
"#,
        );
        write_file(
            &root.join("crates/foo/bar/Cargo.toml"),
            r#"[package]
name = "bar"
version = "0.1.0"
"#,
        );
        write_file(
            &root.join("crates/deeply/nested/crate/Cargo.toml"),
            r#"[package]
name = "nested"
version = "0.1.0"
"#,
        );
        // This should NOT be included - outside crates/
        write_file(
            &root.join("tools/util/Cargo.toml"),
            r#"[package]
name = "util"
version = "0.1.0"
"#,
        );

        let manifests = discover_manifests(&root).expect("discover");
        let paths: Vec<&str> = manifests.iter().map(|p| p.as_str()).collect();
        assert_eq!(
            paths,
            vec![
                "Cargo.toml",
                "crates/a/Cargo.toml",
                "crates/deeply/nested/crate/Cargo.toml",
                "crates/foo/bar/Cargo.toml"
            ]
        );
    }

    /// Test exclusion patterns in members list (patterns starting with `!`).
    /// Cargo supports `!` prefix to exclude previously matched paths.
    #[test]
    fn discover_exclusion_patterns_in_members() {
        let tmp = TempDir::new().expect("temp dir");
        let root = utf8_root(&tmp);

        write_file(
            &root.join("Cargo.toml"),
            r#"[workspace]
members = ["crates/*", "!crates/excluded"]
"#,
        );
        write_file(
            &root.join("crates/a/Cargo.toml"),
            r#"[package]
name = "a"
version = "0.1.0"
"#,
        );
        write_file(
            &root.join("crates/b/Cargo.toml"),
            r#"[package]
name = "b"
version = "0.1.0"
"#,
        );
        write_file(
            &root.join("crates/excluded/Cargo.toml"),
            r#"[package]
name = "excluded"
version = "0.1.0"
"#,
        );

        let manifests = discover_manifests(&root).expect("discover");
        let paths: Vec<&str> = manifests.iter().map(|p| p.as_str()).collect();
        assert_eq!(
            paths,
            vec!["Cargo.toml", "crates/a/Cargo.toml", "crates/b/Cargo.toml"]
        );
    }

    /// Test relative path normalization: `./path` and `path` are equivalent.
    #[test]
    fn discover_normalizes_relative_paths() {
        let tmp = TempDir::new().expect("temp dir");
        let root = utf8_root(&tmp);

        write_file(
            &root.join("Cargo.toml"),
            r#"[workspace]
members = ["./crates/a", "crates/b"]
exclude = ["./crates/c"]
"#,
        );
        write_file(
            &root.join("crates/a/Cargo.toml"),
            r#"[package]
name = "a"
version = "0.1.0"
"#,
        );
        write_file(
            &root.join("crates/b/Cargo.toml"),
            r#"[package]
name = "b"
version = "0.1.0"
"#,
        );
        write_file(
            &root.join("crates/c/Cargo.toml"),
            r#"[package]
name = "c"
version = "0.1.0"
"#,
        );

        let manifests = discover_manifests(&root).expect("discover");
        let paths: Vec<&str> = manifests.iter().map(|p| p.as_str()).collect();
        assert_eq!(
            paths,
            vec!["Cargo.toml", "crates/a/Cargo.toml", "crates/b/Cargo.toml"]
        );
    }

    /// Test that patterns matching nothing are handled gracefully.
    #[test]
    fn discover_handles_non_matching_patterns() {
        let tmp = TempDir::new().expect("temp dir");
        let root = utf8_root(&tmp);

        write_file(
            &root.join("Cargo.toml"),
            r#"[workspace]
members = ["nonexistent/*", "also-missing/**"]
"#,
        );

        let manifests = discover_manifests(&root).expect("discover");
        let paths: Vec<&str> = manifests.iter().map(|p| p.as_str()).collect();
        // Should only include root manifest when patterns match nothing
        assert_eq!(paths, vec!["Cargo.toml"]);
    }

    /// Test that empty patterns are skipped.
    #[test]
    fn discover_handles_empty_patterns() {
        let tmp = TempDir::new().expect("temp dir");
        let root = utf8_root(&tmp);

        write_file(
            &root.join("Cargo.toml"),
            r#"[workspace]
members = ["", "crates/a", "   "]
"#,
        );
        write_file(
            &root.join("crates/a/Cargo.toml"),
            r#"[package]
name = "a"
version = "0.1.0"
"#,
        );

        let manifests = discover_manifests(&root).expect("discover");
        let paths: Vec<&str> = manifests.iter().map(|p| p.as_str()).collect();
        assert_eq!(paths, vec!["Cargo.toml", "crates/a/Cargo.toml"]);
    }

    /// Test combined exclude field and exclusion patterns in members.
    #[test]
    fn discover_combined_exclude_mechanisms() {
        let tmp = TempDir::new().expect("temp dir");
        let root = utf8_root(&tmp);

        write_file(
            &root.join("Cargo.toml"),
            r#"[workspace]
members = ["crates/*", "!crates/internal"]
exclude = ["crates/excluded"]
"#,
        );
        write_file(
            &root.join("crates/a/Cargo.toml"),
            r#"[package]
name = "a"
version = "0.1.0"
"#,
        );
        write_file(
            &root.join("crates/internal/Cargo.toml"),
            r#"[package]
name = "internal"
version = "0.1.0"
"#,
        );
        write_file(
            &root.join("crates/excluded/Cargo.toml"),
            r#"[package]
name = "excluded"
version = "0.1.0"
"#,
        );

        let manifests = discover_manifests(&root).expect("discover");
        let paths: Vec<&str> = manifests.iter().map(|p| p.as_str()).collect();
        assert_eq!(paths, vec!["Cargo.toml", "crates/a/Cargo.toml"]);
    }

    /// Test that pattern matching is case-sensitive on all platforms.
    /// (Cargo behavior - patterns are case-sensitive)
    #[test]
    fn discover_patterns_are_case_sensitive() {
        let tmp = TempDir::new().expect("temp dir");
        let root = utf8_root(&tmp);

        write_file(
            &root.join("Cargo.toml"),
            r#"[workspace]
members = ["Crates/*"]
"#,
        );
        // Note: On case-insensitive filesystems (Windows/macOS), this may still match
        // but the pattern itself is treated case-sensitively by globset
        write_file(
            &root.join("Crates/A/Cargo.toml"),
            r#"[package]
name = "a"
version = "0.1.0"
"#,
        );
        write_file(
            &root.join("crates/b/Cargo.toml"),
            r#"[package]
name = "b"
version = "0.1.0"
"#,
        );

        let manifests = discover_manifests(&root).expect("discover");
        let paths: Vec<&str> = manifests.iter().map(|p| p.as_str()).collect();
        // Should include Crates/A but not crates/b (case-sensitive match)
        assert!(paths.contains(&"Crates/A/Cargo.toml"));
        assert!(!paths.contains(&"crates/b/Cargo.toml"));
    }

    // =========================================================================
    // Unit tests for helper functions
    // =========================================================================

    #[test]
    fn parse_member_pattern_handles_exclusion() {
        let pattern = parse_member_pattern("!crates/excluded").unwrap();
        assert!(pattern.is_exclusion);
        assert_eq!(pattern.pattern, "crates/excluded");
    }

    #[test]
    fn parse_member_pattern_handles_inclusion() {
        let pattern = parse_member_pattern("crates/*").unwrap();
        assert!(!pattern.is_exclusion);
        assert_eq!(pattern.pattern, "crates/*");
    }

    #[test]
    fn parse_member_pattern_handles_whitespace() {
        let pattern = parse_member_pattern("  !  crates/excluded  ").unwrap();
        assert!(pattern.is_exclusion);
        assert_eq!(pattern.pattern, "crates/excluded");
    }

    #[test]
    fn parse_member_pattern_rejects_empty_patterns() {
        assert!(parse_member_pattern("").is_none());
        assert!(parse_member_pattern("   ").is_none());
        assert!(parse_member_pattern("!").is_none());
        assert!(parse_member_pattern("  !  ").is_none());
    }

    #[test]
    fn parse_member_pattern_rejects_parent_directory_references() {
        // Cargo does not allow parent directory references in workspace members
        assert!(parse_member_pattern("../other-crate").is_none());
        assert!(parse_member_pattern("..\\other-crate").is_none());
        assert!(parse_member_pattern("!../excluded").is_none());
        assert!(parse_member_pattern("crates/../../other").is_none());
        assert!(parse_member_pattern("crates/..\\other").is_none());
    }

    #[test]
    fn parse_member_pattern_rejects_absolute_paths() {
        // Cargo does not allow absolute paths in workspace members
        assert!(parse_member_pattern("/absolute/path").is_none());
        assert!(parse_member_pattern("\\absolute\\path").is_none());
        assert!(parse_member_pattern("!/absolute/excluded").is_none());

        #[cfg(windows)]
        {
            assert!(parse_member_pattern("C:\\absolute\\path").is_none());
            assert!(parse_member_pattern("D:/absolute/path").is_none());
            assert!(parse_member_pattern("e:\\test").is_none());
        }
    }

    #[test]
    fn normalize_path_strips_dot_slash() {
        assert_eq!(normalize_path("./crates/a"), "crates/a");
        assert_eq!(normalize_path(".\\crates\\a"), "crates\\a");
        assert_eq!(normalize_path("crates/a"), "crates/a");
        assert_eq!(normalize_path("  ./crates/a  "), "crates/a");
    }

    // =========================================================================
    // Cargo Edge Case Tests
    // =========================================================================

    /// Test that parent directory references in members are rejected.
    /// Cargo does not allow workspace members to reference directories outside
    /// the workspace root.
    #[test]
    fn discover_rejects_parent_directory_references() {
        let tmp = TempDir::new().expect("temp dir");
        let root = utf8_root(&tmp);

        write_file(
            &root.join("Cargo.toml"),
            r#"[workspace]
members = ["../other-crate", "crates/*"]
"#,
        );
        write_file(
            &root.join("crates/a/Cargo.toml"),
            r#"[package]
name = "a"
version = "0.1.0"
"#,
        );

        let manifests = discover_manifests(&root).expect("discover");
        let paths: Vec<&str> = manifests.iter().map(|p| p.as_str()).collect();
        // Should only include root and valid members, not parent directory reference
        assert_eq!(paths, vec!["Cargo.toml", "crates/a/Cargo.toml"]);
    }

    /// Test that absolute paths in members are rejected.
    /// Cargo only allows relative paths in workspace members.
    #[test]
    fn discover_rejects_absolute_paths() {
        let tmp = TempDir::new().expect("temp dir");
        let root = utf8_root(&tmp);

        write_file(
            &root.join("Cargo.toml"),
            r#"[workspace]
members = ["/absolute/path", "crates/*"]
"#,
        );
        write_file(
            &root.join("crates/a/Cargo.toml"),
            r#"[package]
name = "a"
version = "0.1.0"
"#,
        );

        let manifests = discover_manifests(&root).expect("discover");
        let paths: Vec<&str> = manifests.iter().map(|p| p.as_str()).collect();
        // Should only include root and valid members, not absolute path
        assert_eq!(paths, vec!["Cargo.toml", "crates/a/Cargo.toml"]);
    }

    /// Test that parent directory references in exclude are rejected.
    #[test]
    fn discover_rejects_parent_directory_in_exclude() {
        let tmp = TempDir::new().expect("temp dir");
        let root = utf8_root(&tmp);

        write_file(
            &root.join("Cargo.toml"),
            r#"[workspace]
members = ["crates/*"]
exclude = ["../other-crate", "crates/excluded"]
"#,
        );
        write_file(
            &root.join("crates/a/Cargo.toml"),
            r#"[package]
name = "a"
version = "0.1.0"
"#,
        );
        write_file(
            &root.join("crates/excluded/Cargo.toml"),
            r#"[package]
name = "excluded"
version = "0.1.0"
"#,
        );

        let manifests = discover_manifests(&root).expect("discover");
        let paths: Vec<&str> = manifests.iter().map(|p| p.as_str()).collect();
        // Should exclude crates/excluded but parent directory reference is ignored
        assert_eq!(paths, vec!["Cargo.toml", "crates/a/Cargo.toml"]);
    }

    /// Test that virtual workspaces (workspace without package) are handled correctly.
    #[test]
    fn discover_virtual_workspace() {
        let tmp = TempDir::new().expect("temp dir");
        let root = utf8_root(&tmp);

        write_file(
            &root.join("Cargo.toml"),
            r#"[workspace]
members = ["crates/*"]
"#,
        );
        write_file(
            &root.join("crates/a/Cargo.toml"),
            r#"[package]
name = "a"
version = "0.1.0"
"#,
        );

        let manifests = discover_manifests(&root).expect("discover");
        let paths: Vec<&str> = manifests.iter().map(|p| p.as_str()).collect();
        // Virtual workspace still includes root Cargo.toml
        assert_eq!(paths, vec!["Cargo.toml", "crates/a/Cargo.toml"]);
    }

    /// Test that virtual workspaces are excluded from member discovery.
    /// Virtual workspaces have [workspace] but no [package] section and represent workspace boundaries.
    #[test]
    fn discover_virtual_workspace_is_excluded() {
        let tmp = TempDir::new().expect("temp dir");
        let root = utf8_root(&tmp);

        write_file(
            &root.join("Cargo.toml"),
            r#"[workspace]
members = ["crates/*"]
"#,
        );
        // Virtual workspace - has [workspace] but no [package], should be excluded
        write_file(
            &root.join("crates/virtual/Cargo.toml"),
            r#"[workspace]
members = ["nested/*"]
"#,
        );
        write_file(
            &root.join("crates/virtual/nested/b/Cargo.toml"),
            r#"[package]
name = "b"
version = "0.1.0"
"#,
        );

        let manifests = discover_manifests(&root).expect("discover");
        let paths: Vec<&str> = manifests.iter().map(|p| p.as_str()).collect();
        // Should include root and crates/virtual/nested/b (matches glob pattern)
        // But NOT crates/virtual (virtual workspace is excluded)
        assert!(paths.contains(&"Cargo.toml"));
        assert!(!paths.contains(&"crates/virtual/Cargo.toml"));
        // Note: crates/virtual/nested/b is included because it matches the glob pattern "crates/*"
        // This matches Cargo's behavior - virtual workspaces are excluded, but their members
        // can still be matched by parent workspace patterns
        assert!(paths.contains(&"crates/virtual/nested/b/Cargo.toml"));
    }

    /// Test that package workspaces are included as regular members.
    /// Package workspaces have both [package] and [workspace] sections.
    #[test]
    fn discover_package_workspace_is_included() {
        let tmp = TempDir::new().expect("temp dir");
        let root = utf8_root(&tmp);

        write_file(
            &root.join("Cargo.toml"),
            r#"[workspace]
members = ["crates/*"]
"#,
        );
        // Package workspace - has both [package] and [workspace], should be included
        write_file(
            &root.join("crates/a/Cargo.toml"),
            r#"[package]
name = "a"
version = "0.1.0"

[workspace]
members = ["nested/*"]
"#,
        );
        write_file(
            &root.join("crates/a/nested/b/Cargo.toml"),
            r#"[package]
name = "b"
version = "0.1.0"
"#,
        );

        let manifests = discover_manifests(&root).expect("discover");
        let paths: Vec<&str> = manifests.iter().map(|p| p.as_str()).collect();
        // Should include root, crates/a (package workspace), and crates/a/nested/b
        // Package workspaces are included as regular members
        assert!(paths.contains(&"Cargo.toml"));
        assert!(paths.contains(&"crates/a/Cargo.toml"));
        assert!(paths.contains(&"crates/a/nested/b/Cargo.toml"));
    }

    /// Test that patterns with embedded parent references are rejected.
    #[test]
    fn discover_rejects_embedded_parent_references() {
        let tmp = TempDir::new().expect("temp dir");
        let root = utf8_root(&tmp);

        write_file(
            &root.join("Cargo.toml"),
            r#"[workspace]
members = ["crates/../../other", "crates/*"]
"#,
        );
        write_file(
            &root.join("crates/a/Cargo.toml"),
            r#"[package]
name = "a"
version = "0.1.0"
"#,
        );

        let manifests = discover_manifests(&root).expect("discover");
        let paths: Vec<&str> = manifests.iter().map(|p| p.as_str()).collect();
        // Should only include root and valid members
        assert_eq!(paths, vec!["Cargo.toml", "crates/a/Cargo.toml"]);
    }

    /// Test that exclusion patterns with parent references are rejected.
    #[test]
    fn discover_rejects_exclusion_with_parent_references() {
        let tmp = TempDir::new().expect("temp dir");
        let root = utf8_root(&tmp);

        write_file(
            &root.join("Cargo.toml"),
            r#"[workspace]
members = ["crates/*", "!crates/../../other"]
"#,
        );
        write_file(
            &root.join("crates/a/Cargo.toml"),
            r#"[package]
name = "a"
version = "0.1.0"
"#,
        );

        let manifests = discover_manifests(&root).expect("discover");
        let paths: Vec<&str> = manifests.iter().map(|p| p.as_str()).collect();
        // Should include all members, invalid exclusion is ignored
        assert_eq!(paths, vec!["Cargo.toml", "crates/a/Cargo.toml"]);
    }
}
