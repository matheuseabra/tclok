use std::process::Command;

#[test]
fn redirected_stdout_is_a_single_plain_timestamp() {
    let output = Command::new(env!("CARGO_BIN_EXE_tclok"))
        .args(["--12h", "--no-seconds"])
        .output()
        .expect("launch tclok");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("UTF-8 timestamp");
    assert!(stdout.contains(':'));
    assert!(!stdout.contains('\x1b'));
    assert_eq!(stdout.lines().count(), 1);
}
