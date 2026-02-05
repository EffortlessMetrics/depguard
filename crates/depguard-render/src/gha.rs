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
