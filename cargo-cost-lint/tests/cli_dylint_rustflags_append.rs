//! Pins how `cargo-cost-lint` combines an inherited `DYLINT_RUSTFLAGS` with
//! the lint levels from `budget.toml`: the inherited flags come first and the
//! tool's flags are appended after them, so rustc (last flag wins) still
//! applies the budget levels. Without lint flags the child inherits the
//! variable untouched, and `--verbose` must say so instead of `(empty)`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const PRE_EXISTING: &str = "-Wpre_existing_lint";
const BUDGET_ALLOW: &str = "[lints]\nsoroban_storage_in_loop = \"allow\"\n";

fn workspace_dir() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop();
    path
}

/// A copy of `test_fixtures/smoke_test` with `budget` as its `budget.toml`
/// (or no `budget.toml` at all). The fixture's own `[workspace]` table keeps
/// config discovery inside the copy.
fn fixture_copy(budget: Option<&str>) -> tempfile::TempDir {
    let src = workspace_dir().join("test_fixtures").join("smoke_test");
    let dir = tempfile::tempdir().expect("failed to create temp dir");
    fs::create_dir_all(dir.path().join("src")).unwrap();
    fs::copy(src.join("Cargo.toml"), dir.path().join("Cargo.toml")).unwrap();
    fs::copy(src.join("src/lib.rs"), dir.path().join("src/lib.rs")).unwrap();
    if let Some(budget) = budget {
        fs::write(dir.path().join("budget.toml"), budget).unwrap();
    }
    dir
}

fn cost_lint(cwd: &Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_cargo-cost-lint"));
    cmd.arg("--verbose")
        .arg("--no-cache")
        .current_dir(cwd)
        .env("DYLINT_RUSTFLAGS", PRE_EXISTING);
    cmd
}

/// Runs with an empty `PATH`: the `[verbose]` lines are printed before
/// `cargo dylint` is spawned, so the spawn can fail fast without `cargo`.
fn verbose_stderr_without_cargo(cwd: &Path) -> String {
    let empty_path = tempfile::tempdir().expect("failed to create temp dir");
    let output = cost_lint(cwd)
        .env("PATH", empty_path.path())
        .output()
        .expect("failed to execute cargo-cost-lint");
    String::from_utf8(output.stderr).expect("stderr is not valid UTF-8")
}

fn rustflags_line(stderr: &str) -> &str {
    stderr
        .lines()
        .find(|l| l.starts_with("[verbose] DYLINT_RUSTFLAGS: "))
        .unwrap_or_else(|| panic!("no DYLINT_RUSTFLAGS line in stderr:\n{stderr}"))
}

#[test]
fn verbose_rustflags_line_lists_inherited_flags_before_budget_flags() {
    let dir = fixture_copy(Some(BUDGET_ALLOW));
    let stderr = verbose_stderr_without_cargo(dir.path());
    assert_eq!(
        rustflags_line(&stderr),
        "[verbose] DYLINT_RUSTFLAGS: -Wpre_existing_lint -A soroban_storage_in_loop",
        "stderr:\n{stderr}"
    );
}

#[test]
fn verbose_rustflags_line_echoes_inherited_value_without_budget_toml() {
    let dir = fixture_copy(None);
    let stderr = verbose_stderr_without_cargo(dir.path());
    assert_eq!(
        rustflags_line(&stderr),
        "[verbose] DYLINT_RUSTFLAGS: -Wpre_existing_lint",
        "stderr:\n{stderr}"
    );
    assert!(
        !stderr.contains("DYLINT_RUSTFLAGS: (empty)"),
        "the inherited value was reported as empty:\n{stderr}"
    );
}

/// Builds the lint cdylib and returns the directory `DYLINT_LIBRARY_PATH`
/// must point at (same pattern as `integration.rs::test_json_output`).
fn build_lint_library() -> PathBuf {
    let lint_dir = workspace_dir().join("soroban_cost_lints");
    let status = Command::new(env!("CARGO"))
        .arg("build")
        .current_dir(lint_dir)
        .status()
        .expect("Failed to build soroban_cost_lints");
    assert!(status.success(), "Failed to build soroban_cost_lints");

    match std::env::var("CARGO_TARGET_DIR") {
        Ok(dir) => PathBuf::from(dir).join("debug"),
        Err(_) => workspace_dir().join("target").join("debug"),
    }
}

/// What the child rustc actually received, not just what `--verbose` claims:
/// the inherited `-Wpre_existing_lint` must reach rustc (it reports the
/// unknown lint), and the appended `-A soroban_storage_in_loop` from
/// `budget.toml` must still win over the deny-by-default level, so the run
/// succeeds. Overwriting `DYLINT_RUSTFLAGS` instead of appending drops the
/// unknown-lint warning and fails this test.
#[test]
fn child_receives_inherited_rustflags_with_budget_flags_appended() {
    let lib_dir = build_lint_library();
    let dir = fixture_copy(Some(BUDGET_ALLOW));

    let output: Output = cost_lint(dir.path())
        .env("DYLINT_LIBRARY_PATH", lib_dir)
        .output()
        .expect("failed to execute cargo-cost-lint");
    let stderr = String::from_utf8(output.stderr).expect("stderr is not valid UTF-8");

    assert_eq!(
        rustflags_line(&stderr),
        "[verbose] DYLINT_RUSTFLAGS: -Wpre_existing_lint -A soroban_storage_in_loop",
        "stderr:\n{stderr}"
    );

    // The `[verbose]` lines echo both flags, so only rustc's output counts.
    let rustc_output: Vec<&str> = stderr
        .lines()
        .filter(|l| !l.starts_with("[verbose]"))
        .collect();
    let reported_pre_existing_lint = rustc_output
        .iter()
        .any(|l| l.contains("unknown lint: `pre_existing_lint`"));
    let reported_storage_in_loop = rustc_output
        .iter()
        .any(|l| l.contains("soroban_storage_in_loop"));
    assert!(
        reported_pre_existing_lint,
        "the child did not inherit DYLINT_RUSTFLAGS={PRE_EXISTING}, stderr:\n{stderr}"
    );
    assert!(
        !reported_storage_in_loop,
        "budget.toml's allow did not win over the default deny, stderr:\n{stderr}"
    );
    assert!(
        output.status.success(),
        "with soroban_storage_in_loop allowed the run must succeed, stderr:\n{stderr}"
    );
}
