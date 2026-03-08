use crate::{RenderableReport, RenderableSeverity, RenderableVerdictStatus};
use std::collections::BTreeMap;

pub fn render_markdown(report: &RenderableReport) -> String {
    let mut out = String::new();

    out.push_str("# Depguard report\n\n");
    let verdict = match report.verdict {
        RenderableVerdictStatus::Pass => "PASS",
        RenderableVerdictStatus::Warn => "WARN",
        RenderableVerdictStatus::Fail => "FAIL",
        RenderableVerdictStatus::Skip => "SKIP",
    };
    out.push_str(&format!(
        "- Verdict: **{}**\n- Findings: {} (emitted) / {} (total)\n\n",
        verdict, report.data.findings_emitted, report.data.findings_total
    ));

    if let Some(r) = &report.data.truncated_reason {
        out.push_str(&format!("> Note: {}\n\n", r));
    }

    if report.findings.is_empty() {
        out.push_str("No findings.\n");
        return out;
    }

    // Add summary counts by severity
    let severity_counts = count_by_severity(&report.findings);
    out.push_str("## Summary\n\n");
    out.push_str(&format_severity_counts(&severity_counts));
    out.push_str("\n\n");

    // Group findings by severity for better readability
    out.push_str("## Findings\n\n");

    // Render findings grouped by severity (ERROR, WARN, INFO order)
    for severity in [
        RenderableSeverity::Error,
        RenderableSeverity::Warning,
        RenderableSeverity::Info,
    ] {
        let findings_for_severity: Vec<_> = report
            .findings
            .iter()
            .filter(|f| f.severity == severity)
            .collect();

        if findings_for_severity.is_empty() {
            continue;
        }

        let sev_label = match severity {
            RenderableSeverity::Error => "ERROR",
            RenderableSeverity::Warning => "WARNING",
            RenderableSeverity::Info => "INFO",
        };

        out.push_str(&format!("### {}\n\n", sev_label));

        // Group by check_id within each severity for even better organization
        let grouped = group_by_check_id(&findings_for_severity);
        for (check_id, findings) in grouped {
            if let Some(cid) = check_id {
                out.push_str(&format!("#### `{}`\n\n", cid));
            }
            for f in findings {
                render_finding(&mut out, f);
            }
        }
    }

    out
}

/// Count findings by severity level
fn count_by_severity(findings: &[crate::RenderableFinding]) -> BTreeMap<RenderableSeverity, usize> {
    let mut counts = BTreeMap::new();
    for f in findings {
        *counts.entry(f.severity).or_insert(0) += 1;
    }
    counts
}

/// Format severity counts as a human-readable string
fn format_severity_counts(counts: &BTreeMap<RenderableSeverity, usize>) -> String {
    let mut parts = Vec::new();

    // Order: errors, warnings, info
    if let Some(&n) = counts.get(&RenderableSeverity::Error) {
        parts.push(format!("{} error{}", n, if n == 1 { "" } else { "s" }));
    }
    if let Some(&n) = counts.get(&RenderableSeverity::Warning) {
        parts.push(format!("{} warning{}", n, if n == 1 { "" } else { "s" }));
    }
    if let Some(&n) = counts.get(&RenderableSeverity::Info) {
        parts.push(format!("{} info{}", n, if n == 1 { "" } else { "s" }));
    }

    if parts.is_empty() {
        String::from("0 findings")
    } else {
        parts.join(", ")
    }
}

/// Group findings by check_id, preserving order within each group
fn group_by_check_id<'a>(
    findings: &[&'a crate::RenderableFinding],
) -> Vec<(Option<&'a str>, Vec<&'a crate::RenderableFinding>)> {
    let mut groups: BTreeMap<Option<&'a str>, Vec<&'a crate::RenderableFinding>> = BTreeMap::new();

    for f in findings {
        let key = f.check_id.as_deref();
        groups.entry(key).or_default().push(*f);
    }

    // Convert to sorted vector for deterministic output
    let mut result: Vec<_> = groups.into_iter().collect();
    // Sort by check_id (None comes last, then alphabetically)
    result.sort_by(|a, b| match (&a.0, &b.0) {
        (None, None) => std::cmp::Ordering::Equal,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (Some(_), None) => std::cmp::Ordering::Less,
        (Some(a_id), Some(b_id)) => a_id.cmp(b_id),
    });
    result
}

/// Render a single finding to the output buffer
fn render_finding(out: &mut String, f: &crate::RenderableFinding) {
    if let Some(loc) = &f.location {
        // Create clickable markdown link for file path
        let file_link = format_file_link(&loc.path, loc.line);
        out.push_str(&format!(
            "- `{}` / `{}` — {} ({})\n",
            f.check_id.as_deref().unwrap_or(""),
            f.code,
            f.message,
            file_link
        ));
    } else {
        out.push_str(&format!(
            "- `{}` / `{}` — {}\n",
            f.check_id.as_deref().unwrap_or(""),
            f.code,
            f.message
        ));
    }

    if let Some(help) = &f.help {
        out.push_str(&format!("  - help: {}\n", help));
    }
    if let Some(url) = &f.url {
        out.push_str(&format!("  - url: {}\n", url));
    }
}

/// Format a file path as a clickable markdown link
fn format_file_link(path: &str, line: Option<u32>) -> String {
    match line {
        Some(l) => format!("[`{}`:{}]({}:L{})", path, l, path, l),
        None => format!("[`{}`]({})", path, path),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        RenderableData, RenderableFinding, RenderableLocation, RenderableSeverity,
        RenderableVerdictStatus,
    };

    #[test]
    fn renders_empty_report() {
        let report = RenderableReport {
            verdict: RenderableVerdictStatus::Pass,
            findings: Vec::new(),
            data: RenderableData {
                findings_emitted: 0,
                findings_total: 0,
                truncated_reason: None,
            },
        };
        let md = render_markdown(&report);
        assert!(md.contains("No findings"));
    }

    #[test]
    fn renders_findings_with_location_help_url_and_truncation() {
        let report = RenderableReport {
            verdict: RenderableVerdictStatus::Fail,
            findings: vec![RenderableFinding {
                severity: RenderableSeverity::Warning,
                check_id: Some("deps.no_wildcards".to_string()),
                code: "wildcard_version".to_string(),
                message: "bad dependency".to_string(),
                location: Some(RenderableLocation {
                    path: "Cargo.toml".to_string(),
                    line: Some(7),
                    col: None,
                }),
                help: Some("pin the version".to_string()),
                url: Some("https://example.com/docs".to_string()),
            }],
            data: RenderableData {
                findings_emitted: 1,
                findings_total: 2,
                truncated_reason: Some("truncated".to_string()),
            },
        };

        let md = render_markdown(&report);
        assert!(md.contains("Verdict: **FAIL**"));
        assert!(md.contains("> Note: truncated"));
        assert!(md.contains("## Findings"));
        assert!(md.contains("### WARNING"));
        assert!(md.contains("Cargo.toml"));
        assert!(md.contains("help: pin the version"));
        assert!(md.contains("url: https://example.com/docs"));
    }

    #[test]
    fn renders_skip_with_no_location() {
        let report = RenderableReport {
            verdict: RenderableVerdictStatus::Skip,
            findings: vec![RenderableFinding {
                severity: RenderableSeverity::Info,
                check_id: None,
                code: "info".to_string(),
                message: "skipped".to_string(),
                location: None,
                help: None,
                url: None,
            }],
            data: RenderableData {
                findings_emitted: 1,
                findings_total: 1,
                truncated_reason: None,
            },
        };

        let md = render_markdown(&report);
        assert!(md.contains("Verdict: **SKIP**"));
        assert!(md.contains("### INFO"));
        assert!(md.contains("skipped"));
    }

    #[test]
    fn renders_warn_verdict() {
        let report = RenderableReport {
            verdict: RenderableVerdictStatus::Warn,
            findings: Vec::new(),
            data: RenderableData {
                findings_emitted: 0,
                findings_total: 0,
                truncated_reason: None,
            },
        };

        let md = render_markdown(&report);
        assert!(md.contains("Verdict: **WARN**"));
    }

    #[test]
    fn renders_summary_counts() {
        let report = RenderableReport {
            verdict: RenderableVerdictStatus::Fail,
            findings: vec![
                RenderableFinding {
                    severity: RenderableSeverity::Error,
                    check_id: Some("check.a".to_string()),
                    code: "code1".to_string(),
                    message: "error 1".to_string(),
                    location: None,
                    help: None,
                    url: None,
                },
                RenderableFinding {
                    severity: RenderableSeverity::Error,
                    check_id: Some("check.a".to_string()),
                    code: "code2".to_string(),
                    message: "error 2".to_string(),
                    location: None,
                    help: None,
                    url: None,
                },
                RenderableFinding {
                    severity: RenderableSeverity::Warning,
                    check_id: Some("check.b".to_string()),
                    code: "code3".to_string(),
                    message: "warning 1".to_string(),
                    location: None,
                    help: None,
                    url: None,
                },
                RenderableFinding {
                    severity: RenderableSeverity::Info,
                    check_id: Some("check.c".to_string()),
                    code: "code4".to_string(),
                    message: "info 1".to_string(),
                    location: None,
                    help: None,
                    url: None,
                },
            ],
            data: RenderableData {
                findings_emitted: 4,
                findings_total: 4,
                truncated_reason: None,
            },
        };

        let md = render_markdown(&report);
        assert!(md.contains("## Summary"));
        assert!(md.contains("2 errors, 1 warning, 1 info"));
    }

    #[test]
    fn renders_singular_severity_labels() {
        let report = RenderableReport {
            verdict: RenderableVerdictStatus::Fail,
            findings: vec![RenderableFinding {
                severity: RenderableSeverity::Error,
                check_id: Some("check.a".to_string()),
                code: "code1".to_string(),
                message: "error 1".to_string(),
                location: None,
                help: None,
                url: None,
            }],
            data: RenderableData {
                findings_emitted: 1,
                findings_total: 1,
                truncated_reason: None,
            },
        };

        let md = render_markdown(&report);
        assert!(md.contains("1 error")); // singular, not "errors"
        assert!(!md.contains("1 errors"));
    }

    #[test]
    fn groups_findings_by_severity() {
        let report = RenderableReport {
            verdict: RenderableVerdictStatus::Fail,
            findings: vec![
                RenderableFinding {
                    severity: RenderableSeverity::Info,
                    check_id: Some("check.c".to_string()),
                    code: "code3".to_string(),
                    message: "info message".to_string(),
                    location: None,
                    help: None,
                    url: None,
                },
                RenderableFinding {
                    severity: RenderableSeverity::Error,
                    check_id: Some("check.a".to_string()),
                    code: "code1".to_string(),
                    message: "error message".to_string(),
                    location: None,
                    help: None,
                    url: None,
                },
            ],
            data: RenderableData {
                findings_emitted: 2,
                findings_total: 2,
                truncated_reason: None,
            },
        };

        let md = render_markdown(&report);
        // ERROR section should come before INFO section
        let error_pos = md.find("### ERROR").expect("ERROR section not found");
        let info_pos = md.find("### INFO").expect("INFO section not found");
        assert!(error_pos < info_pos, "ERROR should come before INFO");
    }

    #[test]
    fn groups_findings_by_check_id() {
        let report = RenderableReport {
            verdict: RenderableVerdictStatus::Fail,
            findings: vec![
                RenderableFinding {
                    severity: RenderableSeverity::Error,
                    check_id: Some("deps.check_b".to_string()),
                    code: "code2".to_string(),
                    message: "error from check_b".to_string(),
                    location: None,
                    help: None,
                    url: None,
                },
                RenderableFinding {
                    severity: RenderableSeverity::Error,
                    check_id: Some("deps.check_a".to_string()),
                    code: "code1".to_string(),
                    message: "error from check_a".to_string(),
                    location: None,
                    help: None,
                    url: None,
                },
                RenderableFinding {
                    severity: RenderableSeverity::Error,
                    check_id: Some("deps.check_a".to_string()),
                    code: "code1b".to_string(),
                    message: "another error from check_a".to_string(),
                    location: None,
                    help: None,
                    url: None,
                },
            ],
            data: RenderableData {
                findings_emitted: 3,
                findings_total: 3,
                truncated_reason: None,
            },
        };

        let md = render_markdown(&report);
        // check_a should come before check_b (alphabetically)
        let check_a_pos = md
            .find("#### `deps.check_a`")
            .expect("check_a section not found");
        let check_b_pos = md
            .find("#### `deps.check_b`")
            .expect("check_b section not found");
        assert!(
            check_a_pos < check_b_pos,
            "check_a should come before check_b alphabetically"
        );

        // Both check_a findings should be under the same heading
        assert!(md.contains("error from check_a"));
        assert!(md.contains("another error from check_a"));
    }

    #[test]
    fn renders_clickable_file_links() {
        let report = RenderableReport {
            verdict: RenderableVerdictStatus::Fail,
            findings: vec![RenderableFinding {
                severity: RenderableSeverity::Error,
                check_id: Some("deps.check".to_string()),
                code: "code1".to_string(),
                message: "error message".to_string(),
                location: Some(RenderableLocation {
                    path: "src/main.rs".to_string(),
                    line: Some(42),
                    col: None,
                }),
                help: None,
                url: None,
            }],
            data: RenderableData {
                findings_emitted: 1,
                findings_total: 1,
                truncated_reason: None,
            },
        };

        let md = render_markdown(&report);
        // Should contain clickable markdown link format
        assert!(md.contains("[`src/main.rs`:42](src/main.rs:L42)"));
    }

    #[test]
    fn renders_file_link_without_line() {
        let report = RenderableReport {
            verdict: RenderableVerdictStatus::Fail,
            findings: vec![RenderableFinding {
                severity: RenderableSeverity::Error,
                check_id: Some("deps.check".to_string()),
                code: "code1".to_string(),
                message: "error message".to_string(),
                location: Some(RenderableLocation {
                    path: "Cargo.toml".to_string(),
                    line: None,
                    col: None,
                }),
                help: None,
                url: None,
            }],
            data: RenderableData {
                findings_emitted: 1,
                findings_total: 1,
                truncated_reason: None,
            },
        };

        let md = render_markdown(&report);
        // Should contain clickable link without line number
        assert!(md.contains("[`Cargo.toml`](Cargo.toml)"));
    }

    #[test]
    fn handles_findings_with_no_check_id() {
        let report = RenderableReport {
            verdict: RenderableVerdictStatus::Fail,
            findings: vec![
                RenderableFinding {
                    severity: RenderableSeverity::Error,
                    check_id: None,
                    code: "code1".to_string(),
                    message: "error without check_id".to_string(),
                    location: None,
                    help: None,
                    url: None,
                },
                RenderableFinding {
                    severity: RenderableSeverity::Error,
                    check_id: Some("deps.check".to_string()),
                    code: "code2".to_string(),
                    message: "error with check_id".to_string(),
                    location: None,
                    help: None,
                    url: None,
                },
            ],
            data: RenderableData {
                findings_emitted: 2,
                findings_total: 2,
                truncated_reason: None,
            },
        };

        let md = render_markdown(&report);
        // Findings with check_id should be grouped under a heading
        assert!(md.contains("#### `deps.check`"));
        // Finding without check_id should still be rendered
        assert!(md.contains("error without check_id"));
    }

    #[test]
    fn output_is_deterministic() {
        // Create findings in non-deterministic order
        let findings = vec![
            RenderableFinding {
                severity: RenderableSeverity::Warning,
                check_id: Some("deps.b".to_string()),
                code: "code2".to_string(),
                message: "warning b".to_string(),
                location: None,
                help: None,
                url: None,
            },
            RenderableFinding {
                severity: RenderableSeverity::Error,
                check_id: Some("deps.a".to_string()),
                code: "code1".to_string(),
                message: "error a".to_string(),
                location: None,
                help: None,
                url: None,
            },
            RenderableFinding {
                severity: RenderableSeverity::Info,
                check_id: Some("deps.c".to_string()),
                code: "code3".to_string(),
                message: "info c".to_string(),
                location: None,
                help: None,
                url: None,
            },
        ];

        let report = RenderableReport {
            verdict: RenderableVerdictStatus::Fail,
            findings: findings.clone(),
            data: RenderableData {
                findings_emitted: 3,
                findings_total: 3,
                truncated_reason: None,
            },
        };

        // Render multiple times and ensure output is identical
        let md1 = render_markdown(&report);
        let md2 = render_markdown(&report);
        let md3 = render_markdown(&report);

        assert_eq!(md1, md2);
        assert_eq!(md2, md3);
    }
}
