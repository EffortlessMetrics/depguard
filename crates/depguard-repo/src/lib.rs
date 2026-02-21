//! Repository adapters: discover workspaces, read manifest files, and assemble parsed models.
//!
//! Parsing is delegated to `depguard-repo-parser`; this crate is responsible for
//! filesystem IO, manifest discovery, and model caching.
//! It should not spawn external processes; diff scoping should be supplied as a list
//! of changed paths by the caller (typically the CLI).

#![forbid(unsafe_code)]

mod cache;
mod discover;

use anyhow::Context;
use cache::{ManifestCache, ManifestStamp};
use camino::{Utf8Path, Utf8PathBuf};
use depguard_domain_core::model::WorkspaceModel;
use depguard_repo_parser::{
    parse_member_manifest as parse_member_manifest_impl,
    parse_root_manifest as parse_root_manifest_impl,
};
use depguard_types::RepoPath;
use rayon::prelude::*;

pub use discover::discover_manifests;

/// Fuzz-friendly API for testing parsing robustness without filesystem access.
/// These functions are designed to never panic on any input.
pub mod fuzz {
    use super::*;

    /// Parse arbitrary text as a root Cargo.toml manifest.
    ///
    /// Returns `Ok(...)` on valid TOML that can be parsed as a manifest,
    /// `Err(...)` otherwise. **Never panics** on any input.
    pub fn parse_root_manifest(text: &str) -> anyhow::Result<()> {
        let path = RepoPath::new("Cargo.toml");
        let _ = parse_root_manifest_impl(&path, text)?;
        Ok(())
    }

    /// Parse arbitrary text as a member Cargo.toml manifest.
    ///
    /// Returns `Ok(...)` on valid TOML that can be parsed as a manifest,
    /// `Err(...)` otherwise. **Never panics** on any input.
    pub fn parse_member_manifest(text: &str) -> anyhow::Result<()> {
        let path = RepoPath::new("crates/fuzz/Cargo.toml");
        let _ = parse_member_manifest_impl(&path, text)?;
        Ok(())
    }

    /// Expand workspace member glob patterns against a list of candidate paths.
    ///
    /// This tests the glob compilation and matching logic without filesystem access.
    /// Returns `Ok(matched_paths)` if the pattern is valid, `Err(...)` otherwise.
    /// **Never panics** on any input.
    pub fn expand_globs(patterns: &[String], candidates: &[String]) -> anyhow::Result<Vec<String>> {
        use globset::{Glob, GlobSetBuilder};

        let mut builder = GlobSetBuilder::new();
        for p in patterns {
            builder.add(Glob::new(p)?);
        }
        let set = builder.build()?;

        let matched: Vec<String> = candidates
            .iter()
            .filter(|c| set.is_match(c))
            .cloned()
            .collect();

        Ok(matched)
    }
}

/// Input to scope selection. In `Diff`, the caller provides the changed files (from git).
#[derive(Clone, Debug)]
pub enum ScopeInput {
    Repo,
    Diff { changed_files: Vec<RepoPath> },
}

/// Build the in-memory workspace model used by the policy engine.
///
/// `repo_root` should be the git/workspace root (directory containing the root `Cargo.toml`).
pub fn build_workspace_model(
    repo_root: &Utf8Path,
    scope: ScopeInput,
) -> anyhow::Result<WorkspaceModel> {
    build_workspace_model_with_cache(repo_root, scope, None)
}

/// Build the in-memory workspace model and optionally cache parsed manifests.
///
/// When `cache_dir` is set, parsed manifests are persisted and reused when file
/// metadata `(size, modified timestamp)` is unchanged.
pub fn build_workspace_model_with_cache(
    repo_root: &Utf8Path,
    scope: ScopeInput,
    cache_dir: Option<&Utf8Path>,
) -> anyhow::Result<WorkspaceModel> {
    let manifests = discover::discover_manifests(repo_root).context("discover manifests")?;
    let root_manifest = RepoPath::new("Cargo.toml");
    let in_scope = manifests_in_scope(&manifests, &root_manifest, scope);

    let mut cache = cache_dir
        .map(|dir| ManifestCache::load(repo_root, dir))
        .transpose()?;

    // Always parse (or restore) the root manifest for `[workspace.dependencies]`.
    let root_abs = repo_root.join(root_manifest.as_str());
    let root_stamp = cache_stamp_for(&root_abs)?;

    let (root_ws_deps, root_model) = if let Some(store) = cache.as_mut() {
        if let Some(cached) = store.root_if_fresh(&root_manifest, root_stamp) {
            cached
        } else {
            let root_text =
                std::fs::read_to_string(&root_abs).with_context(|| format!("read {}", root_abs))?;
            let (deps, model) = parse_root_manifest_impl(&root_manifest, &root_text)
                .context("parse root manifest")?;
            store.store_root(&root_manifest, root_stamp, &deps, &model);
            (deps, model)
        }
    } else {
        let root_text =
            std::fs::read_to_string(&root_abs).with_context(|| format!("read {}", root_abs))?;
        parse_root_manifest_impl(&root_manifest, &root_text).context("parse root manifest")?
    };

    let mut model = WorkspaceModel {
        repo_root: RepoPath::from(repo_root),
        workspace_dependencies: root_ws_deps,
        manifests: Vec::new(),
    };

    // Add the parsed root manifest (it may or may not be a package).
    model.manifests.push(root_model);

    // Parse all other manifests in scope (excluding root, which we already parsed).
    // This is parallel for large workspaces but deterministic because `par_iter` on Vec
    // preserves index order in `collect`.
    let mut member_paths: Vec<RepoPath> = in_scope
        .into_iter()
        .filter(|p| p.as_str() != "Cargo.toml")
        .collect();
    member_paths.sort();

    if let Some(store) = cache.as_mut() {
        for manifest_path in &member_paths {
            let abs = repo_root.join(manifest_path.as_str());
            let stamp = cache_stamp_for(&abs)?;
            if let Some(cached) = store.member_if_fresh(manifest_path, stamp) {
                model.manifests.push(cached);
                continue;
            }

            let text = std::fs::read_to_string(&abs).with_context(|| format!("read {}", abs))?;
            let parsed = parse_member_manifest_impl(manifest_path, &text)
                .with_context(|| format!("parse {}", manifest_path.as_str()))?;
            store.store_member(manifest_path, stamp, &parsed);
            model.manifests.push(parsed);
        }
        store.save_if_dirty()?;
    } else {
        let parsed_members: Vec<anyhow::Result<_>> = member_paths
            .par_iter()
            .map(|manifest_path| {
                let abs = repo_root.join(manifest_path.as_str());
                let text =
                    std::fs::read_to_string(&abs).with_context(|| format!("read {}", abs))?;
                parse_member_manifest_impl(manifest_path, &text)
                    .with_context(|| format!("parse {}", manifest_path.as_str()))
            })
            .collect();

        for parsed in parsed_members {
            model.manifests.push(parsed?);
        }
    }

    Ok(model)
}

fn manifests_in_scope(
    manifests: &[RepoPath],
    root_manifest: &RepoPath,
    scope: ScopeInput,
) -> Vec<RepoPath> {
    match scope {
        ScopeInput::Repo => manifests.to_vec(),
        ScopeInput::Diff { changed_files } => {
            let mut scoped = vec![root_manifest.clone()];
            let changed: std::collections::BTreeSet<_> = changed_files
                .into_iter()
                .map(|p| p.as_str().to_string())
                .collect();

            for manifest in manifests {
                if changed.contains(manifest.as_str())
                    && !scoped.iter().any(|m| m.as_str() == manifest.as_str())
                {
                    scoped.push(manifest.clone());
                }
            }
            scoped
        }
    }
}

fn cache_stamp_for(path: &Utf8PathBuf) -> anyhow::Result<ManifestStamp> {
    ManifestStamp::from_path(path).with_context(|| format!("cache stamp {}", path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use camino::Utf8PathBuf;
    use proptest::prelude::*;
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
    fn build_workspace_model_repo_scope_includes_all_manifests() {
        let tmp = TempDir::new().expect("temp dir");
        let root = utf8_root(&tmp);

        write_file(
            &root.join("Cargo.toml"),
            r#"[workspace]
members = ["crates/a", "crates/b"]

[workspace.dependencies]
serde = "1.0"
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

        let model = build_workspace_model(&root, ScopeInput::Repo).expect("build model");
        let mut paths: Vec<String> = model
            .manifests
            .iter()
            .map(|m| m.path.as_str().to_string())
            .collect();
        paths.sort();
        assert_eq!(
            paths,
            vec![
                "Cargo.toml".to_string(),
                "crates/a/Cargo.toml".to_string(),
                "crates/b/Cargo.toml".to_string()
            ]
        );

        let serde_dep = model
            .workspace_dependencies
            .get("serde")
            .expect("workspace dep");
        assert_eq!(serde_dep.version.as_deref(), Some("1.0"));
    }

    #[test]
    fn build_workspace_model_diff_scope_filters_members() {
        let tmp = TempDir::new().expect("temp dir");
        let root = utf8_root(&tmp);

        write_file(
            &root.join("Cargo.toml"),
            r#"[workspace]
members = ["crates/a", "crates/b"]
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

        let model = build_workspace_model(
            &root,
            ScopeInput::Diff {
                changed_files: vec![RepoPath::new("crates/a/Cargo.toml")],
            },
        )
        .expect("build model");

        let mut paths: Vec<String> = model
            .manifests
            .iter()
            .map(|m| m.path.as_str().to_string())
            .collect();
        paths.sort();
        assert_eq!(
            paths,
            vec!["Cargo.toml".to_string(), "crates/a/Cargo.toml".to_string()]
        );
    }

    #[test]
    fn build_workspace_model_with_cache_writes_cache_file() {
        let tmp = TempDir::new().expect("temp dir");
        let root = utf8_root(&tmp);
        write_file(
            &root.join("Cargo.toml"),
            r#"[workspace]
members = ["crates/a"]
"#,
        );
        write_file(
            &root.join("crates/a/Cargo.toml"),
            r#"[package]
name = "a"
version = "0.1.0"
"#,
        );

        let cache_dir = Utf8Path::new(".depguard-cache");
        let _ = build_workspace_model_with_cache(&root, ScopeInput::Repo, Some(cache_dir))
            .expect("build model with cache");

        let cache_file = root
            .join(".depguard-cache")
            .join(cache::MANIFEST_CACHE_FILENAME);
        assert!(cache_file.exists(), "expected cache file at {}", cache_file);
    }

    #[test]
    fn build_workspace_model_with_cache_invalidates_on_manifest_change() {
        let tmp = TempDir::new().expect("temp dir");
        let root = utf8_root(&tmp);
        write_file(
            &root.join("Cargo.toml"),
            r#"[workspace]
members = ["crates/a"]
"#,
        );
        let member_path = root.join("crates/a/Cargo.toml");
        write_file(
            &member_path,
            r#"[package]
name = "a"
version = "0.1.0"
"#,
        );

        let cache_dir = Utf8Path::new(".depguard-cache");
        let _ = build_workspace_model_with_cache(&root, ScopeInput::Repo, Some(cache_dir))
            .expect("first build");

        // Break the manifest to ensure stale cache is not reused.
        write_file(
            &member_path,
            r#"[package]
name = "a"
version = "#,
        );

        let err = build_workspace_model_with_cache(&root, ScopeInput::Repo, Some(cache_dir))
            .expect_err("expected parse error");
        assert!(err.to_string().contains("parse"));
    }

    proptest! {
        #[test]
        fn fuzz_parsers_never_panic(input in ".*") {
            let _ = fuzz::parse_root_manifest(&input);
            let _ = fuzz::parse_member_manifest(&input);
        }
    }
}
