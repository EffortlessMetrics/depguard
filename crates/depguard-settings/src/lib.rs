//! Config parsing and profile/preset resolution.
//!
//! This crate is intentionally IO-free: it parses and resolves configuration provided as strings.

#![forbid(unsafe_code)]

mod model;
mod presets;
mod resolve;
mod validation_error;

pub use model::{CheckConfig, DepguardConfigV1};
pub use resolve::{Overrides, ResolvedConfig};
pub use validation_error::{ValidationError, ValidationErrors};

/// Parse `depguard.toml` (or equivalent) into a typed model.
pub fn parse_config_toml(input: &str) -> anyhow::Result<DepguardConfigV1> {
    let cfg: DepguardConfigV1 = toml::from_str(input)?;
    Ok(cfg)
}

/// Resolve the effective config used by the engine (profiles + overrides + per-check config).
pub fn resolve_config(
    cfg: DepguardConfigV1,
    overrides: Overrides,
) -> anyhow::Result<ResolvedConfig> {
    resolve::resolve_config(cfg, overrides)
}

#[cfg(test)]
mod tests {
    use super::*;
    use depguard_domain_core::policy::{FailOn, Scope};
    use depguard_types::Severity;

    #[test]
    fn parse_empty_config() {
        let cfg = parse_config_toml("").unwrap();
        assert_eq!(cfg.profile, None);
        assert_eq!(cfg.scope, None);
        assert_eq!(cfg.baseline, None);
        assert!(cfg.checks.is_empty());
    }

    #[test]
    fn parse_minimal_config() {
        let toml = r#"
            profile = "warn"
            scope = "diff"
            baseline = ".depguard-baseline.json"
        "#;
        let cfg = parse_config_toml(toml).unwrap();
        assert_eq!(cfg.profile, Some("warn".to_string()));
        assert_eq!(cfg.scope, Some("diff".to_string()));
        assert_eq!(cfg.baseline, Some(".depguard-baseline.json".to_string()));
    }

    #[test]
    fn parse_config_with_checks() {
        let toml = r#"
            profile = "strict"

            [checks."deps.no_wildcards"]
            enabled = false

            [checks."deps.path_safety"]
            severity = "warning"
            allow = ["vendor/*"]
        "#;
        let cfg = parse_config_toml(toml).unwrap();

        let wildcard_cfg = cfg.checks.get("deps.no_wildcards").unwrap();
        assert_eq!(wildcard_cfg.enabled, Some(false));

        let path_cfg = cfg.checks.get("deps.path_safety").unwrap();
        assert_eq!(path_cfg.severity, Some("warning".to_string()));
        assert_eq!(path_cfg.allow, vec!["vendor/*"]);
    }

    #[test]
    fn resolve_default_profile() {
        let cfg = DepguardConfigV1::default();
        let resolved = resolve_config(cfg, Overrides::default()).unwrap();

        assert_eq!(resolved.effective.profile, "strict");
        assert_eq!(resolved.effective.fail_on, FailOn::Error);
        assert_eq!(resolved.effective.scope, Scope::Repo);
    }

    #[test]
    fn resolve_warn_profile() {
        let cfg = DepguardConfigV1 {
            profile: Some("warn".to_string()),
            ..Default::default()
        };
        let resolved = resolve_config(cfg, Overrides::default()).unwrap();

        assert_eq!(resolved.effective.profile, "warn");
        assert_eq!(resolved.effective.fail_on, FailOn::Warning);
    }

    #[test]
    fn resolve_compat_profile() {
        let cfg = DepguardConfigV1 {
            profile: Some("compat".to_string()),
            ..Default::default()
        };
        let resolved = resolve_config(cfg, Overrides::default()).unwrap();

        assert_eq!(resolved.effective.profile, "compat");
        assert_eq!(resolved.effective.fail_on, FailOn::Error);

        // compat uses warning severity by default
        let check = resolved.effective.checks.get("deps.no_wildcards").unwrap();
        assert_eq!(check.severity, Severity::Warning);
    }

    #[test]
    fn cli_overrides_take_precedence() {
        let cfg = DepguardConfigV1 {
            profile: Some("warn".to_string()),
            scope: Some("repo".to_string()),
            max_findings: Some(100),
            ..Default::default()
        };
        let overrides = Overrides {
            profile: Some("strict".to_string()),
            scope: Some("diff".to_string()),
            max_findings: Some(50),
            baseline: Some("custom-baseline.json".to_string()),
        };
        let resolved = resolve_config(cfg, overrides).unwrap();

        assert_eq!(resolved.effective.profile, "strict");
        assert_eq!(resolved.effective.scope, Scope::Diff);
        assert_eq!(resolved.effective.max_findings, 50);
        assert_eq!(
            resolved.baseline_path.as_deref(),
            Some("custom-baseline.json")
        );
    }

    #[test]
    fn per_check_overrides() {
        let toml = r#"
            [checks."deps.no_wildcards"]
            enabled = false

            [checks."deps.path_safety"]
            severity = "info"
            allow = ["special-path"]
        "#;
        let cfg = parse_config_toml(toml).unwrap();
        let resolved = resolve_config(cfg, Overrides::default()).unwrap();

        let wildcard = resolved.effective.checks.get("deps.no_wildcards").unwrap();
        assert!(!wildcard.enabled);

        let path_safety = resolved.effective.checks.get("deps.path_safety").unwrap();
        assert_eq!(path_safety.severity, Severity::Info);
        assert_eq!(path_safety.allow, vec!["special-path"]);
    }

    #[test]
    fn per_check_ignore_publish_false_override() {
        let toml = r#"
            [checks."deps.path_requires_version"]
            ignore_publish_false = true
        "#;
        let cfg = parse_config_toml(toml).unwrap();
        let resolved = resolve_config(cfg, Overrides::default()).unwrap();

        let check = resolved
            .effective
            .checks
            .get("deps.path_requires_version")
            .expect("check");
        assert!(check.ignore_publish_false);
    }

    #[test]
    fn invalid_scope_returns_error() {
        let cfg = DepguardConfigV1 {
            scope: Some("invalid".to_string()),
            ..Default::default()
        };
        let result = resolve_config(cfg, Overrides::default());
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        // Check for the key path and error type
        assert!(
            err_msg.contains("scope:"),
            "error message should contain key path: {err_msg}"
        );
        assert!(
            err_msg.contains("unknown scope"),
            "error message should contain 'unknown scope': {err_msg}"
        );
    }

    #[test]
    fn invalid_severity_returns_error() {
        let toml = r#"
            [checks."deps.no_wildcards"]
            severity = "fatal"
        "#;
        let cfg = parse_config_toml(toml).unwrap();
        let result = resolve_config(cfg, Overrides::default());
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        // Check for the key path and error type
        assert!(
            err_msg.contains("checks.deps.no_wildcards.severity"),
            "error message should contain key path: {err_msg}"
        );
        assert!(
            err_msg.contains("unknown severity"),
            "error message should contain 'unknown severity': {err_msg}"
        );
    }

    #[test]
    fn invalid_allowlist_glob_returns_error() {
        let toml = r#"
            [checks."deps.no_wildcards"]
            allow = ["["]
        "#;
        let cfg = parse_config_toml(toml).unwrap();
        let result = resolve_config(cfg, Overrides::default());
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        // Check for the key path and error type
        assert!(
            err_msg.contains("checks.deps.no_wildcards.allow"),
            "error message should contain key path: {err_msg}"
        );
        assert!(
            err_msg.contains("invalid glob pattern"),
            "error message should contain 'invalid glob pattern': {err_msg}"
        );
    }

    #[test]
    fn fail_on_config_overrides_profile() {
        let cfg = DepguardConfigV1 {
            profile: Some("strict".to_string()),
            fail_on: Some("warn".to_string()),
            ..Default::default()
        };
        let resolved = resolve_config(cfg, Overrides::default()).unwrap();
        // strict profile defaults to FailOn::Error, but config overrides to Warning
        assert_eq!(resolved.effective.fail_on, FailOn::Warning);
    }

    #[test]
    fn invalid_fail_on_returns_error() {
        let cfg = DepguardConfigV1 {
            fail_on: Some("never".to_string()),
            ..Default::default()
        };
        let result = resolve_config(cfg, Overrides::default());
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        // Check for the key path and error type
        assert!(
            err_msg.contains("fail_on:"),
            "error message should contain key path: {err_msg}"
        );
        assert!(
            err_msg.contains("unknown fail_on"),
            "error message should contain 'unknown fail_on': {err_msg}"
        );
    }

    #[test]
    fn additional_checks_have_stable_default_severities() {
        let cfg = DepguardConfigV1::default();
        let resolved = resolve_config(cfg, Overrides::default()).unwrap();
        let strict = depguard_check_catalog::checks_for_profile("strict");
        assert_eq!(resolved.effective.checks.len(), strict.len());

        for check in strict {
            let actual = resolved
                .effective
                .checks
                .get(check.id)
                .expect("catalog check should be present");
            let expected_enabled =
                check.enabled && depguard_check_catalog::is_check_available(check.id);
            assert_eq!(
                actual.enabled, expected_enabled,
                "check {} enabled default should match catalog",
                check.id
            );
            assert_eq!(
                actual.severity, check.severity,
                "check {} severity should match catalog",
                check.id
            );
        }
    }

    #[test]
    fn enabling_default_features_without_severity_uses_warning_default() {
        let toml = r#"
            [checks."deps.default_features_explicit"]
            enabled = true
        "#;
        let cfg = parse_config_toml(toml).unwrap();
        let resolved = resolve_config(cfg, Overrides::default()).unwrap();

        let check = resolved
            .effective
            .checks
            .get("deps.default_features_explicit")
            .expect("default_features check should exist");
        assert!(check.enabled);
        assert_eq!(check.severity, Severity::Warning);
    }

    #[test]
    fn validation_error_can_be_extracted_from_anyhow() {
        use crate::ValidationError;

        let cfg = DepguardConfigV1 {
            scope: Some("invalid".to_string()),
            ..Default::default()
        };
        let result = resolve_config(cfg, Overrides::default());
        assert!(result.is_err());

        let err = result.unwrap_err();
        // The ValidationError should be extractable via downcast
        let validation_err = err.downcast_ref::<ValidationError>();
        assert!(
            validation_err.is_some(),
            "should be able to downcast to ValidationError"
        );

        let ve = validation_err.unwrap();
        assert_eq!(ve.key_path(), "scope");
        assert!(ve.message().contains("invalid"));
        assert_eq!(ve.suggestion(), Some("expected 'repo' or 'diff'"));
        assert_eq!(ve.file_path(), None);
        assert_eq!(ve.line(), None);
    }

    #[test]
    fn validation_error_with_file_info() {
        use crate::ValidationError;
        use std::path::PathBuf;

        let err = ValidationError::unknown_severity("deps.no_wildcards", "fatal")
            .with_file(PathBuf::from("depguard.toml"))
            .with_line(10);

        let display = err.to_string();
        assert!(
            display.contains("depguard.toml:10"),
            "should contain file and line: {display}"
        );
        assert!(
            display.contains("checks.deps.no_wildcards.severity"),
            "should contain key path: {display}"
        );
    }

    #[test]
    fn validation_errors_can_aggregate_multiple() {
        use crate::{ValidationError, ValidationErrors};

        let mut errors = ValidationErrors::new();
        errors.push(ValidationError::unknown_scope("invalid"));
        errors.push(ValidationError::unknown_fail_on("never"));
        errors.push(ValidationError::unknown_severity("some.check", "bad"));

        assert_eq!(errors.len(), 3);
        assert!(!errors.is_empty());

        // Verify iteration works
        let count = errors.iter().count();
        assert_eq!(count, 3);

        // Verify display shows all errors
        let display = errors.to_string();
        assert!(display.contains("scope:"));
        assert!(display.contains("fail_on:"));
        assert!(display.contains("checks.some.check.severity:"));
    }

    #[test]
    fn validation_error_backwards_compatible_with_anyhow() {
        // Ensure that ValidationError works seamlessly with anyhow's context
        let cfg = DepguardConfigV1 {
            fail_on: Some("invalid_value".to_string()),
            ..Default::default()
        };
        let result = resolve_config(cfg, Overrides::default());

        let err_msg = result.unwrap_err().to_string();
        // The error message should still be human-readable
        assert!(err_msg.contains("fail_on:"));
        assert!(err_msg.contains("unknown fail_on"));
        assert!(err_msg.contains("hint:") || err_msg.contains("expected"));
    }

    #[test]
    fn invalid_profile_returns_error() {
        let cfg = DepguardConfigV1 {
            profile: Some("invalid_profile".to_string()),
            ..Default::default()
        };
        let result = resolve_config(cfg, Overrides::default());
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("profile:"),
            "error message should contain key path: {err_msg}"
        );
        assert!(
            err_msg.contains("unknown profile"),
            "error message should contain 'unknown profile': {err_msg}"
        );
    }

    #[test]
    fn invalid_max_findings_zero_returns_error() {
        let cfg = DepguardConfigV1 {
            max_findings: Some(0),
            ..Default::default()
        };
        let result = resolve_config(cfg, Overrides::default());
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("max_findings:"),
            "error message should contain key path: {err_msg}"
        );
        assert!(
            err_msg.contains("at least 1"),
            "error message should contain 'at least 1': {err_msg}"
        );
    }

    #[test]
    fn ignore_publish_false_on_unsupported_check_returns_error() {
        let toml = r#"
            [checks."deps.no_wildcards"]
            ignore_publish_false = true
        "#;
        let cfg = parse_config_toml(toml).unwrap();
        let result = resolve_config(cfg, Overrides::default());
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("checks.deps.no_wildcards.ignore_publish_false"),
            "error message should contain key path: {err_msg}"
        );
        assert!(
            err_msg.contains("not supported"),
            "error message should contain 'not supported': {err_msg}"
        );
    }

    #[test]
    fn ignore_publish_false_on_supported_check_works() {
        let toml = r#"
            [checks."deps.path_requires_version"]
            ignore_publish_false = true
        "#;
        let cfg = parse_config_toml(toml).unwrap();
        let result = resolve_config(cfg, Overrides::default());
        assert!(result.is_ok());
        let resolved = result.unwrap();
        let check = resolved
            .effective
            .checks
            .get("deps.path_requires_version")
            .expect("check should exist");
        assert!(check.ignore_publish_false);
    }

    #[test]
    fn valid_profile_aliases_work() {
        for profile in ["strict", "warn", "team", "compat", "oss"] {
            let cfg = DepguardConfigV1 {
                profile: Some(profile.to_string()),
                ..Default::default()
            };
            let result = resolve_config(cfg, Overrides::default());
            assert!(
                result.is_ok(),
                "profile '{profile}' should be valid: {:?}",
                result.err()
            );
        }
    }
}
