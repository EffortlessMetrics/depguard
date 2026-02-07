//! Shared test utilities for the depguard workspace.
//!
//! This crate exists because `xtask` needs `normalize_nondeterministic` at
//! runtime (not behind `#[cfg(test)]`), so a `#[cfg(test)]` module inside
//! `depguard-types` would not suffice.

use serde_json::Value;

/// Normalize non-deterministic JSON fields for golden-file comparison.
///
/// Two concerns are handled separately:
///
/// 1. **Root-only** — `tool.version` is replaced with `"__VERSION__"` only
///    when the *root* object looks like a report envelope (has all five keys:
///    `schema`, `tool`, `run`, `verdict`, `findings`).  This prevents
///    false normalization of nested objects that happen to share the same
///    shape (e.g. a finding `data` payload containing envelope-like keys).
///
/// 2. **Recursive** — timestamp keys (`started_at`, `finished_at`,
///    `ended_at`) and `duration_ms` are normalized at any depth because
///    their placeholder values are fixed and cannot collide with real data.
pub fn normalize_nondeterministic(mut value: Value) -> Value {
    // Root-only: normalize tool.version if this is an envelope
    if let Some(obj) = value.as_object_mut() {
        let is_envelope = obj.contains_key("schema")
            && obj.contains_key("tool")
            && obj.contains_key("run")
            && obj.contains_key("verdict")
            && obj.contains_key("findings");
        if is_envelope
            && let Some(tool) = obj.get_mut("tool")
            && let Some(tool_obj) = tool.as_object_mut()
            && tool_obj.contains_key("name")
            && tool_obj.contains_key("version")
        {
            tool_obj.insert(
                "version".to_string(),
                Value::String("__VERSION__".to_string()),
            );
        }
    }
    // Recursive: timestamps and duration at any depth
    normalize_timestamps_recursive(&mut value);
    value
}

fn normalize_timestamps_recursive(value: &mut Value) {
    match value {
        Value::Object(map) => {
            if map.contains_key("started_at") {
                map.insert(
                    "started_at".to_string(),
                    Value::String("__TIMESTAMP__".to_string()),
                );
            }
            if map.contains_key("finished_at") {
                map.insert(
                    "finished_at".to_string(),
                    Value::String("__TIMESTAMP__".to_string()),
                );
            }
            if map.contains_key("ended_at") {
                map.insert(
                    "ended_at".to_string(),
                    Value::String("__TIMESTAMP__".to_string()),
                );
            }
            if map.contains_key("duration_ms") {
                map.insert("duration_ms".to_string(), Value::Number(0.into()));
            }
            for val in map.values_mut() {
                normalize_timestamps_recursive(val);
            }
        }
        Value::Array(arr) => {
            for val in arr.iter_mut() {
                normalize_timestamps_recursive(val);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn normalize_only_touches_envelope_tool_version() {
        let input = json!({
            "schema": "urn:effortless:sensor.report.v1",
            "tool": { "name": "depguard", "version": "0.1.0" },
            "run": { "started_at": "2025-01-01T00:00:00Z", "ended_at": "2025-01-01T00:00:01Z" },
            "verdict": { "pass": true },
            "findings": [
                {
                    "data": { "name": "serde", "version": "1.0.200" }
                },
                {
                    "data": { "tool": { "name": "cargo", "version": "1.80" } }
                }
            ]
        });

        let result = normalize_nondeterministic(input);

        // Envelope tool.version should be normalized
        assert_eq!(result["tool"]["version"], "__VERSION__");
        assert_eq!(result["tool"]["name"], "depguard");

        // Finding data with name+version (not a tool key) must be untouched
        assert_eq!(result["findings"][0]["data"]["name"], "serde");
        assert_eq!(result["findings"][0]["data"]["version"], "1.0.200");

        // Finding data with nested tool object must be untouched (not an envelope)
        assert_eq!(result["findings"][1]["data"]["tool"]["name"], "cargo");
        assert_eq!(result["findings"][1]["data"]["tool"]["version"], "1.80");
    }

    #[test]
    fn nested_receipt_like_object_not_normalized() {
        let input = json!({
            "schema": "depguard.report.v2",
            "tool": { "name": "depguard", "version": "0.1.0" },
            "run": { "started_at": "2025-06-01T00:00:00Z", "ended_at": "2025-06-01T00:00:01Z" },
            "verdict": { "status": "fail" },
            "findings": [
                {
                    "data": {
                        "schema": "fake",
                        "tool": { "name": "inner", "version": "9.9.9" },
                        "run": { "started_at": "2025-06-01T12:00:00Z" },
                        "verdict": { "status": "pass" },
                        "findings": []
                    }
                }
            ]
        });

        let result = normalize_nondeterministic(input);

        // Root tool.version IS normalized
        assert_eq!(result["tool"]["version"], "__VERSION__");

        // Nested object tool.version is NOT normalized (not at root)
        assert_eq!(
            result["findings"][0]["data"]["tool"]["version"], "9.9.9",
            "nested tool.version should NOT be normalized"
        );

        // But nested timestamps ARE normalized (recursive)
        assert_eq!(
            result["findings"][0]["data"]["run"]["started_at"], "__TIMESTAMP__",
            "nested started_at should be normalized"
        );

        // Root timestamps are also normalized
        assert_eq!(result["run"]["started_at"], "__TIMESTAMP__");
        assert_eq!(result["run"]["ended_at"], "__TIMESTAMP__");
    }

    #[test]
    fn root_without_envelope_keys_not_normalized() {
        let input = json!({
            "tool": { "name": "other", "version": "2.0.0" },
            "run": { "started_at": "2025-01-01T00:00:00Z" }
        });

        let result = normalize_nondeterministic(input);

        // tool.version should NOT be normalized (missing schema/verdict/findings)
        assert_eq!(result["tool"]["version"], "2.0.0");

        // But timestamps are still normalized (recursive)
        assert_eq!(result["run"]["started_at"], "__TIMESTAMP__");
    }
}
