use crate::{RenderableReport, RenderableSeverity, RenderableVerdictStatus};

pub fn render_junit(report: &RenderableReport) -> String {
    let tests = report.findings.len();
    let failures = report
        .findings
        .iter()
        .filter(|f| {
            matches!(
                f.severity,
                RenderableSeverity::Warning | RenderableSeverity::Error
            )
        })
        .count();
    let skipped = report
        .findings
        .iter()
        .filter(|f| matches!(f.severity, RenderableSeverity::Info))
        .count();

    let verdict = match report.verdict {
        RenderableVerdictStatus::Pass => "pass",
        RenderableVerdictStatus::Warn => "warn",
        RenderableVerdictStatus::Fail => "fail",
        RenderableVerdictStatus::Skip => "skip",
    };

    let mut out = String::new();
    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    out.push_str(&format!(
        "<testsuite name=\"depguard\" tests=\"{}\" failures=\"{}\" skipped=\"{}\">\n",
        tests, failures, skipped
    ));
    out.push_str("  <properties>\n");
    out.push_str(&format!(
        "    <property name=\"depguard.verdict\" value=\"{}\"/>\n",
        verdict
    ));
    out.push_str(&format!(
        "    <property name=\"depguard.findings_emitted\" value=\"{}\"/>\n",
        report.data.findings_emitted
    ));
    out.push_str(&format!(
        "    <property name=\"depguard.findings_total\" value=\"{}\"/>\n",
        report.data.findings_total
    ));
    if let Some(reason) = &report.data.truncated_reason {
        out.push_str(&format!(
            "    <property name=\"depguard.truncated_reason\" value=\"{}\"/>\n",
            xml_escape(reason)
        ));
    }
    out.push_str("  </properties>\n");

    for finding in &report.findings {
        let class_name = xml_escape(finding.check_id.as_deref().unwrap_or("depguard.unknown"));
        let test_name = if let Some(loc) = &finding.location {
            match loc.line {
                Some(line) => format!("{} [{}:{}]", finding.code, loc.path, line),
                None => format!("{} [{}]", finding.code, loc.path),
            }
        } else {
            finding.code.clone()
        };
        out.push_str(&format!(
            "  <testcase classname=\"{}\" name=\"{}\">",
            class_name,
            xml_escape(&test_name)
        ));

        match finding.severity {
            RenderableSeverity::Info => {
                out.push_str("<skipped/>");
            }
            RenderableSeverity::Warning | RenderableSeverity::Error => {
                let failure_type = match finding.severity {
                    RenderableSeverity::Warning => "warning",
                    RenderableSeverity::Error => "error",
                    RenderableSeverity::Info => "info",
                };
                out.push_str(&format!(
                    "<failure type=\"{}\" message=\"{}\">",
                    failure_type,
                    xml_escape(&finding.message)
                ));
                out.push_str(&xml_escape(&failure_body(finding)));
                out.push_str("</failure>");
            }
        }

        out.push_str("</testcase>\n");
    }

    out.push_str("</testsuite>\n");
    out
}

fn failure_body(finding: &crate::RenderableFinding) -> String {
    let mut body = String::new();
    body.push_str(&finding.message);
    if let Some(loc) = &finding.location {
        body.push_str("\nlocation: ");
        body.push_str(&loc.path);
        if let Some(line) = loc.line {
            body.push(':');
            body.push_str(&line.to_string());
            if let Some(col) = loc.col {
                body.push(':');
                body.push_str(&col.to_string());
            }
        }
    }
    if let Some(help) = &finding.help {
        body.push_str("\nhelp: ");
        body.push_str(help);
    }
    if let Some(url) = &finding.url {
        body.push_str("\nurl: ");
        body.push_str(url);
    }
    body
}

fn xml_escape(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(ch),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{RenderableData, RenderableFinding, RenderableLocation};

    #[test]
    fn render_junit_includes_failure_and_skip() {
        let report = RenderableReport {
            verdict: RenderableVerdictStatus::Fail,
            findings: vec![
                RenderableFinding {
                    severity: RenderableSeverity::Error,
                    check_id: Some("deps.no_wildcards".to_string()),
                    code: "wildcard_version".to_string(),
                    message: "dependency uses wildcard".to_string(),
                    location: Some(RenderableLocation {
                        path: "Cargo.toml".to_string(),
                        line: Some(8),
                        col: Some(2),
                    }),
                    help: Some("pin the version".to_string()),
                    url: Some("https://example.invalid/help".to_string()),
                },
                RenderableFinding {
                    severity: RenderableSeverity::Info,
                    check_id: Some("deps.info".to_string()),
                    code: "note".to_string(),
                    message: "informational".to_string(),
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

        let xml = render_junit(&report);
        assert!(
            xml.contains("<testsuite name=\"depguard\" tests=\"2\" failures=\"1\" skipped=\"1\">")
        );
        assert!(xml.contains(
            "<testcase classname=\"deps.no_wildcards\" name=\"wildcard_version [Cargo.toml:8]\">"
        ));
        assert!(xml.contains("<failure type=\"error\""));
        assert!(xml.contains("<skipped/>"));
    }

    #[test]
    fn render_junit_escapes_xml_content() {
        let report = RenderableReport {
            verdict: RenderableVerdictStatus::Warn,
            findings: vec![RenderableFinding {
                severity: RenderableSeverity::Warning,
                check_id: Some("deps.<bad>&\"".to_string()),
                code: "code<'&>".to_string(),
                message: "bad <value> & \"quote\"".to_string(),
                location: None,
                help: None,
                url: None,
            }],
            data: RenderableData {
                findings_emitted: 1,
                findings_total: 1,
                truncated_reason: Some("too <many> & more".to_string()),
            },
        };

        let xml = render_junit(&report);
        assert!(xml.contains("classname=\"deps.&lt;bad&gt;&amp;&quot;\""));
        assert!(xml.contains("name=\"code&lt;&apos;&amp;&gt;\""));
        assert!(xml.contains("message=\"bad &lt;value&gt; &amp; &quot;quote&quot;\""));
        assert!(xml.contains("depguard.truncated_reason"));
        assert!(xml.contains("too &lt;many&gt; &amp; more"));
    }
}
