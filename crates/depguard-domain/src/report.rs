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
