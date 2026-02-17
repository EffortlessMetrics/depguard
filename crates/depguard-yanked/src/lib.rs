//! Offline yanked-version index parsing and querying.
//!
//! This crate is intentionally IO-free: callers provide index contents as text.

#![forbid(unsafe_code)]

use anyhow::Context;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

/// In-memory yanked-version index.
///
/// Keys are dependency names and values are exact yanked version strings.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct YankedIndex {
    entries: BTreeMap<String, BTreeSet<String>>,
}

impl YankedIndex {
    /// Returns true when `crate_name` has the exact `version` marked as yanked.
    pub fn is_yanked(&self, crate_name: &str, version: &str) -> bool {
        self.entries
            .get(crate_name)
            .map(|versions| versions.contains(version))
            .unwrap_or(false)
    }

    /// Number of crates represented in the index.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns true if the index has no entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Adds a yanked version entry.
    pub fn insert(&mut self, crate_name: &str, version: &str) {
        let crate_name = crate_name.trim();
        let version = version.trim();
        if crate_name.is_empty() || version.is_empty() {
            return;
        }
        self.entries
            .entry(crate_name.to_string())
            .or_default()
            .insert(version.to_string());
    }
}

/// Parse a yanked index from text.
///
/// Supported formats:
/// - JSON object map: `{ "serde": ["1.0.0", "1.0.1"] }`
/// - JSON array of objects: `[{"crate":"serde","version":"1.0.0"}]`
/// - Plain text lines:
///   - `serde 1.0.0`
///   - `serde@1.0.0`
///   - Comments with `# ...` are ignored
pub fn parse_yanked_index(input: &str) -> anyhow::Result<YankedIndex> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Ok(YankedIndex::default());
    }

    let first = trimmed.as_bytes()[0];
    if first == b'{' || first == b'[' {
        return parse_json_index(trimmed);
    }

    parse_line_index(input)
}

fn parse_json_index(input: &str) -> anyhow::Result<YankedIndex> {
    let value: Value = serde_json::from_str(input).context("parse yanked index JSON")?;
    let mut index = YankedIndex::default();

    match value {
        Value::Object(map) => {
            for (crate_name, versions) in map {
                match versions {
                    Value::String(version) => index.insert(&crate_name, &version),
                    Value::Array(items) => {
                        for item in items {
                            let Some(version) = item.as_str() else {
                                anyhow::bail!(
                                    "invalid yanked index JSON: expected string version in array for crate '{}'",
                                    crate_name
                                );
                            };
                            index.insert(&crate_name, version);
                        }
                    }
                    other => {
                        anyhow::bail!(
                            "invalid yanked index JSON: crate '{}' value must be string or array, got {}",
                            crate_name,
                            json_type(&other)
                        );
                    }
                }
            }
        }
        Value::Array(items) => {
            for (idx, item) in items.into_iter().enumerate() {
                let Value::Object(obj) = item else {
                    anyhow::bail!(
                        "invalid yanked index JSON: array entry {} must be an object",
                        idx
                    );
                };

                let crate_name = obj
                    .get("crate")
                    .and_then(Value::as_str)
                    .or_else(|| obj.get("name").and_then(Value::as_str))
                    .or_else(|| obj.get("dependency").and_then(Value::as_str))
                    .context("invalid yanked index JSON: array entry missing crate/name")?;

                let version = obj
                    .get("version")
                    .and_then(Value::as_str)
                    .context("invalid yanked index JSON: array entry missing version")?;

                index.insert(crate_name, version);
            }
        }
        other => {
            anyhow::bail!(
                "invalid yanked index JSON: expected object or array, got {}",
                json_type(&other)
            );
        }
    }

    Ok(index)
}

fn parse_line_index(input: &str) -> anyhow::Result<YankedIndex> {
    let mut index = YankedIndex::default();

    for (line_no, raw_line) in input.lines().enumerate() {
        let line_no = line_no + 1;
        let line = strip_comment(raw_line).trim();
        if line.is_empty() {
            continue;
        }

        if let Some((crate_name, version)) = parse_line_pair(line) {
            index.insert(crate_name, version);
            continue;
        }

        anyhow::bail!(
            "invalid yanked index line {}: expected '<crate> <version>' or '<crate>@<version>'",
            line_no
        );
    }

    Ok(index)
}

fn strip_comment(line: &str) -> &str {
    line.split_once('#').map(|(head, _)| head).unwrap_or(line)
}

fn parse_line_pair(line: &str) -> Option<(&str, &str)> {
    if let Some((crate_name, version)) = line.split_once('@') {
        let crate_name = crate_name.trim();
        let version = version.trim();
        if !crate_name.is_empty() && !version.is_empty() {
            return Some((crate_name, version));
        }
    }

    let mut parts = line.split_whitespace();
    let crate_name = parts.next()?;
    let version = parts.next()?;
    if parts.next().is_some() {
        return None;
    }
    Some((crate_name, version))
}

fn json_type(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_json_map_index() {
        let input = r#"{
  "serde": ["1.0.188", "1.0.189"],
  "tokio": "1.37.0"
}"#;

        let index = parse_yanked_index(input).expect("parse yanked index");
        assert!(index.is_yanked("serde", "1.0.188"));
        assert!(index.is_yanked("serde", "1.0.189"));
        assert!(index.is_yanked("tokio", "1.37.0"));
        assert!(!index.is_yanked("tokio", "1.38.0"));
    }

    #[test]
    fn parse_json_array_index() {
        let input = r#"[
  {"crate":"serde", "version":"1.0.190"},
  {"name":"tokio", "version":"1.36.0"},
  {"dependency":"time", "version":"0.3.20"}
]"#;

        let index = parse_yanked_index(input).expect("parse yanked index");
        assert!(index.is_yanked("serde", "1.0.190"));
        assert!(index.is_yanked("tokio", "1.36.0"));
        assert!(index.is_yanked("time", "0.3.20"));
    }

    #[test]
    fn parse_line_index_with_comments() {
        let input = r#"
# comment
serde 1.0.191
tokio@1.35.0
time 0.3.21 # inline comment
"#;

        let index = parse_yanked_index(input).expect("parse yanked index");
        assert!(index.is_yanked("serde", "1.0.191"));
        assert!(index.is_yanked("tokio", "1.35.0"));
        assert!(index.is_yanked("time", "0.3.21"));
    }

    #[test]
    fn parse_empty_index() {
        let index = parse_yanked_index("  \n\t").expect("parse yanked index");
        assert!(index.is_empty());
    }

    #[test]
    fn invalid_line_fails_with_line_number() {
        let err = parse_yanked_index("serde 1.0.1 extra").expect_err("expected parse failure");
        assert!(err.to_string().contains("line 1"));
    }
}
