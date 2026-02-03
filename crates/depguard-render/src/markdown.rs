use depguard_types::{DepguardReport, Severity};

pub fn render_markdown(report: &DepguardReport) -> String {
    let mut out = String::new();

    out.push_str("# Depguard report\n\n");
    out.push_str(&format!(
        "- Verdict: **{:?}**\n- Findings: {} (emitted) / {} (total)\n\n",
        report.verdict, report.data.findings_emitted, report.data.findings_total
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
            Severity::Info => "INFO",
            Severity::Warning => "WARN",
            Severity::Error => "ERROR",
        };

        if let Some(loc) = &f.location {
            out.push_str(&format!(
                "- [{}] `{}` / `{}` — {} (`{}`:{} )\n",
                sev,
                f.check_id,
                f.code,
                f.message,
                loc.path.as_str(),
                loc.line.unwrap_or(0)
            ));
        } else {
            out.push_str(&format!(
                "- [{}] `{}` / `{}` — {}\n",
                sev, f.check_id, f.code, f.message
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
    use depguard_types::{DepguardData, ReportEnvelope, ToolMeta, Verdict};
    use time::macros::datetime;

    #[test]
    fn renders_empty_report() {
        let report: DepguardReport = ReportEnvelope {
            schema: "receipt.envelope.v1".to_string(),
            tool: ToolMeta {
                name: "depguard".to_string(),
                version: "0.0.0".to_string(),
            },
            started_at: datetime!(2024-01-01 0:00 UTC),
            finished_at: datetime!(2024-01-01 0:00 UTC),
            verdict: Verdict::Pass,
            findings: Vec::new(),
            data: DepguardData::default(),
        };
        let md = render_markdown(&report);
        assert!(md.contains("No findings"));
    }
}
