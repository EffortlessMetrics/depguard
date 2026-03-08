//! Inline suppression parsing utilities for depguard.

#![forbid(unsafe_code)]

/// Parse inline suppression tokens for a dependency declaration line.
///
/// Supported forms:
/// - `serde = "*" # depguard: allow(deps.no_wildcards)`
/// - `serde = "*" # depguard: allow(no_wildcards, wildcard_version)`
/// - `# depguard: allow(deps.no_wildcards)` directly above a dependency line
pub fn parse_inline_suppressions(source: &str, line: u32) -> Vec<String> {
    let mut out = Vec::new();

    if line == 0 {
        return out;
    }

    // Current line inline comment: dep = "..." # depguard: allow(...)
    if let Some(line_text) = line_text(source, line)
        && let Some((_, comment)) = line_text.split_once('#')
    {
        out.extend(parse_suppressions_from_comment(comment));
    }

    // Contiguous comment lines directly above the declaration.
    let mut current = line.saturating_sub(1);
    while current > 0 {
        let Some(line_text) = line_text(source, current) else {
            break;
        };
        let trimmed = line_text.trim();
        if let Some(comment) = trimmed.strip_prefix('#') {
            out.extend(parse_suppressions_from_comment(comment));
            current = current.saturating_sub(1);
            continue;
        }
        break;
    }

    out.sort();
    out.dedup();
    out
}

fn line_text(source: &str, line: u32) -> Option<&str> {
    source.lines().nth(line.saturating_sub(1) as usize)
}

fn parse_suppressions_from_comment(comment: &str) -> Vec<String> {
    let trimmed = comment.trim();
    let Some(rest) = trimmed.strip_prefix("depguard:") else {
        return Vec::new();
    };
    let rest = rest.trim_start();
    let Some(allow_rest) = rest.strip_prefix("allow(") else {
        return Vec::new();
    };
    let Some(end_idx) = allow_rest.find(')') else {
        return Vec::new();
    };
    let inner = &allow_rest[..end_idx];

    inner
        .split(',')
        .filter_map(normalize_suppression_token)
        .collect()
}

fn normalize_suppression_token(token: &str) -> Option<String> {
    let token = token.trim().trim_matches('"').trim_matches('\'');
    if token.is_empty() {
        return None;
    }

    if token.contains('.') {
        return Some(token.to_string());
    }

    let dep_check_id = format!("deps.{token}");
    if depguard_types::explain::lookup_explanation(&dep_check_id).is_some() {
        return Some(dep_check_id);
    }

    Some(token.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn line_for(source: &str, needle: &str) -> u32 {
        let pos = source
            .lines()
            .position(|line| line.contains(needle))
            .expect("needle should exist");
        (pos + 1) as u32
    }

    #[test]
    fn parse_inline_suppression_from_dependency_comment() {
        let manifest = r#"
[package]
name = "pkg"
version = "0.1.0"

[dependencies]
serde = "*" # depguard: allow(no_wildcards, wildcard_version)
"#;

        let line = line_for(manifest, "serde =");
        let suppressions = parse_inline_suppressions(manifest, line);

        assert_eq!(
            suppressions,
            vec![
                "deps.no_wildcards".to_string(),
                "wildcard_version".to_string()
            ]
        );
    }

    #[test]
    fn parse_inline_suppression_from_preceding_comment_line() {
        let manifest = r#"
[package]
name = "pkg"
version = "0.1.0"

[dependencies]
# depguard: allow(deps.no_wildcards)
serde = "*"
"#;

        let line = line_for(manifest, "serde =");
        let suppressions = parse_inline_suppressions(manifest, line);

        assert_eq!(suppressions, vec!["deps.no_wildcards".to_string()]);
    }

    #[test]
    fn parse_inline_suppression_ignores_unrelated_comments() {
        let manifest = r#"
[package]
name = "pkg"
version = "0.1.0"

[dependencies]
serde = "*" # this is not a depguard directive
"#;

        let line = line_for(manifest, "serde =");
        let suppressions = parse_inline_suppressions(manifest, line);

        assert!(suppressions.is_empty());
    }

    #[test]
    fn parse_inline_suppression_handles_zero_line() {
        let manifest = "serde = \"*\" # depguard: allow(no_wildcards)";
        let suppressions = parse_inline_suppressions(manifest, 0);

        assert!(suppressions.is_empty());
    }

    proptest! {
        #[test]
        fn parse_inline_suppression_sorts_and_dedupes(tokens in prop::collection::vec("[a-z]+\\.[a-z_]+", 0..20)) {
            let mut expected: Vec<String> = tokens.into_iter().collect();
            expected.sort();
            expected.dedup();

            let comment = expected
                .iter()
                .cloned()
                .chain(expected.iter().cloned())
                .collect::<Vec<String>>()
                .join(", ");
            let manifest = format!(
                r#"
[dependencies]
serde = "*" # depguard: allow({comment})
"#
            );

            let line = line_for(&manifest, "serde =");
            let suppressions = parse_inline_suppressions(&manifest, line);

            prop_assert_eq!(suppressions, expected);
        }
    }
}
