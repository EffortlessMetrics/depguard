use camino::{Utf8Path, Utf8PathBuf};
use depguard_repo::{ScopeInput, build_workspace_model};
use depguard_types::RepoPath;
use tempfile::TempDir;

fn write_file(path: &Utf8Path, content: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create parent dirs");
    }
    std::fs::write(path, content).expect("write file");
}

fn setup_workspace() -> (TempDir, Utf8PathBuf) {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).expect("utf8 path");

    let root_manifest = r#"[workspace]
members = ["crates/a", "crates/b"]

[workspace.dependencies]
serde = "1.0"
"#;
    write_file(&root.join("Cargo.toml"), root_manifest);

    let member_a = r#"[package]
name = "crate-a"
version = "0.1.0"
edition = "2021"
"#;
    write_file(&root.join("crates").join("a").join("Cargo.toml"), member_a);

    let member_b = r#"[package]
name = "crate-b"
version = "0.1.0"
edition = "2021"
"#;
    write_file(&root.join("crates").join("b").join("Cargo.toml"), member_b);

    (temp, root)
}

#[test]
fn build_workspace_model_repo_scope_includes_all_manifests() {
    let (_temp, root) = setup_workspace();
    let model = build_workspace_model(root.as_path(), ScopeInput::Repo).expect("build model");

    let paths: Vec<&str> = model.manifests.iter().map(|m| m.path.as_str()).collect();
    assert_eq!(paths.len(), 3);
    assert!(paths.contains(&"Cargo.toml"));
    assert!(paths.contains(&"crates/a/Cargo.toml"));
    assert!(paths.contains(&"crates/b/Cargo.toml"));

    let serde = model
        .workspace_dependencies
        .get("serde")
        .expect("workspace dep serde");
    assert_eq!(serde.version.as_deref(), Some("1.0"));
}

#[test]
fn build_workspace_model_diff_scope_filters_manifests() {
    let (_temp, root) = setup_workspace();
    let model = build_workspace_model(
        root.as_path(),
        ScopeInput::Diff {
            changed_files: vec![RepoPath::new("crates/b/Cargo.toml")],
        },
    )
    .expect("build model");

    let paths: Vec<&str> = model.manifests.iter().map(|m| m.path.as_str()).collect();
    assert_eq!(paths, vec!["Cargo.toml", "crates/b/Cargo.toml"]);
}

#[test]
fn build_workspace_model_diff_scope_ignores_unknown_paths() {
    let (_temp, root) = setup_workspace();
    let model = build_workspace_model(
        root.as_path(),
        ScopeInput::Diff {
            changed_files: vec![RepoPath::new("crates/missing/Cargo.toml")],
        },
    )
    .expect("build model");

    let paths: Vec<&str> = model.manifests.iter().map(|m| m.path.as_str()).collect();
    assert_eq!(paths, vec!["Cargo.toml"]);
}

#[test]
fn fuzz_helpers_parse_and_expand_globs() {
    let ok_manifest = r#"[package]
name = "ok"
version = "0.1.0"
"#;
    assert!(depguard_repo::fuzz::parse_root_manifest(ok_manifest).is_ok());
    assert!(depguard_repo::fuzz::parse_member_manifest("bad = [").is_err());

    let patterns = vec!["crates/*".to_string()];
    let candidates = vec!["crates/a/Cargo.toml".to_string(), "src/lib.rs".to_string()];
    let matched = depguard_repo::fuzz::expand_globs(&patterns, &candidates).expect("glob expand");
    assert_eq!(matched, vec!["crates/a/Cargo.toml".to_string()]);
    assert!(depguard_repo::fuzz::expand_globs(&["[".to_string()], &candidates).is_err());
}
