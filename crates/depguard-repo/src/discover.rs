use anyhow::Context;
use camino::{Utf8Path, Utf8PathBuf};
use depguard_types::RepoPath;
use globset::{Glob, GlobSetBuilder};
use toml_edit::DocumentMut;
use walkdir::WalkDir;
use std::path::PathBuf;

/// Discover Cargo manifests for the workspace rooted at `repo_root`.
///
/// Behavior:
/// - If the root manifest has `[workspace]`, expand `members` (with glob support) and apply `exclude`.
/// - Otherwise, return only `Cargo.toml` (single crate repository).
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

    let members: Vec<String> = doc
        .get("workspace")
        .and_then(|w| w.get("members"))
        .and_then(|i| i.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    let excludes: Vec<String> = doc
        .get("workspace")
        .and_then(|w| w.get("exclude"))
        .and_then(|i| i.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    let member_set = build_globset(&members).context("compile members globset")?;
    let exclude_set = build_globset(&excludes).context("compile exclude globset")?;

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

        // Match both the file path and its parent directory against globs.
        let dir_rel = Utf8Path::new(&rel)
            .parent()
            .map(|p| p.as_str())
            .unwrap_or("");

        let is_member =
            members.is_empty() || member_set.is_match(&rel) || member_set.is_match(dir_rel);
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

fn build_globset(patterns: &[String]) -> anyhow::Result<globset::GlobSet> {
    let mut b = GlobSetBuilder::new();
    for p in patterns {
        // Cargo workspace globs are relative paths.
        b.add(Glob::new(p)?);
    }
    Ok(b.build()?)
}

fn pathbuf_to_utf8(path: PathBuf) -> Option<Utf8PathBuf> {
    Utf8PathBuf::from_path_buf(path).ok()
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
}
