use std::env;
use std::path::PathBuf;
use std::process::Command;

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
    let runs = sarif.get("runs").and_then(|r| r.as_array()).expect("Missing runs");
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
        assert!(
            result.get("level").is_some(),
            "SARIF result missing level"
        );
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