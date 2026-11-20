use std::process::Command;

fn cost_lint(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_cargo-cost-lint"))
        .args(args)
        .output()
        .expect("Failed to execute cargo-cost-lint")
}

#[test]
fn explain_with_uppercase_lint_name_succeeds() {
    let output = cost_lint(&["--explain", "SOROBAN_STORAGE_IN_LOOP"]);
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("soroban_storage_in_loop"),
        "stdout should contain explanation text: {stdout}"
    );
}

#[test]
fn explain_with_mixed_case_lint_name_succeeds() {
    let output = cost_lint(&["--explain", "Soroban_Storage_In_Loop"]);
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("soroban_storage_in_loop"),
        "stdout should contain explanation text: {stdout}"
    );
}

#[test]
fn explain_with_unknown_lint_exits_code_1_and_empty_stdout() {
    let output = cost_lint(&["--explain", "not_a_real_lint"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(
        output.stdout.is_empty(),
        "stdout must stay empty on unknown lint, got: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Error: unknown lint 'not_a_real_lint'"),
        "stderr should contain unknown lint error: {stderr}"
    );
    assert!(
        stderr.contains("Valid lints:"),
        "stderr should list valid lints: {stderr}"
    );
}
