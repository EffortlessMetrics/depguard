use assert_cmd::Command;

/// Helper to get a Command for the depguard binary.
#[allow(deprecated)]
fn depguard_cmd() -> Command {
    Command::cargo_bin("depguard").unwrap()
}

#[test]
fn help_works() {
    depguard_cmd().arg("--help").assert().success();
}
