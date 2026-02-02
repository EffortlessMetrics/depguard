use anyhow::Context;
use depguard_domain::model::{
    DepKind, DepSpec, DependencyDecl, ManifestModel, PackageMeta, WorkspaceDependency,
};
use depguard_types::{Location, RepoPath};
use std::collections::BTreeMap;
use toml_edit::{DocumentMut, Item, Value};

pub fn parse_root_manifest(
    manifest_path: &RepoPath,
    text: &str,
) -> anyhow::Result<(BTreeMap<String, WorkspaceDependency>, ManifestModel)> {
    let doc = text.parse::<DocumentMut>().context("parse Cargo.toml")?;
    let ws_deps = parse_workspace_dependencies(&doc, manifest_path);

    let model = parse_manifest_doc(&doc, manifest_path);

    Ok((ws_deps, model))
}

pub fn parse_member_manifest(manifest_path: &RepoPath, text: &str) -> anyhow::Result<ManifestModel> {
    let doc = text.parse::<DocumentMut>().context("parse Cargo.toml")?;
    Ok(parse_manifest_doc(&doc, manifest_path))
}

fn parse_manifest_doc(doc: &DocumentMut, manifest_path: &RepoPath) -> ManifestModel {
    let package = parse_package(doc);

    let mut deps: Vec<DependencyDecl> = Vec::new();
    deps.extend(parse_dep_table(doc.get("dependencies"), DepKind::Normal, manifest_path));
    deps.extend(parse_dep_table(
        doc.get("dev-dependencies"),
        DepKind::Dev,
        manifest_path,
    ));
    deps.extend(parse_dep_table(
        doc.get("build-dependencies"),
        DepKind::Build,
        manifest_path,
    ));

    // TODO: target-specific dependencies under `[target.'cfg(...)'.dependencies]` etc.

    ManifestModel {
        path: manifest_path.clone(),
        package,
        dependencies: deps,
    }
}

fn parse_package(doc: &DocumentMut) -> Option<PackageMeta> {
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

fn parse_workspace_dependencies(doc: &DocumentMut, manifest_path: &RepoPath) -> BTreeMap<String, WorkspaceDependency> {
    let mut out = BTreeMap::new();
    let Some(ws) = doc.get("workspace").and_then(|i| i.as_table()) else { return out };
    let Some(deps) = ws.get("dependencies").and_then(|i| i.as_table()) else { return out };

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
    out
}

fn parse_dep_table(section: Option<&Item>, kind: DepKind, manifest_path: &RepoPath) -> Vec<DependencyDecl> {
    let Some(tbl) = section.and_then(|i| i.as_table()) else { return Vec::new() };

    let mut out = Vec::new();
    for (name, item) in tbl.iter() {
        let spec = parse_spec(item);
        out.push(DependencyDecl {
            kind,
            name: name.to_string(),
            spec,
            location: Some(Location {
                path: manifest_path.clone(),
                line: None,
                col: None,
            }),
        });
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
