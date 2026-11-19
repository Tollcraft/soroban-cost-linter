use std::process::Command;

fn list_lints(format: &str) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_cargo-cost-lint"))
        .args(["--list-lints", "--format", format])
        .output()
        .expect("Failed to execute cargo-cost-lint")
}

#[test]
fn list_lints_rejects_github_format() {
    let output = list_lints("github");
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty(), "stdout must stay empty");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--list-lints"), "stderr: {stderr}");
}

#[test]
fn list_lints_rejects_sarif_format() {
    let output = list_lints("sarif");
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty(), "stdout must stay empty");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--list-lints"), "stderr: {stderr}");
}
