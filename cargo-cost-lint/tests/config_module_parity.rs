//! Invariant tests confirming `config::BudgetConfig` is the sole config parser
//! (issue #1099).
//!
//! The obsolete `budget_config.rs` module implemented a parallel parser for
//! the `[budget.lints]` wrapper schema; it has been removed. These tests pin
//! the canonical parser's behaviour so a re-introduction would be caught.

#[path = "../src/error.rs"]
mod error;

#[path = "../src/config.rs"]
mod config;

use config::BudgetConfig;
use std::path::Path;
use tempfile::tempdir;

fn known_lints() -> &'static [&'static str] {
    &["soroban_storage_in_loop", "soroban_redundant_auth"]
}

#[test]
fn budget_config_parses_flat_lints_section() {
    let tmp = tempdir().unwrap();
    let path = tmp.path().join("budget.toml");
    std::fs::write(&path, "[lints]\nsoroban_storage_in_loop = \"warn\"\n").unwrap();

    let cfg = BudgetConfig::from_file_validated(&path, known_lints()).unwrap();
    let lints = cfg.lints.as_ref().expect("lints section must be present");
    assert_eq!(
        lints.get("soroban_storage_in_loop").map(String::as_str),
        Some("warn")
    );
}

#[test]
fn obsolete_budget_wrapper_schema_does_not_populate_lints() {
    // The [budget.lints] schema was used by the removed budget_config module.
    // The canonical parser must not silently accept it.
    let tmp = tempdir().unwrap();
    let path = tmp.path().join("budget.toml");
    std::fs::write(
        &path,
        "[budget]\n[budget.lints]\nsoroban_storage_in_loop = \"warn\"\n",
    )
    .unwrap();

    match BudgetConfig::from_file_validated(&path, known_lints()) {
        Ok(cfg) => {
            let lint_count = cfg.lints.as_ref().map(|m| m.len()).unwrap_or(0);
            assert_eq!(
                lint_count, 0,
                "obsolete [budget.lints] schema must not populate the lint map"
            );
        }
        Err(_) => {
            // A hard parse error is also acceptable; the caller gets a clear diagnostic.
        }
    }
}

#[test]
fn invalid_lint_level_is_rejected() {
    let tmp = tempdir().unwrap();
    let path = tmp.path().join("budget.toml");
    std::fs::write(&path, "[lints]\nsoroban_storage_in_loop = \"ignore\"\n").unwrap();

    let err = BudgetConfig::from_file_validated(&path, known_lints()).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("ignore") || msg.contains("level"),
        "error must mention the bad level: {msg}"
    );
}

#[test]
fn unknown_lint_name_is_rejected() {
    let tmp = tempdir().unwrap();
    let path = tmp.path().join("budget.toml");
    std::fs::write(&path, "[lints]\nunknown_lint = \"warn\"\n").unwrap();

    let err = BudgetConfig::from_file_validated(&path, known_lints()).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("unknown_lint"),
        "error must name the unknown lint: {msg}"
    );
}

#[test]
fn missing_config_file_returns_io_error() {
    let err =
        BudgetConfig::from_file_validated(Path::new("/nonexistent/budget.toml"), known_lints())
            .unwrap_err();
    assert!(
        err.to_string().contains("Failed to read"),
        "error must indicate read failure: {err}"
    );
}

#[test]
fn empty_lints_section_is_valid() {
    let tmp = tempdir().unwrap();
    let path = tmp.path().join("budget.toml");
    std::fs::write(&path, "[lints]\n").unwrap();

    let cfg = BudgetConfig::from_file_validated(&path, known_lints()).unwrap();
    let count = cfg.lints.as_ref().map(|m| m.len()).unwrap_or(0);
    assert_eq!(count, 0);
}
