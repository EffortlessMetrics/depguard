//! Fuzz target for TOML manifest parsing.
//!
//! Goal: The parser should **never panic** on any input.
//! It may return errors, but panics are unacceptable.
//!
//! Run with:
//! ```bash
//! cargo +nightly fuzz run fuzz_toml_parser
//! ```

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Only test valid UTF-8 strings (Cargo.toml must be UTF-8)
    if let Ok(text) = std::str::from_utf8(data) {
        // Test root manifest parsing - should never panic
        let _ = depguard_repo::fuzz::parse_root_manifest(text);

        // Test member manifest parsing - should never panic
        let _ = depguard_repo::fuzz::parse_member_manifest(text);
    }
});
