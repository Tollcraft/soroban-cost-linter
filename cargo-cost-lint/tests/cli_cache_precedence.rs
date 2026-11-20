use std::process::Command;

fn cost_lint(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_cargo-cost-lint"))
        .args(args)
        .output()
        .expect("Failed to execute cargo-cost-lint")
}

#[test]
fn clear_cache_with_list_lints_reports_usage_error() {
    let output = cost_lint(&["--clear-cache", "--list-lints"]);
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("cannot be used with") || stderr.contains("Usage:"),
        "stderr should describe the argument conflict: {stderr}"
    );
    assert!(
        stderr.contains("--clear-cache") && stderr.contains("--list-lints"),
        "stderr should name the conflicting arguments: {stderr}"
    );
}

#[test]
fn clear_cache_with_explain_reports_usage_error() {
    let output = cost_lint(&["--clear-cache", "--explain", "soroban_storage_in_loop"]);
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("cannot be used with") || stderr.contains("Usage:"),
        "stderr should describe the argument conflict: {stderr}"
    );
    assert!(
        stderr.contains("--clear-cache") && stderr.contains("--explain"),
        "stderr should name the conflicting arguments: {stderr}"
    );
}

#[test]
fn list_lints_with_clear_cache_reports_usage_error() {
    let output = cost_lint(&["--list-lints", "--clear-cache"]);
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("cannot be used with") || stderr.contains("Usage:"),
        "stderr should describe the argument conflict: {stderr}"
    );
}
