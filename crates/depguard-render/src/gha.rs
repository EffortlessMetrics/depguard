use crate::{RenderableReport, RenderableSeverity};

/// Render findings as GitHub Actions workflow command annotations.
///
/// Format:
/// `::{level} file={path},line={line},col={col}::{message}`
pub fn render_github_annotations(report: &RenderableReport) -> Vec<String> {
    let mut out = Vec::new();

    for f in &report.findings {
        let level = match f.severity {
            RenderableSeverity::Error => "error",
            RenderableSeverity::Warning => "warning",
            RenderableSeverity::Info => "notice",
        };

        let mut meta = String::new();
        if let Some(loc) = &f.location {
            meta.push_str(&format!("file={}", loc.path.as_str()));
            if let Some(line) = loc.line {
                meta.push_str(&format!(",line={}", line));
            }
            if let Some(col) = loc.col {
                meta.push_str(&format!(",col={}", col));
            }
        }

        let check_id = f.check_id.as_deref().unwrap_or("depguard");
        let message = format!("[{}:{}] {}", check_id, f.code, f.message)
            .replace('%', "%25")
            .replace('\r', "%0D")
            .replace('\n', "%0A");

        if meta.is_empty() {
            out.push(format!("::{}::{}", level, message));
        } else {
            out.push(format!("::{} {}::{}", level, meta, message));
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        RenderableData, RenderableFinding, RenderableLocation, RenderableReport,
        RenderableSeverity, RenderableVerdictStatus,
    };

    #[test]
    fn annotations_escape_and_include_metadata() {
        let report = RenderableReport {
            verdict: RenderableVerdictStatus::Fail,
            findings: vec![
                RenderableFinding {
                    severity: RenderableSeverity::Error,
                    check_id: Some("deps.no_wildcards".to_string()),
                    code: "wildcard_version".to_string(),
                    message: "bad%line\r\nnext".to_string(),
                    location: Some(RenderableLocation {
                        path: "Cargo.toml".to_string(),
                        line: Some(2),
                        col: Some(3),
                    }),
                    help: None,
                    url: None,
                },
                RenderableFinding {
                    severity: RenderableSeverity::Info,
                    check_id: None,
                    code: "info".to_string(),
                    message: "ok".to_string(),
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

        let annotations = render_github_annotations(&report);
        assert_eq!(
            annotations[0],
            "::error file=Cargo.toml,line=2,col=3::[deps.no_wildcards:wildcard_version] bad%25line%0D%0Anext"
        );
        assert_eq!(annotations[1], "::notice::[depguard:info] ok");
    }

    #[test]
    fn annotations_warning_with_partial_location_and_default_check_id() {
        let report = RenderableReport {
            verdict: RenderableVerdictStatus::Warn,
            findings: vec![RenderableFinding {
                severity: RenderableSeverity::Warning,
                check_id: None,
                code: "warn_code".to_string(),
                message: "be careful".to_string(),
                location: Some(RenderableLocation {
                    path: "src/lib.rs".to_string(),
                    line: Some(10),
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

        let annotations = render_github_annotations(&report);
        assert_eq!(
            annotations[0],
            "::warning file=src/lib.rs,line=10::[depguard:warn_code] be careful"
        );
    }
}
