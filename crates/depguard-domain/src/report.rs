use depguard_types::{DepguardData, Finding, Verdict};

#[derive(Clone, Debug)]
pub struct DomainReport {
    pub verdict: Verdict,
    pub findings: Vec<Finding>,
    pub data: DepguardData,
}
