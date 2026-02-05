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
