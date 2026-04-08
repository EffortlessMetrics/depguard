use crate::model::{DepKind, DepSpec};
use globset::{Glob, GlobSet, GlobSetBuilder};
use serde_json::{Value, json};

pub fn build_allowlist(allow: &[String]) -> Option<GlobSet> {
    if allow.is_empty() {
        return None;
    }

    let mut builder = GlobSetBuilder::new();
    for pattern in allow {
        // Treat allowlist entries as glob patterns (case-sensitive).
        let glob =
            Glob::new(pattern).expect("allowlist patterns must be validated in depguard-settings");
        builder.add(glob);
    }
    Some(
        builder
            .build()
            .expect("allowlist patterns must be validated in depguard-settings"),
    )
}

pub fn is_allowed(allow: Option<&GlobSet>, value: &str) -> bool {
    allow.map(|set| set.is_match(value)).unwrap_or(false)
}

pub fn section_name(kind: DepKind) -> &'static str {
    match kind {
        DepKind::Normal => "dependencies",
        DepKind::Dev => "dev-dependencies",
        DepKind::Build => "build-dependencies",
    }
}

pub fn spec_to_json(spec: &DepSpec) -> Value {
    let mut obj = serde_json::Map::new();
    if let Some(v) = &spec.version {
        obj.insert("version".into(), json!(v));
    }
    if let Some(p) = &spec.path {
        obj.insert("path".into(), json!(p));
    }
    if spec.workspace {
        obj.insert("workspace".into(), json!(true));
    }
    if let Some(g) = &spec.git {
        obj.insert("git".into(), json!(g));
    }
    if let Some(b) = &spec.branch {
        obj.insert("branch".into(), json!(b));
    }
    if let Some(t) = &spec.tag {
        obj.insert("tag".into(), json!(t));
    }
    if let Some(r) = &spec.rev {
        obj.insert("rev".into(), json!(r));
    }
    if let Some(df) = spec.default_features {
        obj.insert("default-features".into(), json!(df));
    }
    if spec.optional {
        obj.insert("optional".into(), json!(true));
    }
    Value::Object(obj)
}
