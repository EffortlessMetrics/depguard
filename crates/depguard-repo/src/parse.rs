use anyhow::Context;
use depguard_domain::model::{
    DepKind, DepSpec, DependencyDecl, ManifestModel, PackageMeta, WorkspaceDependency,
};
use depguard_types::{Location, RepoPath};
use std::collections::BTreeMap;
use toml_edit::{ImDocument, Item, Value};

/// Calculate the 1-based line number from a byte offset in the source text.
fn byte_offset_to_line(source: &str, offset: usize) -> u32 {
    // Count newlines before the offset
    let line_count = source[..offset.min(source.len())]
        .bytes()
        .filter(|&b| b == b'\n')
        .count();
    // Lines are 1-based
    (line_count + 1) as u32
}

pub fn parse_root_manifest(
    manifest_path: &RepoPath,
    text: &str,
) -> anyhow::Result<(BTreeMap<String, WorkspaceDependency>, ManifestModel)> {
    let doc: ImDocument<&str> = ImDocument::parse(text).context("parse Cargo.toml")?;
    let ws_deps = parse_workspace_dependencies(&doc, manifest_path, text);

    let model = parse_manifest_doc(&doc, manifest_path, text);

    Ok((ws_deps, model))
}

pub fn parse_member_manifest(
    manifest_path: &RepoPath,
    text: &str,
) -> anyhow::Result<ManifestModel> {
    let doc: ImDocument<&str> = ImDocument::parse(text).context("parse Cargo.toml")?;
    Ok(parse_manifest_doc(&doc, manifest_path, text))
}

fn parse_manifest_doc(
    doc: &ImDocument<&str>,
    manifest_path: &RepoPath,
    source: &str,
) -> ManifestModel {
    let package = parse_package(doc);

    let mut deps: Vec<DependencyDecl> = Vec::new();
    deps.extend(parse_dep_table(
        doc.get("dependencies"),
        DepKind::Normal,
        manifest_path,
        source,
    ));
    deps.extend(parse_dep_table(
        doc.get("dev-dependencies"),
        DepKind::Dev,
        manifest_path,
        source,
    ));
    deps.extend(parse_dep_table(
        doc.get("build-dependencies"),
        DepKind::Build,
        manifest_path,
        source,
    ));

    // Parse target-specific dependencies under `[target.'cfg(...)'.dependencies]` etc.
    deps.extend(parse_target_dependencies(doc, manifest_path, source));

    ManifestModel {
        path: manifest_path.clone(),
        package,
        dependencies: deps,
    }
}

fn parse_package(doc: &ImDocument<&str>) -> Option<PackageMeta> {
    let pkg = doc.get("package")?.as_table()?;
    let name = pkg.get("name")?.as_str()?.to_string();

    let publish = match pkg.get("publish") {
        None => true,
        Some(Item::Value(Value::Boolean(b))) => *b.value(),
        Some(Item::Value(Value::Array(a))) => !a.is_empty(),
        _ => true,
    };

    Some(PackageMeta { name, publish })
}

fn parse_workspace_dependencies(
    doc: &ImDocument<&str>,
    manifest_path: &RepoPath,
    source: &str,
) -> BTreeMap<String, WorkspaceDependency> {
    let mut out = BTreeMap::new();
    let Some(ws) = doc.get("workspace").and_then(|i| i.as_table()) else {
        return out;
    };
    let Some(deps) = ws.get("dependencies").and_then(|i| i.as_table()) else {
        return out;
    };

    for (name, item) in deps.iter() {
        let spec = parse_spec(item);
        out.insert(
            name.to_string(),
            WorkspaceDependency {
                name: name.to_string(),
                version: spec.version,
                path: spec.path,
                workspace: spec.workspace,
            },
        );
    }

    // Note: locations for workspace deps are not captured in this scaffold.
    let _ = manifest_path;
    let _ = source;
    out
}

fn parse_dep_table(
    section: Option<&Item>,
    kind: DepKind,
    manifest_path: &RepoPath,
    source: &str,
) -> Vec<DependencyDecl> {
    let Some(tbl) = section.and_then(|i| i.as_table()) else {
        return Vec::new();
    };

    let mut out = Vec::new();
    for (name, item) in tbl.iter() {
        let spec = parse_spec(item);
        // Get line number from the item's span (byte offset in source)
        let line = item
            .span()
            .map(|span| byte_offset_to_line(source, span.start));
        out.push(DependencyDecl {
            kind,
            name: name.to_string(),
            spec,
            location: Some(Location {
                path: manifest_path.clone(),
                line,
                col: None,
            }),
        });
    }
    out
}

/// Parse target-specific dependencies from `[target.*]` tables.
///
/// Cargo supports tables like:
/// - `[target.'cfg(unix)'.dependencies]`
/// - `[target.'cfg(windows)'.dev-dependencies]`
/// - `[target.x86_64-unknown-linux-gnu.build-dependencies]`
fn parse_target_dependencies(
    doc: &ImDocument<&str>,
    manifest_path: &RepoPath,
    source: &str,
) -> Vec<DependencyDecl> {
    let Some(target_table) = doc.get("target").and_then(|i| i.as_table()) else {
        return Vec::new();
    };

    let mut out = Vec::new();

    // Iterate over each target spec (e.g., 'cfg(unix)', 'x86_64-unknown-linux-gnu')
    for (_target_spec, target_item) in target_table.iter() {
        let Some(target_subtable) = target_item.as_table() else {
            continue;
        };

        // Parse dependencies, dev-dependencies, and build-dependencies for this target
        out.extend(parse_dep_table(
            target_subtable.get("dependencies"),
            DepKind::Normal,
            manifest_path,
            source,
        ));
        out.extend(parse_dep_table(
            target_subtable.get("dev-dependencies"),
            DepKind::Dev,
            manifest_path,
            source,
        ));
        out.extend(parse_dep_table(
            target_subtable.get("build-dependencies"),
            DepKind::Build,
            manifest_path,
            source,
        ));
    }

    out
}

fn parse_spec(item: &Item) -> DepSpec {
    match item {
        Item::Value(Value::String(s)) => DepSpec {
            version: Some(s.value().to_string()),
            path: None,
            workspace: false,
        },
        Item::Value(Value::InlineTable(t)) => parse_inline_table(t),
        Item::Table(t) => parse_table(t),
        _ => DepSpec::default(),
    }
}

fn parse_inline_table(t: &toml_edit::InlineTable) -> DepSpec {
    let mut spec = DepSpec::default();
    if let Some(v) = t.get("version").and_then(|v| v.as_str()) {
        spec.version = Some(v.to_string());
    }
    if let Some(p) = t.get("path").and_then(|v| v.as_str()) {
        spec.path = Some(p.to_string());
    }
    if let Some(w) = t.get("workspace").and_then(|v| v.as_bool()) {
        spec.workspace = w;
    }
    spec
}

fn parse_table(t: &toml_edit::Table) -> DepSpec {
    let mut spec = DepSpec::default();
    if let Some(v) = t.get("version").and_then(|v| v.as_str()) {
        spec.version = Some(v.to_string());
    }
    if let Some(p) = t.get("path").and_then(|v| v.as_str()) {
        spec.path = Some(p.to_string());
    }
    if let Some(w) = t.get("workspace").and_then(|v| v.as_bool()) {
        spec.workspace = w;
    }
    spec
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_target_specific_dependencies() {
        let manifest = r#"
[package]
name = "test-pkg"
version = "0.1.0"

[dependencies]
serde = "1.0"

[dev-dependencies]
mockall = "0.11"

[target.'cfg(unix)'.dependencies]
nix = "0.26"

[target.'cfg(windows)'.dependencies]
windows = "0.48"

[target.'cfg(unix)'.dev-dependencies]
pprof = "0.11"

[target.x86_64-unknown-linux-gnu.build-dependencies]
cc = "1.0"
"#;

        let manifest_path = RepoPath::new("crates/test-pkg/Cargo.toml");
        let model = parse_member_manifest(&manifest_path, manifest)
            .expect("target-specific manifest should parse successfully");

        // Check that we have all expected dependencies
        let dep_names: Vec<_> = model.dependencies.iter().map(|d| d.name.as_str()).collect();

        // Normal dependencies (regular + target-specific)
        assert!(dep_names.contains(&"serde"), "should contain serde");
        assert!(
            dep_names.contains(&"nix"),
            "should contain nix (unix target)"
        );
        assert!(
            dep_names.contains(&"windows"),
            "should contain windows (windows target)"
        );

        // Dev dependencies (regular + target-specific)
        assert!(dep_names.contains(&"mockall"), "should contain mockall");
        assert!(
            dep_names.contains(&"pprof"),
            "should contain pprof (unix target dev-dep)"
        );

        // Build dependencies (target-specific)
        assert!(
            dep_names.contains(&"cc"),
            "should contain cc (linux target build-dep)"
        );

        // Verify DepKind is correct
        let find_dep = |name: &str| {
            model
                .dependencies
                .iter()
                .find(|d| d.name == name)
                .unwrap_or_else(|| panic!("dependency '{}' should exist", name))
        };

        assert_eq!(find_dep("serde").kind, DepKind::Normal);
        assert_eq!(find_dep("nix").kind, DepKind::Normal);
        assert_eq!(find_dep("windows").kind, DepKind::Normal);
        assert_eq!(find_dep("mockall").kind, DepKind::Dev);
        assert_eq!(find_dep("pprof").kind, DepKind::Dev);
        assert_eq!(find_dep("cc").kind, DepKind::Build);
    }

    #[test]
    fn test_no_target_dependencies() {
        let manifest = r#"
[package]
name = "simple-pkg"
version = "0.1.0"

[dependencies]
serde = "1.0"
"#;

        let manifest_path = RepoPath::new("Cargo.toml");
        let model = parse_member_manifest(&manifest_path, manifest)
            .expect("simple manifest should parse successfully");

        assert_eq!(model.dependencies.len(), 1);
        assert_eq!(model.dependencies[0].name, "serde");
    }

    #[test]
    fn test_byte_offset_to_line() {
        let source = "line1\nline2\nline3\nline4";
        // Offset 0 is on line 1
        assert_eq!(byte_offset_to_line(source, 0), 1);
        // Offset 5 is the newline after "line1", still line 1
        assert_eq!(byte_offset_to_line(source, 5), 1);
        // Offset 6 is the start of "line2", so line 2
        assert_eq!(byte_offset_to_line(source, 6), 2);
        // Offset 12 is the start of "line3", so line 3
        assert_eq!(byte_offset_to_line(source, 12), 3);
        // Offset 18 is the start of "line4", so line 4
        assert_eq!(byte_offset_to_line(source, 18), 4);
        // Offset beyond end should clamp
        assert_eq!(byte_offset_to_line(source, 1000), 4);
    }

    #[test]
    fn test_line_numbers_captured_correctly() {
        // Note: Lines start at 1, and the manifest below has:
        // Line 1: [package]
        // Line 2: name = "test-pkg"
        // Line 3: version = "0.1.0"
        // Line 4: (empty)
        // Line 5: [dependencies]
        // Line 6: serde = "1.0"
        // Line 7: tokio = { version = "1.0", features = ["full"] }
        // Line 8: (empty)
        // Line 9: [dev-dependencies]
        // Line 10: insta = "1.0"
        let manifest = "[package]\n\
            name = \"test-pkg\"\n\
            version = \"0.1.0\"\n\
            \n\
            [dependencies]\n\
            serde = \"1.0\"\n\
            tokio = { version = \"1.0\", features = [\"full\"] }\n\
            \n\
            [dev-dependencies]\n\
            insta = \"1.0\"\n";

        let manifest_path = RepoPath::new("Cargo.toml");
        let model = parse_member_manifest(&manifest_path, manifest)
            .expect("manifest with line numbers should parse successfully");

        // Find each dependency and check its line number
        let find_dep = |name: &str| {
            model
                .dependencies
                .iter()
                .find(|d| d.name == name)
                .unwrap_or_else(|| panic!("dependency '{}' should exist", name))
        };

        let serde_dep = find_dep("serde");
        let serde_line = serde_dep
            .location
            .as_ref()
            .expect("serde should have a location")
            .line;
        assert_eq!(serde_line, Some(6), "serde should be on line 6");

        let tokio_dep = find_dep("tokio");
        let tokio_line = tokio_dep
            .location
            .as_ref()
            .expect("tokio should have a location")
            .line;
        assert_eq!(tokio_line, Some(7), "tokio should be on line 7");

        let insta_dep = find_dep("insta");
        let insta_line = insta_dep
            .location
            .as_ref()
            .expect("insta should have a location")
            .line;
        assert_eq!(insta_line, Some(10), "insta should be on line 10");
    }

    #[test]
    fn test_line_numbers_with_table_style_dep() {
        // Test with a dependency specified as a TOML table (not inline)
        // Line 1: [package]
        // Line 2: name = "test-pkg"
        // Line 3: version = "0.1.0"
        // Line 4: (empty)
        // Line 5: [dependencies]
        // Line 6: serde = "1.0"
        // Line 7: (empty)
        // Line 8: [dependencies.tokio]
        // Line 9: version = "1.0"
        // Line 10: features = ["full"]
        let manifest = "[package]\n\
            name = \"test-pkg\"\n\
            version = \"0.1.0\"\n\
            \n\
            [dependencies]\n\
            serde = \"1.0\"\n\
            \n\
            [dependencies.tokio]\n\
            version = \"1.0\"\n\
            features = [\"full\"]\n";

        let manifest_path = RepoPath::new("Cargo.toml");
        let model = parse_member_manifest(&manifest_path, manifest)
            .expect("manifest with table-style dep should parse successfully");

        let find_dep = |name: &str| {
            model
                .dependencies
                .iter()
                .find(|d| d.name == name)
                .unwrap_or_else(|| panic!("dependency '{}' should exist", name))
        };

        let serde_dep = find_dep("serde");
        let serde_line = serde_dep
            .location
            .as_ref()
            .expect("serde should have a location")
            .line;
        assert_eq!(serde_line, Some(6), "serde should be on line 6");

        let tokio_dep = find_dep("tokio");
        let tokio_line = tokio_dep
            .location
            .as_ref()
            .expect("tokio should have a location")
            .line;
        // For table-style deps, the span points to the table section
        assert!(tokio_line.is_some(), "tokio should have a line number");
    }
}
