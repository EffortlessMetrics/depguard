use camino::{Utf8Path, Utf8PathBuf};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Canonical repo-relative path used in findings and reports.
///
/// Normalization rules are intentionally simple and deterministic:
/// - always forward slashes (`/`)
/// - no leading `./`
/// - never absolute (best-effort: absolute inputs are preserved but flagged by checks)
#[derive(
    Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(transparent)]
pub struct RepoPath(String);

impl Default for RepoPath {
    fn default() -> Self {
        RepoPath::new(".")
    }
}

impl RepoPath {
    pub fn new<S: AsRef<str>>(s: S) -> Self {
        let mut v = s.as_ref().replace('\\', "/");
        while v.starts_with("./") {
            v = v.trim_start_matches("./").to_string();
        }
        // Avoid empty path; keep it explicit.
        if v.is_empty() {
            v = ".".to_string();
        }
        Self(v)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn to_utf8_pathbuf(&self) -> Utf8PathBuf {
        Utf8PathBuf::from(self.0.clone())
    }

    pub fn join(&self, segment: &str) -> RepoPath {
        let base = Utf8Path::new(self.as_str());
        RepoPath::new(base.join(segment).as_str())
    }
}

impl From<&Utf8Path> for RepoPath {
    fn from(value: &Utf8Path) -> Self {
        RepoPath::new(value.as_str())
    }
}

impl From<Utf8PathBuf> for RepoPath {
    fn from(value: Utf8PathBuf) -> Self {
        RepoPath::new(value.as_str())
    }
}
