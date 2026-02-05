use globset::{Glob, GlobSet, GlobSetBuilder};

pub fn build_allowlist(allow: &[String]) -> Option<GlobSet> {
    if allow.is_empty() {
        return None;
    }

    let mut builder = GlobSetBuilder::new();
    for pattern in allow {
        // Treat allowlist entries as glob patterns (case-sensitive).
        let glob =
            Glob::new(pattern).expect("allowlist patterns must be validated in depguard-settings");
        builder.add(glob);
    }
    Some(
        builder
            .build()
            .expect("allowlist patterns must be validated in depguard-settings"),
    )
}

pub fn is_allowed(allow: Option<&GlobSet>, value: &str) -> bool {
    allow.map(|set| set.is_match(value)).unwrap_or(false)
}
