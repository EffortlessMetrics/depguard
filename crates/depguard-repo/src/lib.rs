//! Repository adapters: discover workspaces, read and parse Cargo manifests.
//!
//! This crate is allowed to do filesystem IO. It should not spawn external processes;
//! diff scoping should be supplied as a list of changed paths by the caller (typically the CLI).

#![forbid(unsafe_code)]

mod discover;
mod parse;

use anyhow::Context;
use camino::Utf8Path;
use depguard_domain::model::WorkspaceModel;
use depguard_types::RepoPath;

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
        let _ = parse::parse_root_manifest(&path, text)?;
        Ok(())
    }

    /// Parse arbitrary text as a member Cargo.toml manifest.
    ///
    /// Returns `Ok(...)` on valid TOML that can be parsed as a manifest,
    /// `Err(...)` otherwise. **Never panics** on any input.
    pub fn parse_member_manifest(text: &str) -> anyhow::Result<()> {
        let path = RepoPath::new("crates/fuzz/Cargo.toml");
        let _ = parse::parse_member_manifest(&path, text)?;
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
    let manifests = discover::discover_manifests(repo_root).context("discover manifests")?;

    // Always parse the root manifest for `[workspace.dependencies]`.
    let root_manifest = RepoPath::new("Cargo.toml");
    let root_abs = repo_root.join(root_manifest.as_str());
    let root_text =
        std::fs::read_to_string(&root_abs).with_context(|| format!("read {}", root_abs))?;
    let (root_ws_deps, root_model) =
        parse::parse_root_manifest(&root_manifest, &root_text).context("parse root manifest")?;

    let in_scope = match scope {
        ScopeInput::Repo => manifests.clone(),
        ScopeInput::Diff { changed_files } => {
            let mut s = Vec::new();
            // Root is always included (cheap and needed for workspace deps checks).
            s.push(root_manifest.clone());

            let changed: std::collections::BTreeSet<_> = changed_files
                .into_iter()
                .map(|p| p.as_str().to_string())
                .collect();

            for m in manifests {
                if changed.contains(m.as_str()) && !s.iter().any(|x| x.as_str() == m.as_str()) {
                    s.push(m);
                }
            }
            s
        }
    };

    let mut model = WorkspaceModel {
        repo_root: RepoPath::from(repo_root),
        workspace_dependencies: root_ws_deps,
        manifests: Vec::new(),
    };

    // Add the parsed root manifest (it may or may not be a package).
    model.manifests.push(root_model);

    // Parse all other manifests in scope (excluding root, which we already parsed).
    for manifest_path in in_scope.into_iter().filter(|p| p.as_str() != "Cargo.toml") {
        let abs = repo_root.join(manifest_path.as_str());
        let text = std::fs::read_to_string(&abs).with_context(|| format!("read {}", abs))?;
        let m = parse::parse_member_manifest(&manifest_path, &text)
            .with_context(|| format!("parse {}", manifest_path.as_str()))?;
        model.manifests.push(m);
    }

    Ok(model)
}
