#![cfg(unix)]

use std::env;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;
use tempfile::tempdir;

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
        std::iter::once(mock_dir).chain(env::split_paths(
            &env::var_os("PATH").unwrap_or_default(),
        )),
    )
    .unwrap()
}

fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(dir)
        .status()
        .expect("Failed to run git");
    assert!(status.success(), "git {:?} failed", args);
}

#[test]
fn quiet_diff_only_has_no_stderr_for_changed_files() {
    let dir = tempdir().unwrap();
    git(dir.path(), &["init", "--quiet", "--initial-branch=main"]);
    git(dir.path(), &["config", "user.name", "Test Contributor"]);
    git(dir.path(), &["config", "user.email", "test@example.com"]);
    fs::write(dir.path().join("contract.rs"), "pub fn contract() {}\n").unwrap();
    git(dir.path(), &["add", "contract.rs"]);
    git(dir.path(), &["commit", "--quiet", "-m", "base"]);
    git(dir.path(), &["update-ref", "refs/remotes/origin/main", "HEAD"]);
    fs::write(
        dir.path().join("contract.rs"),
        "pub fn contract() { let _ = 1; }\n",
    )
    .unwrap();
    git(dir.path(), &["add", "contract.rs"]);
    git(dir.path(), &["commit", "--quiet", "-m", "change"]);

    let cargo_output = dir.path().join("cargo-output.jsonl");
    fs::write(&cargo_output, "").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_cargo-cost-lint"))
        .args(["--quiet", "--diff-only", "--no-cache"])
        .current_dir(dir.path())
        .env("PATH", install_mock_cargo(dir.path()))
        .env("MOCK_CARGO_OUTPUT", cargo_output)
        .env("MOCK_CARGO_EXIT", "0")
        .output()
        .expect("Failed to run cargo-cost-lint");

    assert!(output.status.success());
    assert!(
        output.stderr.is_empty(),
        "quiet diff-only should suppress notices, got: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn quiet_diff_only_still_reports_non_git_error() {
    let dir = tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_cargo-cost-lint"))
        .args(["--quiet", "--diff-only", "--no-cache"])
        .current_dir(dir.path())
        .output()
        .expect("Failed to run cargo-cost-lint");

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("error:"),
        "quiet mode must not suppress errors"
    );
}