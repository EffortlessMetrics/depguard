//! Fuzz target for inline suppression parsing.
//!
//! Goal: The parser should **never panic** on any input.
//! It may return empty results for malformed directives.
//!
//! Run with:
//! ```bash
//! cargo +nightly fuzz run fuzz_inline_suppressions
//! ```

#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;

#[derive(Arbitrary, Debug)]
struct InlineSuppressionInput {
    source: String,
    line: u32,
}

fuzz_target!(|input: InlineSuppressionInput| {
    if input.source.len() > 16_384 {
        return;
    }

    let _ = depguard_repo::parser::parse_inline_suppressions(&input.source, input.line);
});
