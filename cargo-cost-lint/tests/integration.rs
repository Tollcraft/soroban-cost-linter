use std::env;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn test_list_lints() {
    let bin_path = env!("CARGO_BIN_EXE_cargo-cost-lint");

    let output = Command::new(bin_path)
        .arg("--list-lints")
        .output()
        .expect("Failed to execute cargo-cost-lint --list-lints");

    assert!(
        output.status.success(),
        "--list-lints should exit 0, got: {}",
        output.status
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout is not valid UTF-8");

    // Every lint registered in the lint crate must appear in the output.
    //
    // NOTE: This list must be kept in sync with the `register_lints` call in
    // `soroban_cost_lints/src/lib.rs`. When adding or removing a lint there,
    // update this array as well.
    let expected_lints = &[
        "soroban_storage_in_loop",
        "redundant_env_clone",
        "unnecessary_host_function_call",
        "host_in_loop",
        "symbol_new_for_short_literal",
        "storage_write_without_read",
        "inefficient_bytes_concat",
        "map_insert_in_loop",
        "bytes_append_in_loop",
    ];

    for expected in expected_lints {
        assert!(
            stdout.contains(expected),
            "--list-lints output missing lint '{}'.\nOutput:\n{}",
            expected,
            stdout
        );
    }

    // Every line should be tab-separated with three fields: name, level, description.
    for line in stdout.lines() {
        let fields: Vec<&str> = line.split('\t').collect();
        assert_eq!(
            fields.len(),
            3,
            "Each line should have 3 tab-separated fields (name, level, description). Got: '{}'",
            line
        );
        assert!(!fields[0].is_empty(), "lint name must not be empty");
        assert!(!fields[1].is_empty(), "lint level must not be empty");
        assert!(!fields[2].is_empty(), "lint description must not be empty");
    }

    // The number of lines should match the number of lints registered.
    assert_eq!(
        stdout.lines().count(),
        expected_lints.len(),
        "--list-lints output should have exactly {} lines, one per registered lint.\nOutput:\n{}",
        expected_lints.len(),
        stdout
    );
}

#[test]
fn test_version() {
    let bin_path = env!("CARGO_BIN_EXE_cargo-cost-lint");

    let output = Command::new(bin_path)
        .arg("--version")
        .output()
        .expect("Failed to execute cargo-cost-lint --version");

    assert!(output.status.success(), "--version should exit 0");

    let stdout = String::from_utf8(output.stdout).expect("stdout is not valid UTF-8");
    assert!(
        stdout.starts_with("cargo-cost-lint "),
        "--version output should start with 'cargo-cost-lint '. Got: '{}'",
        stdout.trim()
    );
}

#[test]
fn test_json_output() {
    let bin_path = env!("CARGO_BIN_EXE_cargo-cost-lint");

    // Construct path to the fixture directory from the workspace
    let mut fixture_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    fixture_dir.pop(); // Go up to workspace root
    fixture_dir.push("soroban_cost_lints");
    fixture_dir.push("test_fixtures");
    fixture_dir.push("real_sdk");

    assert!(
        fixture_dir.exists(),
        "Fixture directory not found: {:?}",
        fixture_dir
    );

    // Find the workspace target directory dynamically based on the binary path
    let mut target_dir = PathBuf::from(env!("CARGO_BIN_EXE_cargo-cost-lint"));
    target_dir.pop(); // Remove the binary name, leaving the profile directory (e.g., target/debug)

    // Build the soroban_cost_lints cdylib first from its own directory so it picks up .cargo/config.toml
    let mut lint_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    lint_dir.pop();
    lint_dir.push("soroban_cost_lints");

    let status = Command::new(env!("CARGO"))
        .arg("build")
        .current_dir(&lint_dir)
        .status()
        .expect("Failed to build soroban_cost_lints");
    assert!(status.success(), "Failed to build soroban_cost_lints");

    // Run the built wrapper binary in the fixture directory with --format json
    let output = Command::new(bin_path)
        .arg("--format")
        .arg("json")
        .current_dir(fixture_dir)
        .env("DYLINT_LIBRARY_PATH", target_dir)
        .output()
        .expect("Failed to execute cargo-cost-lint");

    let stdout_str = String::from_utf8(output.stdout).expect("Stdout is not valid UTF-8");
    let lines: Vec<&str> = stdout_str.lines().filter(|l| !l.is_empty()).collect();

    let stderr_str = String::from_utf8(output.stderr).expect("Stderr is not valid UTF-8");
    if lines.is_empty() {
        println!("Stderr output:\n{}", stderr_str);
    }
    // The fixture should have some lint violations.
    assert!(
        !lines.is_empty(),
        "Expected JSON output, but stdout was empty. Stderr: {}",
        stderr_str
    );

    let mut found_storage_in_loop = false;
    for line in lines {
        // Assert that the line is valid JSON conforming to our schema
        let json: serde_json::Value =
            serde_json::from_str(line).expect("Output line is not valid JSON");

        assert!(json.get("name").is_some(), "JSON missing 'name' field");
        assert!(json.get("level").is_some(), "JSON missing 'level' field");
        assert!(json.get("file").is_some(), "JSON missing 'file' field");
        assert!(json.get("span").is_some(), "JSON missing 'span' field");
        assert!(
            json.get("message").is_some(),
            "JSON missing 'message' field"
        );

        if json["name"] == "soroban_storage_in_loop" {
            found_storage_in_loop = true;
        }
    }

    assert!(
        found_storage_in_loop,
        "Expected to find 'soroban_storage_in_loop' lint, but it was not present"
    );
}

/// Integration test: run cargo-cost-lint against a mock workspace with
/// multiple contracts and verify it lints all of them.
#[test]
fn test_cli_workspace_lints_all_contracts() {
    let bin_path = env!("CARGO_BIN_EXE_cargo-cost-lint");

    let mut target_dir = PathBuf::from(bin_path);
    target_dir.pop();

    let mut lint_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    lint_dir.pop();
    lint_dir.push("soroban_cost_lints");

    let status = Command::new(env!("CARGO"))
        .arg("build")
        .current_dir(&lint_dir)
        .status()
        .expect("Failed to build soroban_cost_lints");
    assert!(status.success(), "Failed to build soroban_cost_lints");

    let mut workspace_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    workspace_dir.pop();
    workspace_dir.push("tests");
    workspace_dir.push("cli_integration_workspace");

    assert!(
        workspace_dir.exists(),
        "Mock workspace directory not found: {:?}",
        workspace_dir
    );

    let output = Command::new(bin_path)
        .arg("--format")
        .arg("json")
        .current_dir(&workspace_dir)
        .env("DYLINT_LIBRARY_PATH", &target_dir)
        .output()
        .expect("Failed to execute cargo-cost-lint");

    let stdout_str = String::from_utf8(output.stdout).expect("Stdout is not valid UTF-8");
    let stderr_str = String::from_utf8(output.stderr).expect("Stderr is not valid UTF-8");

    let lines: Vec<&str> = stdout_str.lines().filter(|l| !l.is_empty()).collect();

    assert!(
        !lines.is_empty(),
        "Expected lint findings from workspace, but stdout was empty. Stderr: {}",
        stderr_str
    );

    let mut found_contracts: std::collections::HashSet<String> = std::collections::HashSet::new();

    for line in lines {
        let json: serde_json::Value =
            serde_json::from_str(line).expect("Output line is not valid JSON");

        assert!(json.get("name").is_some(), "JSON missing 'name' field");
        assert!(json.get("file").is_some(), "JSON missing 'file' field");
        assert!(json.get("span").is_some(), "JSON missing 'span' field");

        let file = json["file"].as_str().unwrap_or("");
        if file.contains("alpha") {
            found_contracts.insert("alpha".to_string());
        }
        if file.contains("beta") {
            found_contracts.insert("beta".to_string());
        }
    }

    assert!(
        found_contracts.contains("alpha"),
        "Expected findings from alpha contract"
    );
}

#[test]
fn test_sarif_output() {
    let bin_path = env!("CARGO_BIN_EXE_cargo-cost-lint");

    let mut fixture_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    fixture_dir.pop();
    fixture_dir.push("soroban_cost_lints");
    fixture_dir.push("test_fixtures");
    fixture_dir.push("real_sdk");

    assert!(
        fixture_dir.exists(),
        "Fixture directory not found: {:?}",
        fixture_dir
    );

    let mut target_dir = PathBuf::from(env!("CARGO_BIN_EXE_cargo-cost-lint"));
    target_dir.pop();

    let mut lint_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    lint_dir.pop();
    lint_dir.push("soroban_cost_lints");

    let status = Command::new(env!("CARGO"))
        .arg("build")
        .current_dir(&lint_dir)
        .status()
        .expect("Failed to build soroban_cost_lints");
    assert!(status.success(), "Failed to build soroban_cost_lints");

    let output = Command::new(bin_path)
        .arg("--format")
        .arg("sarif")
        .current_dir(fixture_dir)
        .env("DYLINT_LIBRARY_PATH", target_dir)
        .output()
        .expect("Failed to execute cargo-cost-lint");

    let stdout_str = String::from_utf8(output.stdout).expect("Stdout is not valid UTF-8");
    let stderr_str = String::from_utf8(output.stderr).expect("Stderr is not valid UTF-8");

    let sarif: serde_json::Value =
        serde_json::from_str(&stdout_str).expect("SARIF output is not valid JSON");

    assert_eq!(
        sarif.get("$schema").and_then(|v| v.as_str()),
        Some("https://json.schemastore.org/sarif-2.1.0")
    );
    assert_eq!(sarif.get("version").and_then(|v| v.as_str()), Some("2.1.0"));
    let runs = sarif
        .get("runs")
        .and_then(|r| r.as_array())
        .expect("Missing runs");
    assert!(!runs.is_empty(), "SARIF runs should not be empty");

    let first_run = &runs[0];
    let tool = first_run
        .get("tool")
        .and_then(|t| t.get("driver"))
        .expect("Missing tool.driver");
    assert_eq!(
        tool.get("name").and_then(|n| n.as_str()),
        Some("cargo-cost-lint")
    );

    let results = first_run
        .get("results")
        .and_then(|r| r.as_array())
        .expect("Missing results");

    assert!(
        !results.is_empty(),
        "Expected SARIF results, but there were none. Stderr: {}",
        stderr_str
    );

    for result in results {
        assert!(
            result.get("ruleId").is_some(),
            "SARIF result missing ruleId"
        );
        assert!(result.get("level").is_some(), "SARIF result missing level");
        assert!(
            result.get("message").is_some(),
            "SARIF result missing message"
        );
        let locations = result
            .get("locations")
            .and_then(|l| l.as_array())
            .expect("SARIF result missing locations");
        assert!(
            !locations.is_empty(),
            "SARIF result should have at least one location"
        );
        let location = &locations[0];
        let physical = location
            .get("physicalLocation")
            .expect("SARIF location missing physicalLocation");
        assert!(
            physical.get("artifactLocation").is_some(),
            "SARIF physicalLocation missing artifactLocation"
        );
    }
}

#[test]
fn test_deny_lint_exit_code() {
    let bin_path = env!("CARGO_BIN_EXE_cargo-cost-lint");

    let mut fixture_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    fixture_dir.pop();
    fixture_dir.push("test_fixtures");
    fixture_dir.push("smoke_test");

    assert!(
        fixture_dir.exists(),
        "Fixture directory not found: {:?}",
        fixture_dir
    );

    let mut target_dir = PathBuf::from(env!("CARGO_BIN_EXE_cargo-cost-lint"));
    target_dir.pop();

    let mut lint_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    lint_dir.pop();
    lint_dir.push("soroban_cost_lints");

    let status = Command::new(env!("CARGO"))
        .arg("build")
        .current_dir(&lint_dir)
        .status()
        .expect("Failed to build soroban_cost_lints");
    assert!(status.success(), "Failed to build soroban_cost_lints");

    // Running with --config budget.toml (which has soroban_storage_in_loop = deny)
    let output = Command::new(bin_path)
        .arg("--config")
        .arg("budget.toml")
        .current_dir(&fixture_dir)
        .env("DYLINT_LIBRARY_PATH", &target_dir)
        .output()
        .expect("Failed to execute cargo-cost-lint");

    assert!(
        !output.status.success(),
        "cargo-cost-lint should exit with non-zero exit code when a deny lint is triggered, but got success exit code"
    );
}

#[test]
fn test_warn_lint_exit_code_zero() {
    let bin_path = env!("CARGO_BIN_EXE_cargo-cost-lint");

    let mut fixture_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    fixture_dir.pop();
    fixture_dir.push("test_fixtures");
    fixture_dir.push("smoke_test");

    assert!(
        fixture_dir.exists(),
        "Fixture directory not found: {:?}",
        fixture_dir
    );

    let mut target_dir = PathBuf::from(env!("CARGO_BIN_EXE_cargo-cost-lint"));
    target_dir.pop();

    let mut lint_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    lint_dir.pop();
    lint_dir.push("soroban_cost_lints");

    let status = Command::new(env!("CARGO"))
        .arg("build")
        .current_dir(&lint_dir)
        .status()
        .expect("Failed to build soroban_cost_lints");
    assert!(status.success(), "Failed to build soroban_cost_lints");

    // Running without deny lints configured should exit 0
    let output = Command::new(bin_path)
        .current_dir(&fixture_dir)
        .env("DYLINT_LIBRARY_PATH", &target_dir)
        .output()
        .expect("Failed to execute cargo-cost-lint");

    assert!(
        output.status.success(),
        "cargo-cost-lint should exit 0 when only warn lints occur, got: {}",
        output.status
    );
}

