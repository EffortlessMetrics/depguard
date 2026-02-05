#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RenderableSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RenderableVerdictStatus {
    Pass,
    Warn,
    Fail,
    Skip,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenderableLocation {
    pub path: String,
    pub line: Option<u32>,
    pub col: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenderableFinding {
    pub severity: RenderableSeverity,
    pub check_id: Option<String>,
    pub code: String,
    pub message: String,
    pub location: Option<RenderableLocation>,
    pub help: Option<String>,
    pub url: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenderableData {
    pub findings_emitted: u32,
    pub findings_total: u32,
    pub truncated_reason: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenderableReport {
    pub verdict: RenderableVerdictStatus,
    pub findings: Vec<RenderableFinding>,
    pub data: RenderableData,
}
