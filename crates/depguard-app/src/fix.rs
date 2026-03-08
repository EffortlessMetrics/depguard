use crate::report::ReportVariant;
use anyhow::Context;
use camino::{Utf8Path, Utf8PathBuf};
use depguard_types::{
    BuildfixAction, BuildfixActionTarget, BuildfixActionType, BuildfixConfidence,
    BuildfixFindingRef, BuildfixFixAction, BuildfixLocation, BuildfixMetadata, BuildfixPlanV1,
    BuildfixSafety, BuildfixSourceReport, Location, SCHEMA_BUILDFIX_PLAN_V1, ids,
};
use serde_json::Value as JsonValue;
use std::collections::{BTreeMap, BTreeSet};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use toml_edit::{DocumentMut, Item, Value as TomlValue, value as toml_value};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FixApplyResult {
    pub planned: u32,
    pub applied: u32,
    pub skipped: u32,
    pub failed: u32,
}

#[derive(Clone, Debug)]
struct SafeFixCandidate {
    check_id: String,
    code: String,
    fingerprint: Option<String>,
    location: Option<Location>,
    manifest: String,
    section: String,
    dependency: String,
    target: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct SafeFixKey {
    manifest: String,
    section: String,
    dependency: String,
    target: Option<String>,
}

pub fn generate_buildfix_plan(
    report: &ReportVariant,
    report_path: &str,
    dry_run: bool,
) -> BuildfixPlanV1 {
    let source = BuildfixSourceReport {
        tool: "depguard".to_string(),
        report_path: report_path.to_string(),
        report_schema: Some(report_schema(report).to_string()),
    };

    let fixes = collect_safe_fix_candidates(report)
        .into_iter()
        .map(|candidate| {
            let line = candidate.location.as_ref().and_then(|loc| loc.line);
            let col = candidate.location.as_ref().and_then(|loc| loc.col);

            BuildfixFixAction {
                finding_ref: BuildfixFindingRef {
                    tool: Some("depguard".to_string()),
                    check_id: candidate.check_id.clone(),
                    code: candidate.code.clone(),
                    fingerprint: candidate.fingerprint.clone(),
                    location: Some(BuildfixLocation {
                        path: Some(candidate.manifest.clone()),
                        line,
                        col,
                    }),
                    safety: Some(BuildfixSafety::Safe),
                    preconditions: None,
                },
                action: BuildfixAction {
                    action_type: BuildfixActionType::Insert,
                    target: Some(BuildfixActionTarget {
                        path: Some(candidate.manifest.clone()),
                        line_start: line,
                        line_end: line,
                        col_start: col,
                        col_end: col,
                        pattern: Some(format!(
                            "{}{}{}",
                            candidate.section,
                            candidate
                                .target
                                .as_ref()
                                .map(|target| format!(".{}.", target))
                                .unwrap_or_else(|| ".".to_string()),
                            candidate.dependency
                        )),
                    }),
                    content: Some("default-features = true".to_string()),
                    command: None,
                    description: Some(format!(
                        "Add default-features = true for dependency '{}' in [{}]",
                        candidate.dependency, candidate.section
                    )),
                },
                confidence: Some(BuildfixConfidence::High),
                requires_review: Some(false),
            }
        })
        .collect();

    BuildfixPlanV1 {
        schema: SCHEMA_BUILDFIX_PLAN_V1.to_string(),
        source,
        fixes,
        metadata: Some(BuildfixMetadata {
            generated_at: Some(
                OffsetDateTime::now_utc()
                    .format(&Rfc3339)
                    .unwrap_or_else(|_| OffsetDateTime::now_utc().to_string()),
            ),
            generator: Some(format!("depguard {}", env!("CARGO_PKG_VERSION"))),
            dry_run: Some(dry_run),
        }),
    }
}

pub fn serialize_buildfix_plan(plan: &BuildfixPlanV1) -> anyhow::Result<Vec<u8>> {
    serde_json::to_vec_pretty(plan).context(
        "Failed to serialize buildfix plan to JSON. \
         This is an internal error - the plan data structure may contain invalid values. \
         Please report this issue if it persists.",
    )
}

pub fn apply_safe_fixes(repo_root: &Utf8Path, report: &ReportVariant) -> FixApplyResult {
    let candidates = collect_safe_fix_candidates(report);
    let mut result = FixApplyResult {
        planned: candidates.len() as u32,
        ..FixApplyResult::default()
    };

    let mut by_manifest: BTreeMap<String, Vec<SafeFixCandidate>> = BTreeMap::new();
    for candidate in candidates {
        by_manifest
            .entry(candidate.manifest.clone())
            .or_default()
            .push(candidate);
    }

    for (manifest, fixes) in by_manifest {
        let manifest_path = match normalize_manifest_path(repo_root, &manifest) {
            Ok(p) => p,
            Err(_) => {
                result.failed += fixes.len() as u32;
                continue;
            }
        };
        let text = match std::fs::read_to_string(&manifest_path) {
            Ok(text) => text,
            Err(_) => {
                result.failed += fixes.len() as u32;
                continue;
            }
        };

        let mut doc = match text.parse::<DocumentMut>() {
            Ok(doc) => doc,
            Err(_) => {
                result.failed += fixes.len() as u32;
                continue;
            }
        };

        let mut file_applied = 0u32;
        let mut file_skipped = 0u32;
        for fix in &fixes {
            if apply_default_features_fix(&mut doc, fix) {
                file_applied += 1;
            } else {
                file_skipped += 1;
            }
        }

        if file_applied == 0 {
            result.skipped += file_skipped;
            continue;
        }

        if std::fs::write(&manifest_path, doc.to_string()).is_err() {
            result.failed += fixes.len() as u32;
            continue;
        }

        result.applied += file_applied;
        result.skipped += file_skipped;
    }

    result
}

fn report_schema(report: &ReportVariant) -> &str {
    match report {
        ReportVariant::V1(report) => report.schema.as_str(),
        ReportVariant::V2(report) => report.schema.as_str(),
    }
}

fn collect_safe_fix_candidates(report: &ReportVariant) -> Vec<SafeFixCandidate> {
    let mut out = Vec::new();
    let mut seen = BTreeSet::new();

    visit_findings(report, |finding| {
        let Some(candidate) = candidate_from_finding(
            finding.check_id,
            finding.code,
            finding.fingerprint,
            finding.location,
            finding.data,
        ) else {
            return;
        };

        let key = SafeFixKey {
            manifest: candidate.manifest.clone(),
            section: candidate.section.clone(),
            dependency: candidate.dependency.clone(),
            target: candidate.target.clone(),
        };
        if seen.insert(key) {
            out.push(candidate);
        }
    });

    out
}

fn candidate_from_finding(
    check_id: &str,
    code: &str,
    fingerprint: Option<&str>,
    location: Option<&Location>,
    data: &JsonValue,
) -> Option<SafeFixCandidate> {
    let action = data.get("fix_action").and_then(JsonValue::as_str)?;
    if action != ids::FIX_ACTION_ADD_DEFAULT_FEATURES {
        return None;
    }

    let manifest = data.get("manifest").and_then(JsonValue::as_str)?.trim();
    let section = data.get("section").and_then(JsonValue::as_str)?.trim();
    let dependency = data.get("dependency").and_then(JsonValue::as_str)?.trim();
    if manifest.is_empty() || dependency.is_empty() {
        return None;
    }
    if !matches!(
        section,
        "dependencies" | "dev-dependencies" | "build-dependencies"
    ) {
        return None;
    }

    Some(SafeFixCandidate {
        check_id: check_id.to_string(),
        code: code.to_string(),
        fingerprint: fingerprint.map(str::to_string),
        location: location.cloned(),
        manifest: manifest.to_string(),
        section: section.to_string(),
        dependency: dependency.to_string(),
        target: data
            .get("target")
            .and_then(JsonValue::as_str)
            .map(str::to_string),
    })
}

fn normalize_manifest_path(repo_root: &Utf8Path, path: &str) -> anyhow::Result<Utf8PathBuf> {
    let path = Utf8PathBuf::from(path);
    if path.is_absolute() {
        anyhow::bail!(
            "Manifest path '{}' is absolute but must be relative to the repository root '{}'. \
             Update the manifest path in your report or regenerate the report from the correct repository.",
            path,
            repo_root
        );
    }
    for component in path.components() {
        if let camino::Utf8Component::ParentDir = component {
            anyhow::bail!(
                "Manifest path '{}' contains '..' which is not allowed for security reasons. \
                 Paths must stay within the repository root '{}'. \
                 This may indicate a malformed report or attempted path traversal.",
                path,
                repo_root
            );
        }
    }
    Ok(repo_root.join(path))
}

fn apply_default_features_fix(doc: &mut DocumentMut, candidate: &SafeFixCandidate) -> bool {
    let Some(dep_item) = dependency_item_mut(
        doc,
        &candidate.section,
        candidate.target.as_deref(),
        &candidate.dependency,
    ) else {
        return false;
    };

    match dep_item {
        Item::Value(TomlValue::InlineTable(table)) => {
            if table.get("default-features").is_some() {
                return false;
            }
            table.insert("default-features", TomlValue::from(true));
            true
        }
        Item::Value(TomlValue::String(version)) => {
            // Convert `dep = "1.0"` → `dep = { version = "1.0", default-features = true }`
            let ver = version.value().to_string();
            let mut table = toml_edit::InlineTable::new();
            table.insert("version", TomlValue::from(ver));
            table.insert("default-features", TomlValue::from(true));
            *dep_item = Item::Value(TomlValue::InlineTable(table));
            true
        }
        Item::Table(table) => {
            if table.get("default-features").is_some() {
                return false;
            }
            table.insert("default-features", toml_value(true));
            true
        }
        _ => false,
    }
}

fn dependency_item_mut<'a>(
    doc: &'a mut DocumentMut,
    section: &str,
    target: Option<&str>,
    dependency: &str,
) -> Option<&'a mut Item> {
    if let Some(target) = target {
        let target_root = doc.get_mut("target")?.as_table_like_mut()?;
        let target_table = target_root.get_mut(target)?.as_table_like_mut()?;
        let section_table = target_table.get_mut(section)?.as_table_like_mut()?;
        return section_table.get_mut(dependency);
    }

    let section_table = doc.get_mut(section)?.as_table_like_mut()?;
    section_table.get_mut(dependency)
}

struct FindingView<'a> {
    check_id: &'a str,
    code: &'a str,
    fingerprint: Option<&'a str>,
    location: Option<&'a Location>,
    data: &'a JsonValue,
}

fn visit_findings(report: &ReportVariant, mut cb: impl FnMut(FindingView<'_>)) {
    match report {
        ReportVariant::V1(report) => {
            for finding in &report.findings {
                cb(FindingView {
                    check_id: &finding.check_id,
                    code: &finding.code,
                    fingerprint: finding.fingerprint.as_deref(),
                    location: finding.location.as_ref(),
                    data: &finding.data,
                });
            }
        }
        ReportVariant::V2(report) => {
            for finding in &report.findings {
                cb(FindingView {
                    check_id: &finding.check_id,
                    code: &finding.code,
                    fingerprint: finding.fingerprint.as_deref(),
                    location: finding.location.as_ref(),
                    data: &finding.data,
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::{ReportVersion, empty_report};
    use depguard_types::{Location, SeverityV2};
    use serde_json::json;

    #[test]
    fn generate_buildfix_plan_includes_safe_default_features_fix() {
        let mut report_variant = empty_report(ReportVersion::V2, "repo", "strict");
        let ReportVariant::V2(ref mut report) = report_variant else {
            panic!("expected v2 report")
        };
        report.findings.push(depguard_types::FindingV2 {
            severity: SeverityV2::Warn,
            check_id: ids::CHECK_DEPS_DEFAULT_FEATURES_EXPLICIT.to_string(),
            code: ids::CODE_DEFAULT_FEATURES_IMPLICIT.to_string(),
            message: "missing default-features".to_string(),
            location: Some(Location {
                path: depguard_types::RepoPath::new("Cargo.toml"),
                line: Some(8),
                col: None,
            }),
            help: None,
            url: None,
            fingerprint: Some("fp-default-features".to_string()),
            data: json!({
                "dependency": "serde",
                "manifest": "Cargo.toml",
                "section": "dependencies",
                "fix_action": ids::FIX_ACTION_ADD_DEFAULT_FEATURES,
            }),
        });

        let plan = generate_buildfix_plan(&report_variant, "artifacts/depguard/report.json", true);
        assert_eq!(plan.schema, SCHEMA_BUILDFIX_PLAN_V1);
        assert_eq!(plan.fixes.len(), 1);
        assert_eq!(
            plan.fixes[0].finding_ref.check_id,
            "deps.default_features_explicit"
        );
        assert_eq!(
            plan.fixes[0].action.content.as_deref(),
            Some("default-features = true")
        );
    }

    #[test]
    fn apply_safe_fixes_adds_default_features() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = Utf8Path::from_path(tmp.path()).expect("utf8");
        std::fs::write(
            root.join("Cargo.toml"),
            r#"[package]
name = "demo"
version = "0.1.0"

[dependencies]
serde = { version = "1.0", optional = true }
"#,
        )
        .expect("write Cargo.toml");

        let mut report_variant = empty_report(ReportVersion::V2, "repo", "strict");
        let ReportVariant::V2(ref mut report) = report_variant else {
            panic!("expected v2 report")
        };
        report.findings.push(depguard_types::FindingV2 {
            severity: SeverityV2::Warn,
            check_id: ids::CHECK_DEPS_DEFAULT_FEATURES_EXPLICIT.to_string(),
            code: ids::CODE_DEFAULT_FEATURES_IMPLICIT.to_string(),
            message: "missing default-features".to_string(),
            location: Some(Location {
                path: depguard_types::RepoPath::new("Cargo.toml"),
                line: Some(6),
                col: None,
            }),
            help: None,
            url: None,
            fingerprint: Some("fp-default-features".to_string()),
            data: json!({
                "dependency": "serde",
                "manifest": "Cargo.toml",
                "section": "dependencies",
                "fix_action": ids::FIX_ACTION_ADD_DEFAULT_FEATURES,
            }),
        });

        let result = apply_safe_fixes(root, &report_variant);
        assert_eq!(
            result,
            FixApplyResult {
                planned: 1,
                applied: 1,
                skipped: 0,
                failed: 0,
            }
        );

        let updated = std::fs::read_to_string(root.join("Cargo.toml")).expect("read Cargo.toml");
        assert!(updated.contains("default-features = true"));
    }

    #[test]
    fn apply_safe_fixes_skips_already_explicit_default_features() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = Utf8Path::from_path(tmp.path()).expect("utf8");
        std::fs::write(
            root.join("Cargo.toml"),
            r#"[package]
name = "demo"
version = "0.1.0"

[dependencies]
serde = { version = "1.0", optional = true, default-features = false }
"#,
        )
        .expect("write Cargo.toml");

        let mut report_variant = empty_report(ReportVersion::V2, "repo", "strict");
        let ReportVariant::V2(ref mut report) = report_variant else {
            panic!("expected v2 report")
        };
        report.findings.push(depguard_types::FindingV2 {
            severity: SeverityV2::Warn,
            check_id: ids::CHECK_DEPS_DEFAULT_FEATURES_EXPLICIT.to_string(),
            code: ids::CODE_DEFAULT_FEATURES_IMPLICIT.to_string(),
            message: "missing default-features".to_string(),
            location: Some(Location {
                path: depguard_types::RepoPath::new("Cargo.toml"),
                line: Some(6),
                col: None,
            }),
            help: None,
            url: None,
            fingerprint: Some("fp-default-features".to_string()),
            data: json!({
                "dependency": "serde",
                "manifest": "Cargo.toml",
                "section": "dependencies",
                "fix_action": ids::FIX_ACTION_ADD_DEFAULT_FEATURES,
            }),
        });

        let result = apply_safe_fixes(root, &report_variant);
        assert_eq!(
            result,
            FixApplyResult {
                planned: 1,
                applied: 0,
                skipped: 1,
                failed: 0,
            }
        );
    }
}
