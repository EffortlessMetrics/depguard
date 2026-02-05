use sha2::{Digest, Sha256};

/// Compute a stable SHA-256 fingerprint for a dependency finding.
///
/// Identity fields:
/// - check_id
/// - code
/// - manifest_path (repo-relative)
/// - dependency name
/// - dependency path (if present)
pub fn fingerprint_for_dep(
    check_id: &str,
    code: &str,
    manifest_path: &str,
    dep_name: &str,
    dep_path: Option<&str>,
) -> String {
    let mut parts = vec![check_id, code, manifest_path, dep_name];
    if let Some(p) = dep_path {
        parts.push(p);
    }
    let canonical = parts.join("|");

    let mut hasher = Sha256::new();
    hasher.update(canonical.as_bytes());
    let digest = hasher.finalize();
    hex::encode(digest)
}
