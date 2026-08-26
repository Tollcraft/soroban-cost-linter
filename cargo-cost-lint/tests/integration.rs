use std::env;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn test_list_lints_json() {
    let bin_path = env!("CARGO_BIN_EXE_cargo-cost-lint");

    let output = Command::new(bin_path)
        .arg("--list-lints")
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to execute cargo-cost-lint");

    assert!(
        output.status.success(),
        "cargo-cost-lint --list-lints failed"
    );

    let stdout_str = String::from_utf8(output.stdout).expect("Stdout is not valid UTF-8");
    let inventory: serde_json::Value =
        serde_json::from_str(&stdout_str).expect("Output is not valid JSON");

    assert_eq!(inventory["version"], "1.0");
    assert!(inventory["schema"].is_string());

    let lints = inventory["lints"]
        .as_array()
        .expect("lints is not an array");
    assert!(!lints.is_empty(), "lints array should not be empty");

    let names: Vec<&str> = lints
        .iter()
        .map(|lint| lint["name"].as_str().expect("name is not a string"))
        .collect();

    let expected = [
        "soroban_storage_in_loop",
        "loop_invariant_storage_access",
        "unbounded_input_loop",
        "redundant_env_clone",
        "unnecessary_host_function_call",
        "host_in_loop",
        "symbol_new_for_short_literal",
        "unnecessary_string_to_bytes",
        "storage_write_without_read",
        "inefficient_bytes_concat",
        "map_insert_in_loop",
        "bytes_append_in_loop",
        "signature_verification_in_loop",
        "storage_key_construction_in_loop",
        "vec_where_slice_could_be_used",
    ];
    for name in &expected {
        assert!(
            names.contains(name),
            "Expected lint '{}' to be in inventory",
            name
        );
    }

    for lint in lints {
        assert!(lint.get("name").is_some(), "lint entry missing 'name'");
        assert!(
            lint.get("default_level").is_some(),
            "lint entry missing 'default_level'"
        );
        assert!(
            lint.get("description").is_some(),
            "lint entry missing 'description'"
        );
        assert!(
            lint.get("category").is_some(),
            "lint entry missing 'category'"
        );
        assert!(
            lint.get("documentation_url").is_some(),
            "lint entry missing 'documentation_url'"
        );
    }
}

#[test]
fn test_list_lints_text() {
    let bin_path = env!("CARGO_BIN_EXE_cargo-cost-lint");

    let output = Command::new(bin_path)
        .arg("--list-lints")
        .output()
        .expect("Failed to execute cargo-cost-lint");

    assert!(
        output.status.success(),
        "cargo-cost-lint --list-lints failed"
    );

    let stdout_str = String::from_utf8(output.stdout).expect("Stdout is not valid UTF-8");
    assert!(stdout_str.contains("Lint inventory (version 1.0):"));
    assert!(stdout_str.contains("soroban_storage_in_loop"));
    assert!(stdout_str.contains("redundant_env_clone"));
    assert!(stdout_str.contains("unnecessary_host_function_call"));
    assert!(stdout_str.contains("host_in_loop"));
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
    target_dir.pop(); // Remove the binary name
    if target_dir.ends_with("deps") {
        target_dir.pop(); // Pop "deps" to reach the profile directory (e.g., target/debug)
    }

    // Build the soroban_cost_lints cdylib first from its own directory so it picks up .cargo/config.toml
    let mut lint_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    lint_dir.pop();
    lint_dir.push("soroban_cost_lints");

    let status = Command::new(env!("CARGO"))
        .arg("build")
        .arg("--target-dir")
        .arg(&target_dir)
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
    let mut found_redundant_storage_read = false;
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
        if json["name"] == "soroban_redundant_storage_read" {
            found_redundant_storage_read = true;
        }
    }

    assert!(
        found_storage_in_loop,
        "Expected to find 'soroban_storage_in_loop' lint, but it was not present"
    );
    assert!(
        found_redundant_storage_read,
        "Expected to find 'soroban_redundant_storage_read' lint, but it was not present"
    );
}

#[test]
fn test_shared_budget_toml_parsing() {
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    let bin_path = env!("CARGO_BIN_EXE_cargo-cost-lint");

    // 1. Both sections
    let dir = tempdir().unwrap();
    let budget_path = dir.path().join("budget.toml");
    let mut f = File::create(&budget_path).unwrap();
    f.write_all(b"network = \"testnet\"\n[margin]\ncpu_margin = 1.50\n[lints]\nsoroban_storage_in_loop = \"warn\"\n").unwrap();

    let output = Command::new(bin_path)
        .arg("--list-lints")
        .current_dir(dir.path())
        .output()
        .expect("Failed to execute cargo-cost-lint");

    assert!(
        output.status.success(),
        "cargo-cost-lint failed with mixed budget.toml"
    );
    let stderr_str = String::from_utf8(output.stderr).unwrap();
    // It should ignore unknown sections without warning or error about them
    assert!(
        !stderr_str.contains("Error: Failed to parse"),
        "Should not fail to parse"
    );

    // 2. Only budget-assert sections
    let dir2 = tempdir().unwrap();
    let budget_path2 = dir2.path().join("budget.toml");
    let mut f2 = File::create(&budget_path2).unwrap();
    f2.write_all(b"network = \"testnet\"\n[margin]\ncpu_margin = 1.50\n[scenarios.workflow]\npackage=\"pkg\"\n").unwrap();

    let output2 = Command::new(bin_path)
        .arg("--list-lints")
        .current_dir(dir2.path())
        .output()
        .expect("Failed to execute cargo-cost-lint");

    assert!(
        output2.status.success(),
        "cargo-cost-lint failed with only budget-assert sections"
    );
    let stderr_str2 = String::from_utf8(output2.stderr).unwrap();
    assert!(
        !stderr_str2.contains("Error: Failed to parse"),
        "Should not fail to parse without lints section"
    );
}
