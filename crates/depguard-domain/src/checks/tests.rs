use super::{
    default_features_explicit, dev_only_in_normal, git_requires_version, no_multiple_versions,
    no_wildcards, optional_unused, path_requires_version, path_safety, utils,
    workspace_inheritance,
};
use crate::model::{DepKind, DepSpec};
use crate::test_support::{
    config_with_check, config_with_check_allow, dep_decl, manifest, model, workspace_dep,
};
use depguard_types::{Severity, ids};
use std::collections::BTreeMap;

#[test]
fn no_wildcards_handles_missing_version_allowlist_and_target() {
    let cfg = config_with_check_allow(
        ids::CHECK_DEPS_NO_WILDCARDS,
        Severity::Warning,
        vec!["allowed"],
        false,
    );

    let deps = vec![
        dep_decl("no_version", DepKind::Normal, DepSpec::default(), None),
        dep_decl(
            "allowed",
            DepKind::Normal,
            DepSpec {
                version: Some("*".to_string()),
                ..DepSpec::default()
            },
            None,
        ),
        dep_decl(
            "bad",
            DepKind::Normal,
            DepSpec {
                version: Some("*".to_string()),
                ..DepSpec::default()
            },
            Some("cfg(windows)"),
        ),
        dep_decl(
            "ok",
            DepKind::Normal,
            DepSpec {
                version: Some("1.2.3".to_string()),
                ..DepSpec::default()
            },
            None,
        ),
    ];

    let manifest = manifest("Cargo.toml", true, deps, BTreeMap::new());
    let model = model(vec![manifest], BTreeMap::new());

    let mut out = Vec::new();
    no_wildcards::run(&model, &cfg, &mut out);

    assert_eq!(out.len(), 1);
    let finding = &out[0];
    assert_eq!(finding.code, ids::CODE_WILDCARD_VERSION);
    assert_eq!(finding.data["section"], "dependencies");
    assert_eq!(finding.data["target"], "cfg(windows)");
}

#[test]
fn path_requires_version_respects_publish_policy_and_allowlist() {
    let deps = vec![
        dep_decl(
            "allowed",
            DepKind::Normal,
            DepSpec {
                path: Some("local/dep".to_string()),
                ..DepSpec::default()
            },
            None,
        ),
        dep_decl(
            "bad",
            DepKind::Normal,
            DepSpec {
                path: Some("local/dep".to_string()),
                ..DepSpec::default()
            },
            Some("cfg(unix)"),
        ),
    ];

    let manifest = manifest("crates/a/Cargo.toml", false, deps, BTreeMap::new());
    let model = model(vec![manifest.clone()], BTreeMap::new());

    let cfg_skip = config_with_check_allow(
        ids::CHECK_DEPS_PATH_REQUIRES_VERSION,
        Severity::Warning,
        Vec::new(),
        false,
    );
    let mut out = Vec::new();
    path_requires_version::run(&model, &cfg_skip, &mut out);
    assert!(out.is_empty());

    let cfg = config_with_check_allow(
        ids::CHECK_DEPS_PATH_REQUIRES_VERSION,
        Severity::Warning,
        vec!["allowed"],
        true,
    );
    let mut out = Vec::new();
    path_requires_version::run(&model, &cfg, &mut out);
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].data["target"], "cfg(unix)");
}

#[test]
fn git_requires_version_respects_publish_policy_and_allowlist() {
    let deps = vec![
        dep_decl(
            "allowed",
            DepKind::Normal,
            DepSpec {
                git: Some("https://example.com/allowed.git".to_string()),
                ..DepSpec::default()
            },
            None,
        ),
        dep_decl(
            "bad",
            DepKind::Normal,
            DepSpec {
                git: Some("https://example.com/bad.git".to_string()),
                ..DepSpec::default()
            },
            Some("cfg(target_os = \"linux\")"),
        ),
    ];

    let manifest = manifest("crates/a/Cargo.toml", false, deps, BTreeMap::new());
    let model = model(vec![manifest.clone()], BTreeMap::new());

    let cfg_skip = config_with_check_allow(
        ids::CHECK_DEPS_GIT_REQUIRES_VERSION,
        Severity::Warning,
        Vec::new(),
        false,
    );
    let mut out = Vec::new();
    git_requires_version::run(&model, &cfg_skip, &mut out);
    assert!(out.is_empty());

    let cfg = config_with_check_allow(
        ids::CHECK_DEPS_GIT_REQUIRES_VERSION,
        Severity::Warning,
        vec!["allowed"],
        true,
    );
    let mut out = Vec::new();
    git_requires_version::run(&model, &cfg, &mut out);
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].data["target"], "cfg(target_os = \"linux\")");
}

#[test]
fn git_requires_version_includes_dep_without_target() {
    let deps = vec![dep_decl(
        "bad",
        DepKind::Normal,
        DepSpec {
            git: Some("https://example.com/bad.git".to_string()),
            ..DepSpec::default()
        },
        None,
    )];

    let manifest = manifest("crates/a/Cargo.toml", true, deps, BTreeMap::new());
    let model = model(vec![manifest], BTreeMap::new());

    let cfg = config_with_check_allow(
        ids::CHECK_DEPS_GIT_REQUIRES_VERSION,
        Severity::Warning,
        Vec::new(),
        false,
    );

    let mut out = Vec::new();
    git_requires_version::run(&model, &cfg, &mut out);
    assert_eq!(out.len(), 1);
    assert!(out[0].data.get("target").is_none());
}

#[test]
fn git_requires_version_skips_versioned_git_deps() {
    let deps = vec![dep_decl(
        "versioned",
        DepKind::Normal,
        DepSpec {
            git: Some("https://example.com/versioned.git".to_string()),
            version: Some("1.2.3".to_string()),
            ..DepSpec::default()
        },
        None,
    )];

    let manifest = manifest("crates/a/Cargo.toml", true, deps, BTreeMap::new());
    let model = model(vec![manifest], BTreeMap::new());

    let cfg = config_with_check_allow(
        ids::CHECK_DEPS_GIT_REQUIRES_VERSION,
        Severity::Warning,
        Vec::new(),
        false,
    );

    let mut out = Vec::new();
    git_requires_version::run(&model, &cfg, &mut out);
    assert!(out.is_empty());
}

#[test]
fn path_safety_reports_absolute_and_escape_with_allowlist() {
    let deps = vec![
        dep_decl(
            "allowed",
            DepKind::Normal,
            DepSpec {
                path: Some("vendor/allowed".to_string()),
                ..DepSpec::default()
            },
            None,
        ),
        dep_decl(
            "abs",
            DepKind::Normal,
            DepSpec {
                path: Some("/abs/path".to_string()),
                ..DepSpec::default()
            },
            None,
        ),
        dep_decl(
            "escape",
            DepKind::Normal,
            DepSpec {
                path: Some("../outside".to_string()),
                ..DepSpec::default()
            },
            Some("cfg(windows)"),
        ),
    ];

    let manifest = manifest("Cargo.toml", true, deps, BTreeMap::new());
    let model = model(vec![manifest], BTreeMap::new());

    let cfg = config_with_check_allow(
        ids::CHECK_DEPS_PATH_SAFETY,
        Severity::Warning,
        vec!["vendor/*"],
        false,
    );

    let mut out = Vec::new();
    path_safety::run(&model, &cfg, &mut out);

    assert_eq!(out.len(), 2);
    assert_eq!(out[0].code, ids::CODE_ABSOLUTE_PATH);
    assert_eq!(out[1].code, ids::CODE_PARENT_ESCAPE);
    assert_eq!(out[1].data["target"], "cfg(windows)");
}

#[test]
fn path_safety_absolute_path_with_target_includes_target() {
    let deps = vec![dep_decl(
        "abs",
        DepKind::Normal,
        DepSpec {
            path: Some("/abs/path".to_string()),
            ..DepSpec::default()
        },
        Some("cfg(unix)"),
    )];

    let manifest = manifest("Cargo.toml", true, deps, BTreeMap::new());
    let model = model(vec![manifest], BTreeMap::new());

    let cfg = config_with_check_allow(
        ids::CHECK_DEPS_PATH_SAFETY,
        Severity::Warning,
        Vec::new(),
        false,
    );

    let mut out = Vec::new();
    path_safety::run(&model, &cfg, &mut out);
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].code, ids::CODE_ABSOLUTE_PATH);
    assert_eq!(out[0].data["target"], "cfg(unix)");
}

#[test]
fn workspace_inheritance_skips_empty_workspace_deps() {
    let deps = vec![dep_decl(
        "serde",
        DepKind::Normal,
        DepSpec {
            version: Some("1.0".to_string()),
            ..DepSpec::default()
        },
        None,
    )];
    let manifest = manifest("Cargo.toml", true, deps, BTreeMap::new());
    let model = model(vec![manifest], BTreeMap::new());

    let cfg = config_with_check(ids::CHECK_DEPS_WORKSPACE_INHERITANCE, Severity::Warning);

    let mut out = Vec::new();
    workspace_inheritance::run(&model, &cfg, &mut out);
    assert!(out.is_empty());
}

#[test]
fn workspace_inheritance_reports_and_allowlist() {
    let deps = vec![
        dep_decl(
            "allowed",
            DepKind::Normal,
            DepSpec {
                version: Some("1.0".to_string()),
                ..DepSpec::default()
            },
            None,
        ),
        dep_decl(
            "workspace_ok",
            DepKind::Normal,
            DepSpec {
                workspace: true,
                ..DepSpec::default()
            },
            None,
        ),
        dep_decl(
            "bad",
            DepKind::Normal,
            DepSpec {
                version: Some("2.0".to_string()),
                ..DepSpec::default()
            },
            Some("cfg(unix)"),
        ),
        dep_decl(
            "other",
            DepKind::Normal,
            DepSpec {
                version: Some("3.0".to_string()),
                ..DepSpec::default()
            },
            None,
        ),
    ];

    let manifest = manifest("Cargo.toml", true, deps, BTreeMap::new());
    let mut workspace_deps = BTreeMap::new();
    workspace_deps.insert("allowed".to_string(), workspace_dep("allowed").1);
    workspace_deps.insert("workspace_ok".to_string(), workspace_dep("workspace_ok").1);
    workspace_deps.insert("bad".to_string(), workspace_dep("bad").1);

    let model = model(vec![manifest], workspace_deps);

    let cfg = config_with_check_allow(
        ids::CHECK_DEPS_WORKSPACE_INHERITANCE,
        Severity::Warning,
        vec!["allowed"],
        false,
    );

    let mut out = Vec::new();
    workspace_inheritance::run(&model, &cfg, &mut out);
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].code, ids::CODE_MISSING_WORKSPACE_TRUE);
    assert_eq!(out[0].data["target"], "cfg(unix)");
}

#[test]
fn dev_only_in_normal_flags_and_skips() {
    let deps = vec![
        dep_decl(
            "proptest",
            DepKind::Normal,
            DepSpec {
                version: Some("1.0".to_string()),
                ..DepSpec::default()
            },
            None,
        ),
        dep_decl(
            "criterion",
            DepKind::Normal,
            DepSpec {
                version: Some("0.5".to_string()),
                ..DepSpec::default()
            },
            Some("cfg(unix)"),
        ),
        dep_decl(
            "serde",
            DepKind::Normal,
            DepSpec {
                version: Some("1.0".to_string()),
                ..DepSpec::default()
            },
            None,
        ),
        dep_decl(
            "insta",
            DepKind::Dev,
            DepSpec {
                version: Some("1.0".to_string()),
                ..DepSpec::default()
            },
            None,
        ),
    ];

    let manifest = manifest("Cargo.toml", true, deps, BTreeMap::new());
    let model = model(vec![manifest], BTreeMap::new());

    let cfg = config_with_check_allow(
        ids::CHECK_DEPS_DEV_ONLY_IN_NORMAL,
        Severity::Warning,
        vec!["proptest"],
        false,
    );

    let mut out = Vec::new();
    dev_only_in_normal::run(&model, &cfg, &mut out);
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].code, ids::CODE_DEV_DEP_IN_NORMAL);
    assert_eq!(out[0].data["target"], "cfg(unix)");
}

#[test]
fn default_features_explicit_flags_missing_default_features() {
    let deps = vec![
        dep_decl(
            "workspace_dep",
            DepKind::Normal,
            DepSpec {
                workspace: true,
                ..DepSpec::default()
            },
            None,
        ),
        dep_decl(
            "simple",
            DepKind::Normal,
            DepSpec {
                version: Some("1.0".to_string()),
                ..DepSpec::default()
            },
            None,
        ),
        dep_decl(
            "explicit",
            DepKind::Normal,
            DepSpec {
                path: Some("local/explicit".to_string()),
                default_features: Some(true),
                ..DepSpec::default()
            },
            None,
        ),
        dep_decl(
            "allowed",
            DepKind::Normal,
            DepSpec {
                git: Some("https://example.com/allowed.git".to_string()),
                ..DepSpec::default()
            },
            None,
        ),
        dep_decl(
            "bad",
            DepKind::Normal,
            DepSpec {
                optional: true,
                ..DepSpec::default()
            },
            Some("cfg(target_os = \"linux\")"),
        ),
    ];

    let manifest = manifest("Cargo.toml", true, deps, BTreeMap::new());
    let model = model(vec![manifest], BTreeMap::new());

    let cfg = config_with_check_allow(
        ids::CHECK_DEPS_DEFAULT_FEATURES_EXPLICIT,
        Severity::Warning,
        vec!["allowed"],
        false,
    );

    let mut out = Vec::new();
    default_features_explicit::run(&model, &cfg, &mut out);
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].code, ids::CODE_DEFAULT_FEATURES_IMPLICIT);
    assert_eq!(out[0].data["target"], "cfg(target_os = \"linux\")");
}

#[test]
fn default_features_explicit_flags_path_and_git_specs() {
    let deps = vec![
        dep_decl(
            "path_dep",
            DepKind::Normal,
            DepSpec {
                path: Some("local/path".to_string()),
                ..DepSpec::default()
            },
            None,
        ),
        dep_decl(
            "git_dep",
            DepKind::Normal,
            DepSpec {
                git: Some("https://example.com/repo.git".to_string()),
                ..DepSpec::default()
            },
            Some("cfg(unix)"),
        ),
    ];

    let manifest = manifest("Cargo.toml", true, deps, BTreeMap::new());
    let model = model(vec![manifest], BTreeMap::new());

    let cfg = config_with_check_allow(
        ids::CHECK_DEPS_DEFAULT_FEATURES_EXPLICIT,
        Severity::Warning,
        Vec::new(),
        false,
    );

    let mut out = Vec::new();
    default_features_explicit::run(&model, &cfg, &mut out);

    assert_eq!(out.len(), 2);
    assert_eq!(out[0].code, ids::CODE_DEFAULT_FEATURES_IMPLICIT);
    assert_eq!(out[1].code, ids::CODE_DEFAULT_FEATURES_IMPLICIT);
    assert_eq!(out[0].data["section"], "dependencies");
    assert_eq!(out[1].data["target"], "cfg(unix)");
}

#[test]
fn default_features_explicit_skips_when_default_features_present() {
    let deps = vec![
        dep_decl(
            "explicit",
            DepKind::Normal,
            DepSpec {
                optional: true,
                default_features: Some(false),
                ..DepSpec::default()
            },
            None,
        ),
        dep_decl(
            "implicit",
            DepKind::Normal,
            DepSpec {
                optional: true,
                ..DepSpec::default()
            },
            None,
        ),
    ];

    let manifest = manifest("Cargo.toml", true, deps, BTreeMap::new());
    let model = model(vec![manifest], BTreeMap::new());

    let cfg = config_with_check_allow(
        ids::CHECK_DEPS_DEFAULT_FEATURES_EXPLICIT,
        Severity::Warning,
        Vec::new(),
        false,
    );

    let mut out = Vec::new();
    default_features_explicit::run(&model, &cfg, &mut out);

    assert_eq!(out.len(), 1);
    assert_eq!(out[0].data["dependency"], "implicit");
}

#[test]
fn default_features_explicit_skips_simple_version_and_workspace_deps() {
    let deps = vec![
        dep_decl(
            "simple",
            DepKind::Normal,
            DepSpec {
                version: Some("1.0".to_string()),
                ..DepSpec::default()
            },
            None,
        ),
        dep_decl(
            "workspace_dep",
            DepKind::Normal,
            DepSpec {
                workspace: true,
                ..DepSpec::default()
            },
            None,
        ),
    ];

    let manifest = manifest("Cargo.toml", true, deps, BTreeMap::new());
    let model = model(vec![manifest], BTreeMap::new());

    let cfg = config_with_check_allow(
        ids::CHECK_DEPS_DEFAULT_FEATURES_EXPLICIT,
        Severity::Warning,
        Vec::new(),
        false,
    );

    let mut out = Vec::new();
    default_features_explicit::run(&model, &cfg, &mut out);
    assert!(out.is_empty());
}

#[test]
fn no_multiple_versions_detects_duplicates_and_skips_allowlist() {
    let deps_a = vec![
        dep_decl(
            "serde",
            DepKind::Normal,
            DepSpec {
                version: Some("1.0".to_string()),
                ..DepSpec::default()
            },
            None,
        ),
        dep_decl(
            "tokio",
            DepKind::Normal,
            DepSpec {
                version: Some("1.0".to_string()),
                ..DepSpec::default()
            },
            None,
        ),
        dep_decl(
            "allowme",
            DepKind::Normal,
            DepSpec {
                version: Some("1.0".to_string()),
                ..DepSpec::default()
            },
            None,
        ),
        dep_decl(
            "workspace_dep",
            DepKind::Normal,
            DepSpec {
                version: Some("9.9.9".to_string()),
                workspace: true,
                ..DepSpec::default()
            },
            None,
        ),
        dep_decl("noversion", DepKind::Normal, DepSpec::default(), None),
    ];

    let deps_b = vec![
        dep_decl(
            "serde",
            DepKind::Normal,
            DepSpec {
                version: Some("2.0".to_string()),
                ..DepSpec::default()
            },
            None,
        ),
        dep_decl(
            "tokio",
            DepKind::Normal,
            DepSpec {
                version: Some("1.0".to_string()),
                ..DepSpec::default()
            },
            None,
        ),
        dep_decl(
            "allowme",
            DepKind::Normal,
            DepSpec {
                version: Some("2.0".to_string()),
                ..DepSpec::default()
            },
            None,
        ),
    ];

    let manifests = vec![
        manifest("Cargo.toml", true, deps_a, BTreeMap::new()),
        manifest("crates/b/Cargo.toml", true, deps_b, BTreeMap::new()),
    ];
    let model = model(manifests, BTreeMap::new());

    let cfg = config_with_check_allow(
        ids::CHECK_DEPS_NO_MULTIPLE_VERSIONS,
        Severity::Warning,
        vec!["allowme"],
        false,
    );

    let mut out = Vec::new();
    no_multiple_versions::run(&model, &cfg, &mut out);
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].code, ids::CODE_DUPLICATE_DIFFERENT_VERSIONS);
    assert!(out[0].message.contains("serde"));
    assert!(out[0].message.contains("1.0"));
    assert!(out[0].message.contains("2.0"));
}

#[test]
fn optional_unused_detects_missing_feature_references() {
    let mut features = BTreeMap::new();
    features.insert(
        "feat".to_string(),
        vec![
            "dep:serde".to_string(),
            "toml/derive".to_string(),
            "url".to_string(),
        ],
    );

    let deps = vec![
        dep_decl(
            "serde",
            DepKind::Normal,
            DepSpec {
                optional: true,
                ..DepSpec::default()
            },
            None,
        ),
        dep_decl(
            "toml",
            DepKind::Normal,
            DepSpec {
                optional: true,
                ..DepSpec::default()
            },
            None,
        ),
        dep_decl(
            "url",
            DepKind::Normal,
            DepSpec {
                optional: true,
                ..DepSpec::default()
            },
            None,
        ),
        dep_decl(
            "rand",
            DepKind::Normal,
            DepSpec {
                optional: true,
                ..DepSpec::default()
            },
            Some("cfg(unix)"),
        ),
        dep_decl(
            "anyhow",
            DepKind::Normal,
            DepSpec {
                optional: true,
                ..DepSpec::default()
            },
            None,
        ),
        dep_decl(
            "log",
            DepKind::Normal,
            DepSpec {
                optional: false,
                ..DepSpec::default()
            },
            None,
        ),
    ];

    let manifest = manifest("Cargo.toml", true, deps, features);
    let model = model(vec![manifest], BTreeMap::new());

    let cfg = config_with_check_allow(
        ids::CHECK_DEPS_OPTIONAL_UNUSED,
        Severity::Warning,
        vec!["anyhow"],
        false,
    );

    let mut out = Vec::new();
    optional_unused::run(&model, &cfg, &mut out);
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].code, ids::CODE_OPTIONAL_NOT_IN_FEATURES);
    assert_eq!(out[0].data["target"], "cfg(unix)");
}

#[test]
fn utils_allowlist_and_section_helpers() {
    let empty: Vec<String> = Vec::new();
    assert!(utils::build_allowlist(&empty).is_none());

    let allow = utils::build_allowlist(&["foo*".to_string()]).expect("globset");
    assert!(utils::is_allowed(Some(&allow), "foobar"));
    assert!(!utils::is_allowed(Some(&allow), "bar"));
    assert!(!utils::is_allowed(None, "foobar"));

    assert_eq!(utils::section_name(DepKind::Normal), "dependencies");
    assert_eq!(utils::section_name(DepKind::Dev), "dev-dependencies");
    assert_eq!(utils::section_name(DepKind::Build), "build-dependencies");
}

#[test]
fn utils_spec_to_json_includes_all_fields() {
    let spec = DepSpec {
        version: Some("1.0".to_string()),
        path: Some("local/path".to_string()),
        workspace: true,
        git: Some("https://example.com/repo.git".to_string()),
        branch: Some("main".to_string()),
        tag: Some("v1.0.0".to_string()),
        rev: Some("deadbeef".to_string()),
        default_features: Some(false),
        optional: true,
    };

    let json = utils::spec_to_json(&spec);
    assert_eq!(json["version"], "1.0");
    assert_eq!(json["path"], "local/path");
    assert_eq!(json["workspace"], true);
    assert_eq!(json["git"], "https://example.com/repo.git");
    assert_eq!(json["branch"], "main");
    assert_eq!(json["tag"], "v1.0.0");
    assert_eq!(json["rev"], "deadbeef");
    assert_eq!(json["default-features"], false);
    assert_eq!(json["optional"], true);
}
