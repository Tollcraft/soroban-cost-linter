use std::process::Command;

fn cost_lint(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_cargo-cost-lint"))
        .args(args)
        .output()
        .expect("Failed to execute cargo-cost-lint")
}

#[test]
fn unknown_lint_passed_to_allow_exits_with_error() {
    let output = cost_lint(&["--allow", "nonexistent_lint_name"]);
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("nonexistent_lint_name"),
        "stderr should name the unrecognized lint: {stderr}"
    );
    assert!(
        stderr.contains("Unknown lint name"),
        "stderr should indicate unknown lint error: {stderr}"
    );
    assert!(
        stderr.contains("Valid lints are:"),
        "stderr should list available registered lints: {stderr}"
    );
    assert!(
        stderr.contains("soroban_storage_in_loop"),
        "stderr should mention known lints: {stderr}"
    );
}

#[test]
fn unknown_lint_passed_to_deny_exits_with_error() {
    let output = cost_lint(&["--deny", "bogus_lint_xyz"]);
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("bogus_lint_xyz"),
        "stderr should name the unrecognized lint: {stderr}"
    );
    assert!(
        stderr.contains("Unknown lint name"),
        "stderr should indicate unknown lint error: {stderr}"
    );
    assert!(
        stderr.contains("Valid lints are:"),
        "stderr should list available registered lints: {stderr}"
    );
    assert!(
        stderr.contains("soroban_storage_in_loop"),
        "stderr should mention known lints: {stderr}"
    );
}
