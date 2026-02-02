//! Developer tasks (schema generation, fixture updates, packaging).
//!
//! Keeping this separate avoids bloating the end-user CLI.

use anyhow::Context;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let cmd = args.get(1).map(|s| s.as_str()).unwrap_or("help");

    match cmd {
        "help" => {
            eprintln!("xtask commands:");
            eprintln!("  help             Show this message");
            eprintln!("  print-schema-ids Print known schema IDs (placeholder)");
            Ok(())
        }
        "print-schema-ids" => {
            // Placeholder: in a fuller implementation, this would generate JSON schemas from Rust types.
            println!("receipt.envelope.v1");
            println!("depguard.report.v1");
            Ok(())
        }
        other => anyhow::bail!("unknown xtask command: {other}"),
    }
    .context("xtask failed")
}
