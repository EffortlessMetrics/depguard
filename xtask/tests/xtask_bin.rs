use std::process::Command;

#[test]
fn xtask_help_runs() {
    let exe = env!("CARGO_BIN_EXE_xtask");
    let output = Command::new(exe)
        .arg("help")
        .output()
        .expect("run xtask");

    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("xtask commands"));
}
