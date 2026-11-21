//! Verify that the obsolete `budget_config` module has been removed and that
//! `config::BudgetConfig` remains the sole config parser.

use std::path::PathBuf;

fn crate_src_dir() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("src");
    path
}

/// `src/budget_config.rs` must not exist. It was an obsolete file that
/// duplicated `BudgetConfig` from `config.rs`; removing it prevents drift.
#[test]
fn no_obsolete_budget_config_module_file() {
    let budget_config = crate_src_dir().join("budget_config.rs");
    assert!(
        !budget_config.exists(),
        "src/budget_config.rs must not exist — it is an obsolete module removed by \
         issue #1099. Delete the file and any `mod budget_config;` declaration in main.rs."
    );
}

/// `main.rs` must not declare a `budget_config` module.
#[test]
fn main_rs_does_not_declare_budget_config_mod() {
    let main_path = crate_src_dir().join("main.rs");
    let src = std::fs::read_to_string(&main_path)
        .unwrap_or_else(|e| panic!("Failed to read main.rs: {e}"));
    assert!(
        !src.contains("mod budget_config"),
        "main.rs must not declare `mod budget_config` — the module was removed by issue #1099"
    );
}

/// The canonical `BudgetConfig` struct lives in `config.rs`.
#[test]
fn budget_config_is_defined_in_config_module() {
    let config_path = crate_src_dir().join("config.rs");
    let src = std::fs::read_to_string(&config_path)
        .unwrap_or_else(|e| panic!("Failed to read config.rs: {e}"));
    assert!(
        src.contains("struct BudgetConfig"),
        "config.rs must define `BudgetConfig` — it is the sole canonical config parser"
    );
}
