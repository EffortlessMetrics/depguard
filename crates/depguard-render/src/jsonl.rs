use crate::{RenderableReport, RenderableSeverity, RenderableVerdictStatus};
use serde_json::json;

pub fn render_jsonl(report: &RenderableReport) -> String {
    let mut lines = Vec::new();

    for finding in &report.findings {
        let (path, line, col) = if let Some(loc) = &finding.location {
            (Some(loc.path.clone()), loc.line, loc.col)
        } else {
            (None, None, None)
        };

        let obj = json!({
            "kind": "finding",
            "severity": severity_str(finding.severity),
            "check_id": finding.check_id,
            "code": finding.code,
            "message": finding.message,
            "path": path,
            "line": line,
            "col": col,
            "help": finding.help,
            "url": finding.url,
        });
        lines.push(obj.to_string());
    }

    let summary = json!({
        "kind": "summary",
        "verdict": verdict_str(report.verdict),
        "findings_emitted": report.data.findings_emitted,
        "findings_total": report.data.findings_total,
        "truncated_reason": report.data.truncated_reason,
    });
    lines.push(summary.to_string());

    let mut out = lines.join("\n");
    out.push('\n');
    out
}

fn severity_str(severity: RenderableSeverity) -> &'static str {
    match severity {
        RenderableSeverity::Info => "info",
        RenderableSeverity::Warning => "warning",
        RenderableSeverity::Error => "error",
    }
}

fn verdict_str(verdict: RenderableVerdictStatus) -> &'static str {
    match verdict {
        RenderableVerdictStatus::Pass => "pass",
        RenderableVerdictStatus::Warn => "warn",
        RenderableVerdictStatus::Fail => "fail",
        RenderableVerdictStatus::Skip => "skip",
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
    fn render_jsonl_emits_findings_and_summary() {
        let report = RenderableReport {
            verdict: RenderableVerdictStatus::Fail,
            findings: vec![RenderableFinding {
                severity: RenderableSeverity::Error,
                check_id: Some("deps.no_wildcards".to_string()),
                code: "wildcard_version".to_string(),
                message: "dependency uses wildcard".to_string(),
                location: Some(RenderableLocation {
                    path: "Cargo.toml".to_string(),
                    line: Some(8),
                    col: Some(1),
                }),
                help: Some("pin it".to_string()),
                url: Some("https://example.invalid/help".to_string()),
            }],
            data: RenderableData {
                findings_emitted: 1,
                findings_total: 1,
                truncated_reason: None,
            },
        };

        let output = render_jsonl(&report);
        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines.len(), 2);

        let finding: serde_json::Value = serde_json::from_str(lines[0]).expect("finding json");
        assert_eq!(finding["kind"], "finding");
        assert_eq!(finding["severity"], "error");
        assert_eq!(finding["path"], "Cargo.toml");
        assert_eq!(finding["line"], 8);

        let summary: serde_json::Value = serde_json::from_str(lines[1]).expect("summary json");
        assert_eq!(summary["kind"], "summary");
        assert_eq!(summary["verdict"], "fail");
        assert_eq!(summary["findings_emitted"], 1);
    }

    #[test]
    fn render_jsonl_empty_report_still_emits_summary() {
        let report = RenderableReport {
            verdict: RenderableVerdictStatus::Pass,
            findings: Vec::new(),
            data: RenderableData {
                findings_emitted: 0,
                findings_total: 0,
                truncated_reason: None,
            },
        };

        let output = render_jsonl(&report);
        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines.len(), 1);

        let summary: serde_json::Value = serde_json::from_str(lines[0]).expect("summary json");
        assert_eq!(summary["kind"], "summary");
        assert_eq!(summary["verdict"], "pass");
    }
}
