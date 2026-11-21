//! Integration test: SARIF rule `shortDescription.text` is populated from the
//! static `LINT_INFO` table rather than the generic `"Cost lint rule: <name>"`
//! placeholder.
//!
//! Regression guard for the fix to issue #1092: `generate_sarif_report` now
//! accepts a `lint_info` slice and uses it when constructing SARIF rule
//! descriptors.

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

fn run_sarif() -> serde_json::Value {
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

    serde_json::from_str(&stdout).expect("SARIF output must be valid JSON")
}

#[test]
fn sarif_rule_short_descriptions_are_not_placeholder_text() {
    let report = run_sarif();
    let rules = report["runs"][0]["tool"]["driver"]["rules"]
        .as_array()
        .expect("`tool.driver.rules` must be an array");

    assert!(
        !rules.is_empty(),
        "the fixture must produce at least one SARIF rule descriptor"
    );

    for rule in rules {
        let id = rule["id"].as_str().expect("rule must have a string `id`");
        let desc = rule["shortDescription"]["text"]
            .as_str()
            .unwrap_or_else(|| panic!("rule `{id}` must have `shortDescription.text` as a string"));

        // The fix ensures that known lints get their real description from
        // LINT_INFO rather than the generic placeholder.
        let placeholder = format!("Cost lint rule: {id}");
        assert_ne!(
            desc,
            placeholder.as_str(),
            "rule `{id}` still uses the generic placeholder description; \
             it should come from LINT_INFO"
        );

        // The description must not be empty.
        assert!(
            !desc.trim().is_empty(),
            "rule `{id}` has an empty shortDescription.text"
        );
    }
}

#[test]
fn sarif_rule_short_description_lookup_falls_back_for_unknown_rule() {
    // Verify that the fallback path ("Cost lint rule: <name>") is exercised when
    // lint_info contains no entry for a given name. This is tested through the
    // unit tests in output_formatters.rs; here we confirm the binary still emits
    // valid, non-empty shortDescription text for every rule it emits.
    let report = run_sarif();
    let rules = report["runs"][0]["tool"]["driver"]["rules"]
        .as_array()
        .expect("`tool.driver.rules` must be an array");

    for rule in rules {
        let id = rule["id"].as_str().expect("rule must have string `id`");
        let desc_text = &rule["shortDescription"]["text"];
        assert!(
            desc_text.is_string() && !desc_text.as_str().unwrap_or("").trim().is_empty(),
            "rule `{id}` must have a non-empty string `shortDescription.text`"
        );
    }
}
