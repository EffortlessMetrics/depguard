//! Render use cases: markdown and GitHub annotations from existing reports.

use anyhow::Context;
use camino::Utf8Path;
use depguard_types::DepguardReport;

/// Input for the markdown render use case.
#[derive(Clone, Debug)]
pub struct MarkdownInput<'a> {
    /// Path to the JSON report file.
    pub report_path: &'a Utf8Path,
}

/// Input for the annotations render use case.
#[derive(Clone, Debug)]
pub struct AnnotationsInput<'a> {
    /// Path to the JSON report file.
    pub report_path: &'a Utf8Path,
    /// Maximum number of annotations to emit.
    pub max: usize,
}

/// Read a report from a JSON file.
fn read_report(path: &Utf8Path) -> anyhow::Result<DepguardReport> {
    let data = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read report file: {}", path))?;
    let report: DepguardReport = serde_json::from_str(&data)
        .with_context(|| format!("failed to parse report JSON: {}", path))?;
    Ok(report)
}

/// Render markdown from a report file.
pub fn run_markdown(input: MarkdownInput<'_>) -> anyhow::Result<String> {
    let report = read_report(input.report_path)?;
    Ok(depguard_render::render_markdown(&report))
}

/// Render GitHub annotations from a report file.
pub fn run_annotations(input: AnnotationsInput<'_>) -> anyhow::Result<Vec<String>> {
    let report = read_report(input.report_path)?;
    let annotations = depguard_render::render_github_annotations(&report);
    Ok(annotations.into_iter().take(input.max).collect())
}

/// Serialize a report to JSON.
pub fn serialize_report(report: &DepguardReport) -> anyhow::Result<Vec<u8>> {
    serde_json::to_vec_pretty(report).context("serialize report to JSON")
}

/// Write a report to a JSON file (creates parent directories as needed).
pub fn write_report(path: &Utf8Path, report: &DepguardReport) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("create directory: {}", parent))?;
    }
    let data = serialize_report(report)?;
    std::fs::write(path, data).with_context(|| format!("write report: {}", path))?;
    Ok(())
}

/// Write text to a file (creates parent directories as needed).
pub fn write_text(path: &Utf8Path, text: &str) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("create directory: {}", parent))?;
    }
    std::fs::write(path, text).with_context(|| format!("write text: {}", path))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use depguard_types::{ReportEnvelope, ToolMeta, Verdict};
    use time::OffsetDateTime;

    fn sample_report() -> DepguardReport {
        ReportEnvelope {
            schema: "receipt.envelope.v1".to_string(),
            tool: ToolMeta {
                name: "depguard".to_string(),
                version: "0.1.0".to_string(),
            },
            started_at: OffsetDateTime::UNIX_EPOCH,
            finished_at: OffsetDateTime::UNIX_EPOCH,
            verdict: Verdict::Pass,
            findings: vec![],
            data: depguard_types::DepguardData::default(),
        }
    }

    #[test]
    fn serialize_report_produces_json() {
        let report = sample_report();
        let bytes = serialize_report(&report).expect("serialize");
        let text = String::from_utf8(bytes).expect("utf8");
        assert!(text.contains("receipt.envelope.v1"));
        assert!(text.contains("depguard"));
    }

    #[test]
    fn write_and_read_report() {
        let tmp = tempfile::tempdir().expect("create temp dir");
        let root = camino::Utf8Path::from_path(tmp.path()).expect("utf8 path");
        let report_path = root.join("subdir/report.json");

        let report = sample_report();
        write_report(&report_path, &report).expect("write report");

        let loaded = read_report(&report_path).expect("read report");
        assert_eq!(loaded.schema, report.schema);
        assert_eq!(loaded.verdict, report.verdict);
    }
}
