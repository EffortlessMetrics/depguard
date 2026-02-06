//! Config parsing and profile/preset resolution.
//!
//! This crate is intentionally IO-free: it parses and resolves configuration provided as strings.

#![forbid(unsafe_code)]

mod model;
mod presets;
mod resolve;

pub use model::{CheckConfig, DepguardConfigV1};
pub use resolve::{Overrides, ResolvedConfig};

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
    use depguard_domain::policy::{FailOn, Scope};
    use depguard_types::Severity;

    #[test]
    fn parse_empty_config() {
        let cfg = parse_config_toml("").unwrap();
        assert_eq!(cfg.profile, None);
        assert_eq!(cfg.scope, None);
        assert!(cfg.checks.is_empty());
    }

    #[test]
    fn parse_minimal_config() {
        let toml = r#"
            profile = "warn"
            scope = "diff"
        "#;
        let cfg = parse_config_toml(toml).unwrap();
        assert_eq!(cfg.profile, Some("warn".to_string()));
        assert_eq!(cfg.scope, Some("diff".to_string()));
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
        };
        let resolved = resolve_config(cfg, overrides).unwrap();

        assert_eq!(resolved.effective.profile, "strict");
        assert_eq!(resolved.effective.scope, Scope::Diff);
        assert_eq!(resolved.effective.max_findings, 50);
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
    fn invalid_scope_returns_error() {
        let cfg = DepguardConfigV1 {
            scope: Some("invalid".to_string()),
            ..Default::default()
        };
        let result = resolve_config(cfg, Overrides::default());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("unknown scope"));
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
        assert!(result.unwrap_err().to_string().contains("invalid severity"));
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
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("invalid allow glob for deps.no_wildcards")
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
        assert!(result.unwrap_err().to_string().contains("unknown fail_on"));
    }
}
