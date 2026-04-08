//! Structured validation errors with config key path tracking.
//!
//! This module provides a rich error type for configuration validation that
//! clearly indicates which config key caused an error and provides helpful
//! context for fixing the issue.

use std::fmt;
use std::path::PathBuf;

/// A structured validation error that tracks the config key path.
#[derive(Clone, Debug, PartialEq)]
pub struct ValidationError {
    /// The config key path (e.g., `checks.deps.no_wildcards.severity`)
    key_path: String,
    /// The validation error message
    message: String,
    /// Optional file path where the config was read from
    file_path: Option<PathBuf>,
    /// Optional line number in the config file
    line: Option<usize>,
    /// Optional suggested fix
    suggestion: Option<String>,
}

impl ValidationError {
    /// Create a new validation error for a config key.
    pub fn new(key_path: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            key_path: key_path.into(),
            message: message.into(),
            file_path: None,
            line: None,
            suggestion: None,
        }
    }

    /// Add a file path to the error.
    pub fn with_file(mut self, path: impl Into<PathBuf>) -> Self {
        self.file_path = Some(path.into());
        self
    }

    /// Add a line number to the error.
    pub fn with_line(mut self, line: usize) -> Self {
        self.line = Some(line);
        self
    }

    /// Add a suggested fix to the error.
    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }

    /// Get the config key path.
    pub fn key_path(&self) -> &str {
        &self.key_path
    }

    /// Get the error message.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Get the file path, if available.
    pub fn file_path(&self) -> Option<&std::path::Path> {
        self.file_path.as_deref()
    }

    /// Get the line number, if available.
    pub fn line(&self) -> Option<usize> {
        self.line
    }

    /// Get the suggested fix, if available.
    pub fn suggestion(&self) -> Option<&str> {
        self.suggestion.as_deref()
    }

    /// Create a validation error for an unknown scope value.
    pub fn unknown_scope(value: &str) -> Self {
        Self::new("scope", format!("unknown scope: '{value}'"))
            .with_suggestion("expected 'repo' or 'diff'")
    }

    /// Create a validation error for an unknown severity value.
    pub fn unknown_severity(check_id: &str, value: &str) -> Self {
        Self::new(
            format!("checks.{check_id}.severity"),
            format!("unknown severity: '{value}'"),
        )
        .with_suggestion("expected 'info', 'warning', or 'error'")
    }

    /// Create a validation error for an unknown fail_on value.
    pub fn unknown_fail_on(value: &str) -> Self {
        Self::new("fail_on", format!("unknown fail_on: '{value}'"))
            .with_suggestion("expected 'error' or 'warning'")
    }

    /// Create a validation error for an unknown profile value.
    pub fn unknown_profile(value: &str) -> Self {
        Self::new("profile", format!("unknown profile: '{value}'"))
            .with_suggestion("expected 'strict', 'warn', or 'compat'")
    }

    /// Create a validation error for an invalid glob pattern in an allowlist.
    pub fn invalid_allow_glob(check_id: &str, pattern: &str, error: &str) -> Self {
        Self::new(
            format!("checks.{check_id}.allow"),
            format!("invalid glob pattern '{pattern}': {error}"),
        )
    }

    /// Create a validation error for an unknown check ID.
    pub fn unknown_check_id(check_id: &str) -> Self {
        Self::new(
            format!("checks.{check_id}"),
            format!("unknown check ID: '{check_id}'"),
        )
        .with_suggestion("run 'depguard explain' to see available checks")
    }

    /// Create a validation error for an invalid max_findings value.
    pub fn invalid_max_findings(value: u32) -> Self {
        Self::new(
            "max_findings",
            format!("invalid max_findings: {value} must be at least 1"),
        )
        .with_suggestion("set max_findings to a positive integer, or remove to use default (200)")
    }

    /// Create a validation error for ignore_publish_false on an unsupported check.
    pub fn ignore_publish_false_not_supported(check_id: &str) -> Self {
        Self::new(
            format!("checks.{check_id}.ignore_publish_false"),
            format!("ignore_publish_false is not supported for check '{check_id}'"),
        )
        .with_suggestion("this option is only valid for 'deps.path_requires_version' check")
    }

    /// Create a validation error for an invalid boolean value.
    pub fn invalid_boolean(key_path: &str, value: &str) -> Self {
        Self::new(key_path, format!("invalid boolean value: '{value}'"))
            .with_suggestion("expected 'true' or 'false'")
    }

    /// Create a validation error for an invalid integer value.
    pub fn invalid_integer(key_path: &str, value: &str) -> Self {
        Self::new(key_path, format!("invalid integer value: '{value}'"))
            .with_suggestion("expected a valid integer")
    }

    /// Create a validation error for a missing required field.
    pub fn missing_required_field(key_path: &str) -> Self {
        Self::new(key_path, format!("required field '{key_path}' is missing"))
    }

    /// Create a validation error for an invalid enum value with custom expected values.
    pub fn invalid_enum_value(key_path: &str, value: &str, expected: &[&str]) -> Self {
        let expected_str = expected.join("', '");
        Self::new(key_path, format!("invalid value '{value}'"))
            .with_suggestion(format!("expected one of: '{expected_str}'"))
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Format: [file:line: ]key: message
        if let Some(ref path) = self.file_path {
            if let Some(line) = self.line {
                write!(f, "{}:{}: ", path.display(), line)?;
            } else {
                write!(f, "{}: ", path.display())?;
            }
        }

        write!(f, "{}: {}", self.key_path, self.message)?;

        if let Some(ref suggestion) = self.suggestion {
            write!(f, "\n  hint: {suggestion}")?;
        }

        Ok(())
    }
}

impl std::error::Error for ValidationError {}

/// A collection of validation errors.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ValidationErrors {
    errors: Vec<ValidationError>,
}

impl ValidationErrors {
    /// Create an empty collection.
    pub fn new() -> Self {
        Self { errors: Vec::new() }
    }

    /// Add a validation error.
    pub fn push(&mut self, error: ValidationError) {
        self.errors.push(error);
    }

    /// Check if there are any errors.
    pub fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }

    /// Get the number of errors.
    pub fn len(&self) -> usize {
        self.errors.len()
    }

    /// Get an iterator over the errors.
    pub fn iter(&self) -> impl Iterator<Item = &ValidationError> {
        self.errors.iter()
    }

    /// Convert into a vector of errors.
    pub fn into_inner(self) -> Vec<ValidationError> {
        self.errors
    }

    /// Merge another collection of errors into this one.
    pub fn extend(&mut self, other: ValidationErrors) {
        self.errors.extend(other.errors);
    }
}

impl fmt::Display for ValidationErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, error) in self.errors.iter().enumerate() {
            if i > 0 {
                writeln!(f)?;
            }
            write!(f, "{error}")?;
        }
        Ok(())
    }
}

impl std::error::Error for ValidationErrors {}

impl From<Vec<ValidationError>> for ValidationErrors {
    fn from(errors: Vec<ValidationError>) -> Self {
        Self { errors }
    }
}

impl From<ValidationError> for ValidationErrors {
    fn from(error: ValidationError) -> Self {
        Self {
            errors: vec![error],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validation_error_display_basic() {
        let err = ValidationError::new("scope", "unknown scope: 'invalid'");
        assert_eq!(err.to_string(), "scope: unknown scope: 'invalid'");
    }

    #[test]
    fn validation_error_display_with_suggestion() {
        let err = ValidationError::new("scope", "unknown scope: 'invalid'")
            .with_suggestion("expected 'repo' or 'diff'");
        assert_eq!(
            err.to_string(),
            "scope: unknown scope: 'invalid'\n  hint: expected 'repo' or 'diff'"
        );
    }

    #[test]
    fn validation_error_display_with_file() {
        let err = ValidationError::new("scope", "unknown scope: 'invalid'")
            .with_file(PathBuf::from("depguard.toml"));
        assert_eq!(
            err.to_string(),
            "depguard.toml: scope: unknown scope: 'invalid'"
        );
    }

    #[test]
    fn validation_error_display_with_file_and_line() {
        let err = ValidationError::new("scope", "unknown scope: 'invalid'")
            .with_file(PathBuf::from("depguard.toml"))
            .with_line(5);
        assert_eq!(
            err.to_string(),
            "depguard.toml:5: scope: unknown scope: 'invalid'"
        );
    }

    #[test]
    fn validation_error_display_full() {
        let err = ValidationError::new(
            "checks.deps.no_wildcards.severity",
            "unknown severity: 'fatal'",
        )
        .with_file(PathBuf::from("depguard.toml"))
        .with_line(10)
        .with_suggestion("expected 'info', 'warning', or 'error'");
        assert_eq!(
            err.to_string(),
            "depguard.toml:10: checks.deps.no_wildcards.severity: unknown severity: 'fatal'\n  hint: expected 'info', 'warning', or 'error'"
        );
    }

    #[test]
    fn unknown_scope_factory() {
        let err = ValidationError::unknown_scope("invalid");
        assert_eq!(err.key_path(), "scope");
        assert!(err.message().contains("invalid"));
        assert_eq!(err.suggestion(), Some("expected 'repo' or 'diff'"));
    }

    #[test]
    fn unknown_severity_factory() {
        let err = ValidationError::unknown_severity("deps.no_wildcards", "fatal");
        assert_eq!(err.key_path(), "checks.deps.no_wildcards.severity");
        assert!(err.message().contains("fatal"));
        assert_eq!(
            err.suggestion(),
            Some("expected 'info', 'warning', or 'error'")
        );
    }

    #[test]
    fn unknown_fail_on_factory() {
        let err = ValidationError::unknown_fail_on("never");
        assert_eq!(err.key_path(), "fail_on");
        assert!(err.message().contains("never"));
        assert_eq!(err.suggestion(), Some("expected 'error' or 'warning'"));
    }

    #[test]
    fn invalid_allow_glob_factory() {
        let err = ValidationError::invalid_allow_glob("deps.no_wildcards", "[", "unclosed bracket");
        assert_eq!(err.key_path(), "checks.deps.no_wildcards.allow");
        assert!(err.message().contains("["));
        assert!(err.message().contains("unclosed bracket"));
    }

    #[test]
    fn validation_errors_collection() {
        let mut errors = ValidationErrors::new();
        assert!(errors.is_empty());
        assert_eq!(errors.len(), 0);

        errors.push(ValidationError::unknown_scope("invalid"));
        errors.push(ValidationError::unknown_fail_on("never"));

        assert!(!errors.is_empty());
        assert_eq!(errors.len(), 2);

        let error_strings: Vec<_> = errors.iter().map(|e| e.to_string()).collect();
        assert_eq!(error_strings.len(), 2);
    }

    #[test]
    fn validation_errors_display() {
        let mut errors = ValidationErrors::new();
        errors.push(ValidationError::unknown_scope("invalid"));
        errors.push(ValidationError::unknown_fail_on("never"));

        let display = errors.to_string();
        assert!(display.contains("scope:"));
        assert!(display.contains("fail_on:"));
    }

    #[test]
    fn validation_errors_from_vec() {
        let errors = ValidationErrors::from(vec![
            ValidationError::unknown_scope("invalid"),
            ValidationError::unknown_fail_on("never"),
        ]);
        assert_eq!(errors.len(), 2);
    }

    #[test]
    fn validation_errors_extend() {
        let mut errors1 = ValidationErrors::new();
        errors1.push(ValidationError::unknown_scope("invalid"));

        let mut errors2 = ValidationErrors::new();
        errors2.push(ValidationError::unknown_fail_on("never"));

        errors1.extend(errors2);
        assert_eq!(errors1.len(), 2);
    }

    #[test]
    fn invalid_max_findings_factory() {
        let err = ValidationError::invalid_max_findings(0);
        assert_eq!(err.key_path(), "max_findings");
        assert!(err.message().contains("0"));
        assert!(err.message().contains("at least 1"));
        assert!(err.suggestion().is_some());
    }

    #[test]
    fn ignore_publish_false_not_supported_factory() {
        let err = ValidationError::ignore_publish_false_not_supported("deps.no_wildcards");
        assert_eq!(
            err.key_path(),
            "checks.deps.no_wildcards.ignore_publish_false"
        );
        assert!(err.message().contains("not supported"));
        assert!(err.message().contains("deps.no_wildcards"));
        assert!(err.suggestion().is_some());
    }

    #[test]
    fn invalid_boolean_factory() {
        let err = ValidationError::invalid_boolean("checks.some_check.enabled", "yes");
        assert_eq!(err.key_path(), "checks.some_check.enabled");
        assert!(err.message().contains("yes"));
        assert!(err.message().contains("boolean"));
        assert_eq!(err.suggestion(), Some("expected 'true' or 'false'"));
    }

    #[test]
    fn invalid_integer_factory() {
        let err = ValidationError::invalid_integer("max_findings", "abc");
        assert_eq!(err.key_path(), "max_findings");
        assert!(err.message().contains("abc"));
        assert!(err.message().contains("integer"));
        assert_eq!(err.suggestion(), Some("expected a valid integer"));
    }

    #[test]
    fn missing_required_field_factory() {
        let err = ValidationError::missing_required_field("profile");
        assert_eq!(err.key_path(), "profile");
        assert!(err.message().contains("required"));
        assert!(err.message().contains("profile"));
    }

    #[test]
    fn invalid_enum_value_factory() {
        let err = ValidationError::invalid_enum_value("scope", "invalid", &["repo", "diff"]);
        assert_eq!(err.key_path(), "scope");
        assert!(err.message().contains("invalid"));
        assert_eq!(err.suggestion(), Some("expected one of: 'repo', 'diff'"));
    }
}
