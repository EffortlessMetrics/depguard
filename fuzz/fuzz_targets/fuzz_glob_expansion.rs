//! Fuzz target for workspace member glob expansion.
//!
//! Goal: The glob expansion should **never panic** on any input.
//! It may return errors for invalid patterns, but panics are unacceptable.
//!
//! Run with:
//! ```bash
//! cargo +nightly fuzz run fuzz_glob_expansion
//! ```

#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;

/// Structured input for glob expansion fuzzing.
/// Using Arbitrary allows libFuzzer to generate more meaningful test cases.
#[derive(Arbitrary, Debug)]
struct GlobInput {
    /// Glob patterns (e.g., "crates/*", "packages/**")
    patterns: Vec<String>,
    /// Candidate paths to match against
    candidates: Vec<String>,
}

fuzz_target!(|input: GlobInput| {
    // Limit input size to avoid OOM and keep fuzzing fast
    if input.patterns.len() > 20 || input.candidates.len() > 100 {
        return;
    }

    // Filter out excessively long strings
    let patterns: Vec<String> = input
        .patterns
        .into_iter()
        .filter(|p| p.len() <= 256)
        .collect();

    let candidates: Vec<String> = input
        .candidates
        .into_iter()
        .filter(|c| c.len() <= 512)
        .collect();

    // Should never panic - errors are fine
    let _ = depguard_repo::fuzz::expand_globs(&patterns, &candidates);
});
