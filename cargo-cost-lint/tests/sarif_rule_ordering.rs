//! Integration test: `tool.driver.rules` in SARIF output is emitted in
//! deterministic lexicographic order across multiple process invocations.
//!
//! Regression guard for the `HashSet` → `BTreeSet` change that fixed #1091:
//! the SARIF rules array must be sorted ascending by rule ID so that repeated
//! runs on the same input produce byte-identical JSON.

use std::path::PathBuf;
use std::process::Command;

fn workspace_root() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p
}

fn ensure_lint_library_built() -> PathBuf {
    let root = workspace_root();
    let lint_dir = root.join("soroban_cost_lints");

    let status = Command::new(env!("CARGO"))
        .arg("build")
        .current_dir(&lint_dir)
        .status()
        .expect("Failed to spawn `cargo build` for soroban_cost_lints");
    assert!(status.success(), "Failed to build soroban_cost_lints");

    match std::env::var("CARGO_TARGET_DIR") {
        Ok(dir) => PathBuf::from(dir).join("debug"),
        Err(_) => root.join("target").join("debug"),
    }
}

fn run_sarif() -> String {
    let bin_path = env!("CARGO_BIN_EXE_cargo-cost-lint");
    let lib_dir = ensure_lint_library_built();
    let fixture_dir = workspace_root().join("test_fixtures/smoke_test");

    let output = Command::new(bin_path)
        .args(["--format", "sarif", "--no-cache"])
        .current_dir(&fixture_dir)
        .env("DYLINT_LIBRARY_PATH", &lib_dir)
        .output()
        .expect("Failed to execute cargo-cost-lint");

    let stdout =
        String::from_utf8(output.stdout).expect("`--format sarif` stdout is not valid UTF-8");
    assert!(
        !stdout.trim().is_empty(),
        "`--format sarif` produced empty stdout; the fixture must emit at least one finding"
    );
    stdout
}

#[test]
fn sarif_rules_are_in_lexicographic_order() {
    let sarif = run_sarif();
    let report: serde_json::Value =
        serde_json::from_str(&sarif).expect("SARIF output must be valid JSON");

    let rules = report["runs"][0]["tool"]["driver"]["rules"]
        .as_array()
        .expect("`tool.driver.rules` must be an array");

    if rules.len() < 2 {
        // A single rule can't be out of order; nothing to assert.
        return;
    }

    let ids: Vec<&str> = rules
        .iter()
        .map(|r| {
            r["id"]
                .as_str()
                .expect("every rule must have a string `id`")
        })
        .collect();

    let mut sorted = ids.clone();
    sorted.sort_unstable();

    assert_eq!(
        ids, sorted,
        "SARIF `tool.driver.rules` must be sorted lexicographically by rule ID; \
         got {ids:?}, expected {sorted:?}"
    );
}

#[test]
fn sarif_rules_are_byte_identical_across_two_runs() {
    // Two consecutive runs on the same fixture must produce byte-identical SARIF
    // reports, confirming that rule ordering is not hash-randomised.
    let first = run_sarif();
    let second = run_sarif();
    assert_eq!(
        first, second,
        "Two consecutive SARIF runs on the same fixture produced different output; \
         rule ordering is non-deterministic"
    );
}
