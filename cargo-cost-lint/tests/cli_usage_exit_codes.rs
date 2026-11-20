use std::process::Command;

fn run_cli(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_cargo-cost-lint"))
        .args(args)
        .output()
        .expect("Failed to execute cargo-cost-lint")
}

#[test]
fn conflicting_workspace_and_package_exits_with_code_2() {
    let output = run_cli(&["--workspace", "--package", "foo"]);
    assert_eq!(
        output.status.code(),
        Some(2),
        "Conflicting package and workspace should exit with code 2"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("cannot be used with") || stderr.contains("mutually exclusive"),
        "stderr should describe the conflict: {stderr}"
    );
}

#[test]
fn unknown_package_name_exits_with_code_2() {
    let output = run_cli(&["--package", "nonexistent_contract_pkg_12345"]);
    assert_eq!(
        output.status.code(),
        Some(2),
        "Unknown package selection should exit with code 2"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("not found in workspace"),
        "stderr should describe missing package: {stderr}"
    );
}

#[test]
fn clap_unknown_flag_exits_with_code_2() {
    let output = run_cli(&["--nonexistent-flag-xyz"]);
    assert_eq!(
        output.status.code(),
        Some(2),
        "Unknown clap flag should exit with code 2"
    );
}

#[test]
fn clap_conflicting_flags_exit_with_code_2() {
    let output = run_cli(&["--quiet", "--verbose"]);
    assert_eq!(
        output.status.code(),
        Some(2),
        "Conflicting flags should exit with code 2"
    );
}

#[test]
fn list_lints_invalid_format_exits_with_code_2() {
    let output = run_cli(&["--list-lints", "--format", "sarif"]);
    assert_eq!(
        output.status.code(),
        Some(2),
        "Invalid format for list-lints should exit with code 2"
    );
}
