use crate::{RenderableReport, RenderableSeverity, RenderableVerdictStatus};

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

    out.push_str("## Findings\n\n");

    for f in &report.findings {
        let sev = match f.severity {
            RenderableSeverity::Info => "INFO",
            RenderableSeverity::Warning => "WARN",
            RenderableSeverity::Error => "ERROR",
        };

        if let Some(loc) = &f.location {
            out.push_str(&format!(
                "- [{}] `{}` / `{}` — {} (`{}`:{} )\n",
                sev,
                f.check_id.as_deref().unwrap_or(""),
                f.code,
                f.message,
                loc.path.as_str(),
                loc.line.unwrap_or(0)
            ));
        } else {
            out.push_str(&format!(
                "- [{}] `{}` / `{}` — {}\n",
                sev,
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

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{RenderableData, RenderableVerdictStatus};

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
}
