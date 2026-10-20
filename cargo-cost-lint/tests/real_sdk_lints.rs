use serde_json::Value;
use std::collections::HashSet;
use std::env;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn real_sdk_28_exercises_nested_type_paths() {
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("cargo-cost-lint should be a workspace member")
        .to_path_buf();
    let fixture_dir = workspace_root.join("soroban_cost_lints/test_fixtures/real_sdk");
    let lint_dir = workspace_root.join("soroban_cost_lints");

    let build_status = Command::new(env!("CARGO"))
        .arg("build")
        .current_dir(&lint_dir)
        .status()
        .expect("failed to build the Soroban lint library");
    assert!(
        build_status.success(),
        "failed to build the Soroban lint library"
    );

    let lib_dir = match env::var("CARGO_TARGET_DIR") {
        Ok(path) => PathBuf::from(path).join("debug"),
        Err(_) => workspace_root.join("target/debug"),
    };
    let output = Command::new(env!("CARGO_BIN_EXE_cargo-cost-lint"))
        .arg("--format")
        .arg("json")
        .current_dir(&fixture_dir)
        .env("DYLINT_LIBRARY_PATH", lib_dir)
        .output()
        .expect("failed to run cargo-cost-lint against the real SDK fixture");

    let stdout = String::from_utf8(output.stdout).expect("linter output was not UTF-8");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success() || !stdout.trim().is_empty(),
        "cargo-cost-lint failed without findings:\n{stderr}"
    );

    let findings: Vec<Value> = stdout
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| serde_json::from_str(line).expect("finding was not valid JSON"))
        .collect();
    let names: HashSet<&str> = findings
        .iter()
        .filter_map(|finding| finding["name"].as_str())
        .collect();

    for lint in [
        "symbol_new_for_short_literal",
        "unnecessary_string_to_bytes",
        "bytes_append_in_loop",
        "soroban_inefficient_bytes_concat",
        "host_in_loop",
        "string_concat_in_loop",
    ] {
        assert!(
            names.contains(lint),
            "expected real SDK fixture to trigger {lint}; got {names:?}"
        );
    }

    let lookalike_line = include_str!("../../soroban_cost_lints/test_fixtures/real_sdk/src/lib.rs")
        .lines()
        .position(|line| line.contains("Should Not Warn"))
        .expect("lookalike fixture call was missing")
        + 1;
    assert!(
        !findings.iter().any(|finding| {
            finding["name"] == "bytes_append_in_loop"
                && finding["span"]["line_start"] == lookalike_line
        }),
        "user-defined lookalike type unexpectedly matched bytes_append_in_loop"
    );
}
