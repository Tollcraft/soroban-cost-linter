#![cfg(unix)]

use serde_json::{Value, json};
use std::env;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;
use tempfile::tempdir;

fn compiler_message(code: &str, level: &str, rendered: &str, file: &str) -> Value {
    json!({
        "reason": "compiler-message",
        "message": {
            "code": { "code": code },
            "level": level,
            "message": rendered.trim(),
            "rendered": rendered,
            "spans": [{
                "is_primary": true,
                "file_name": file,
                "line_start": 1,
                "line_end": 1,
                "column_start": 1,
                "column_end": 2
            }]
        }
    })
}

fn install_mock_cargo(dir: &Path) -> std::ffi::OsString {
    let mock_dir = dir.join("mock-bin");
    fs::create_dir_all(&mock_dir).unwrap();
    let cargo = mock_dir.join("cargo");
    fs::write(
        &cargo,
        "#!/bin/sh\nif [ \"$1\" = metadata ]; then exit 1; fi\ncat \"$MOCK_CARGO_OUTPUT\"\nexit \"$MOCK_CARGO_EXIT\"\n",
    )
    .unwrap();
    let mut permissions = fs::metadata(&cargo).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&cargo, permissions).unwrap();

    env::join_paths(
        std::iter::once(mock_dir).chain(env::split_paths(&env::var_os("PATH").unwrap_or_default())),
    )
    .unwrap()
}

#[test]
fn text_mode_preserves_non_cost_diagnostics_with_lintignore() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join(".lintignore"), "src/ignored.rs\n").unwrap();

    let diagnostics = [
        compiler_message(
            "soroban_storage_in_loop",
            "warning",
            "warning: ignored cost finding\n",
            "src/ignored.rs",
        ),
        compiler_message(
            "unused_variables",
            "warning",
            "warning: unused variable: `unused`\n",
            "src/main.rs",
        ),
        compiler_message(
            "E0308",
            "error",
            "error[E0308]: mismatched types\n",
            "src/main.rs",
        ),
    ];
    let output_path = dir.path().join("cargo-output.jsonl");
    let output = diagnostics
        .iter()
        .map(serde_json::to_string)
        .collect::<Result<Vec<_>, _>>()
        .unwrap()
        .join("\n");
    fs::write(&output_path, format!("{output}\n")).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_cargo-cost-lint"))
        .args(["--no-cache"])
        .current_dir(dir.path())
        .env("PATH", install_mock_cargo(dir.path()))
        .env("MOCK_CARGO_OUTPUT", &output_path)
        .env("MOCK_CARGO_EXIT", "1")
        .output()
        .expect("Failed to run cargo-cost-lint");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(1), "stderr: {stderr}");
    assert!(stderr.contains("unused variable"), "stderr: {stderr}");
    assert!(stderr.contains("error[E0308]"), "stderr: {stderr}");
    assert!(
        !stderr.contains("ignored cost finding"),
        "lintignore should continue suppressing cost findings: {stderr}"
    );
}
