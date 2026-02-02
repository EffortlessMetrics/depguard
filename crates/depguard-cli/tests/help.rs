use assert_cmd::Command;

#[test]
fn help_works() {
    let mut cmd = Command::cargo_bin("depguard").unwrap();
    cmd.arg("--help");
    cmd.assert().success();
}
