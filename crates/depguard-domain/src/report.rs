use depguard_types::{DepguardData, Finding, Severity, Verdict};

#[derive(Clone, Debug, Default)]
pub struct SeverityCounts {
    pub info: u32,
    pub warning: u32,
    pub error: u32,
}

impl SeverityCounts {
    pub fn from_findings(findings: &[Finding]) -> Self {
        let mut counts = SeverityCounts::default();
        for f in findings {
            match f.severity {
                Severity::Info => counts.info += 1,
                Severity::Warning => counts.warning += 1,
                Severity::Error => counts.error += 1,
            }
        }
        counts
    }
}

#[derive(Clone, Debug)]
pub struct DomainReport {
    pub verdict: Verdict,
    pub findings: Vec<Finding>,
    pub data: DepguardData,
    pub counts: SeverityCounts,
}

#[cfg(test)]
mod tests {
    use super::*;
    use depguard_types::Severity;

    #[test]
    fn counts_from_findings() {
        let findings = vec![
            Finding {
                severity: Severity::Info,
                check_id: "a".to_string(),
                code: "a".to_string(),
                message: "a".to_string(),
                location: None,
                help: None,
                url: None,
                fingerprint: None,
                data: serde_json::Value::Null,
            },
            Finding {
                severity: Severity::Warning,
                check_id: "b".to_string(),
                code: "b".to_string(),
                message: "b".to_string(),
                location: None,
                help: None,
                url: None,
                fingerprint: None,
                data: serde_json::Value::Null,
            },
            Finding {
                severity: Severity::Error,
                check_id: "c".to_string(),
                code: "c".to_string(),
                message: "c".to_string(),
                location: None,
                help: None,
                url: None,
                fingerprint: None,
                data: serde_json::Value::Null,
            },
        ];

        let counts = SeverityCounts::from_findings(&findings);
        assert_eq!(counts.info, 1);
        assert_eq!(counts.warning, 1);
        assert_eq!(counts.error, 1);
    }
}
