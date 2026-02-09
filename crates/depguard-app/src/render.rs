//! Render use cases: markdown and GitHub annotations from in-memory reports.

use depguard_render::RenderableReport;

pub fn render_markdown(report: &RenderableReport) -> String {
    depguard_render::render_markdown(report)
}

pub fn render_annotations(report: &RenderableReport, max: usize) -> Vec<String> {
    depguard_render::render_github_annotations(report)
        .into_iter()
        .take(max)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use depguard_render::{
        RenderableData, RenderableFinding, RenderableLocation, RenderableReport,
        RenderableSeverity, RenderableVerdictStatus,
    };

    fn sample_report() -> RenderableReport {
        RenderableReport {
            verdict: RenderableVerdictStatus::Pass,
            findings: vec![
                RenderableFinding {
                    severity: RenderableSeverity::Warning,
                    check_id: Some("deps.no_wildcards".to_string()),
                    code: "wildcard_version".to_string(),
                    message: "bad".to_string(),
                    location: Some(RenderableLocation {
                        path: "Cargo.toml".to_string(),
                        line: Some(1),
                        col: Some(2),
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
        }
    }

    #[test]
    fn render_annotations_respects_max() {
        let report = sample_report();
        let annotations = render_annotations(&report, 1);
        assert_eq!(annotations.len(), 1);
    }

    #[test]
    fn render_markdown_smoke() {
        let report = sample_report();
        let markdown = render_markdown(&report);
        assert!(!markdown.is_empty());
    }
}
