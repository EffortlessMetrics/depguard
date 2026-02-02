use anyhow::Context;
use camino::{Utf8Path, Utf8PathBuf};
use depguard_types::RepoPath;
use globset::{Glob, GlobSetBuilder};
use toml_edit::DocumentMut;
use walkdir::WalkDir;

/// Discover Cargo manifests for the workspace rooted at `repo_root`.
///
/// Behavior:
/// - If the root manifest has `[workspace]`, expand `members` (with glob support) and apply `exclude`.
/// - Otherwise, return only `Cargo.toml` (single crate repository).
pub fn discover_manifests(repo_root: &Utf8Path) -> anyhow::Result<Vec<RepoPath>> {
    let root = repo_root.join("Cargo.toml");
    let text = std::fs::read_to_string(&root).with_context(|| format!("read {}", root))?;
    let doc = text.parse::<DocumentMut>().context("parse root Cargo.toml")?;

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

    for entry in WalkDir::new(repo_root).into_iter().filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() {
            continue;
        }
        if entry.file_name() != "Cargo.toml" {
            continue;
        }

        let abs: Utf8PathBuf = match Utf8PathBuf::from_path_buf(entry.path().to_path_buf()) {
            Ok(p) => p,
            Err(_) => continue,
        };
        let rel = abs.strip_prefix(repo_root).unwrap_or(&abs).as_str().replace('\\', "/");
        if rel == "Cargo.toml" {
            continue;
        }

        // Match both the file path and its parent directory against globs.
        let dir_rel = Utf8Path::new(&rel)
            .parent()
            .map(|p| p.as_str())
            .unwrap_or("");

        let is_member = members.is_empty() || member_set.is_match(&rel) || member_set.is_match(dir_rel);
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
