//! Integration tests for `--config <PATH>` error diagnostics when the
//! specified config file does not exist.

use std::process::Command;

/// When `--config` points to a nonexistent file and a default `budget.toml`
/// exists, the error message should suggest it.
#[test]
fn config_not_found_suggests_default_budget_toml() {
    let temp = tempfile::tempdir().unwrap();
    let budget = temp.path().join("budget.toml");
    std::fs::write(&budget, "[lints]\n").unwrap();

    let bin_path = env!("CARGO_BIN_EXE_cargo-cost-lint");
    let output = Command::new(bin_path)
        .arg("--config")
        .arg("typo-budget.toml")
        .current_dir(temp.path())
        .output()
        .expect("failed to run cargo-cost-lint");

    assert!(!output.status.success(), "should exit non-zero");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains("does not exist"),
        "error must mention 'does not exist'; got: {stderr}"
    );
    assert!(
        stderr.contains("Did you mean"),
        "error must include a suggestion; got: {stderr}"
    );
    assert!(
        stderr.contains("budget.toml"),
        "suggestion must name the found budget.toml; got: {stderr}"
    );
}

/// When `--config` points to a nonexistent file and no default `budget.toml`
/// exists either, the error message still reports the missing file but adds no
/// misleading suggestion.
#[test]
fn config_not_found_no_suggestion_when_no_default() {
    let temp = tempfile::tempdir().unwrap();

    let bin_path = env!("CARGO_BIN_EXE_cargo-cost-lint");
    let output = Command::new(bin_path)
        .arg("--config")
        .arg("missing.toml")
        .current_dir(temp.path())
        .output()
        .expect("failed to run cargo-cost-lint");

    assert!(!output.status.success(), "should exit non-zero");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains("does not exist"),
        "error must mention 'does not exist'; got: {stderr}"
    );
    assert!(
        !stderr.contains("Did you mean"),
        "no suggestion expected when no budget.toml exists; got: {stderr}"
    );
}
