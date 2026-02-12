//! The `explain` use case: look up check/code documentation.

use depguard_types::explain::{self, Explanation};

/// Output from the explain use case.
#[derive(Clone, Debug)]
pub enum ExplainOutput {
    /// Found an explanation for the identifier.
    Found(Explanation),
    /// Unknown identifier; includes available check_ids and codes.
    NotFound {
        identifier: String,
        available_check_ids: &'static [&'static str],
        available_codes: &'static [&'static str],
    },
}

/// Look up an explanation for a check_id or code.
pub fn run_explain(identifier: &str) -> ExplainOutput {
    match explain::lookup_explanation(identifier) {
        Some(exp) => ExplainOutput::Found(exp),
        None => ExplainOutput::NotFound {
            identifier: identifier.to_string(),
            available_check_ids: explain::all_check_ids(),
            available_codes: explain::all_codes(),
        },
    }
}

/// Format an explanation for terminal display.
pub fn format_explanation(exp: &Explanation) -> String {
    let mut out = String::new();

    out.push_str(exp.title);
    out.push('\n');
    out.push_str(&"=".repeat(exp.title.len()));
    out.push_str("\n\n");
    out.push_str(exp.description);
    out.push_str("\n\n");
    out.push_str("Remediation\n");
    out.push_str("-----------\n");
    out.push_str(exp.remediation);
    out.push_str("\n\n");
    out.push_str("Examples\n");
    out.push_str("--------\n\n");
    out.push_str("Before (violation):\n");
    out.push_str("```toml\n");
    out.push_str(exp.examples.before);
    out.push('\n');
    out.push_str("```\n\n");
    out.push_str("After (fixed):\n");
    out.push_str("```toml\n");
    out.push_str(exp.examples.after);
    out.push('\n');
    out.push_str("```\n");

    out
}

/// Format the "not found" error message for terminal display.
pub fn format_not_found(
    identifier: &str,
    check_ids: &[&'static str],
    codes: &[&'static str],
) -> String {
    let mut out = String::new();

    out.push_str(&format!("Unknown check_id or code: {}\n\n", identifier));
    out.push_str("Available check_ids:\n");
    for id in check_ids {
        out.push_str(&format!("  - {}\n", id));
    }
    out.push_str("\nAvailable codes:\n");
    for code in codes {
        out.push_str(&format!("  - {}\n", code));
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explain_known_check_id() {
        let output = run_explain("deps.no_wildcards");
        assert!(matches!(output, ExplainOutput::Found(_)));
    }

    #[test]
    fn explain_known_code() {
        let output = run_explain("wildcard_version");
        assert!(matches!(output, ExplainOutput::Found(_)));
    }

    #[test]
    fn explain_unknown() {
        let output = run_explain("not_a_real_thing");
        let (identifier, available_check_ids, available_codes) = unwrap_not_found(output);
        assert_eq!(identifier, "not_a_real_thing");
        assert!(!available_check_ids.is_empty());
        assert!(!available_codes.is_empty());
    }

    #[test]
    fn format_explanation_output() {
        let output = run_explain("deps.no_wildcards");
        let exp = unwrap_found(output);
        let formatted = format_explanation(&exp);
        assert!(formatted.contains("Remediation"));
        assert!(formatted.contains("Examples"));
        assert!(formatted.contains("```toml"));
    }

    #[test]
    fn format_not_found_output() {
        let formatted = format_not_found("missing", &["check.one", "check.two"], &["code.one"]);
        assert!(formatted.contains("Unknown check_id or code: missing"));
        assert!(formatted.contains("Available check_ids:"));
        assert!(formatted.contains("check.one"));
        assert!(formatted.contains("check.two"));
        assert!(formatted.contains("Available codes:"));
        assert!(formatted.contains("code.one"));
    }

    fn unwrap_found(output: ExplainOutput) -> Explanation {
        match output {
            ExplainOutput::Found(exp) => exp,
            _ => panic!("expected Found"),
        }
    }

    fn unwrap_not_found(
        output: ExplainOutput,
    ) -> (String, &'static [&'static str], &'static [&'static str]) {
        match output {
            ExplainOutput::NotFound {
                identifier,
                available_check_ids,
                available_codes,
            } => (identifier, available_check_ids, available_codes),
            _ => panic!("expected NotFound"),
        }
    }

    #[test]
    #[should_panic(expected = "expected Found")]
    fn unwrap_found_panics_for_not_found() {
        let output = run_explain("not_a_real_thing");
        let _ = unwrap_found(output);
    }

    #[test]
    #[should_panic(expected = "expected NotFound")]
    fn unwrap_not_found_panics_for_found() {
        let output = run_explain("deps.no_wildcards");
        let _ = unwrap_not_found(output);
    }
}
