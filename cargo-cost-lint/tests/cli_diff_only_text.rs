//! `--diff-only` must filter findings in every output format, including the
//! default text mode without a `.lintignore` (which used to take the
//! unfiltered raw-echo branch), while a plain run with no flags keeps echoing
//! cargo's output unchanged.
//!
//! Each test builds a scratch git repository holding a small Soroban contract
//! with the same anti-pattern in two modules, `src/alpha.rs` and
//! `src/beta.rs`, then commits a change on top of a fake `origin/main` so
//! `--diff-only` resolves a known set of changed files. (The module names
//! share no suffix on purpose: changed-file matching also accepts a filename
//! suffix, so e.g. `changed.rs` would match `unchanged.rs`.)

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::Mutex;

// These tests share the Dylint driver cache and fixture target directory.
// Concurrent cargo-dylint invocations can replace the cached driver while
// another test is executing it (ETXTBSY on Linux).
static FIXTURE_LOCK: Mutex<()> = Mutex::new(());

const CARGO_TOML: &str = r#"[package]
name = "diff-only-fixture"
version = "0.1.0"
edition = "2021"
publish = false

[dependencies]
soroban-sdk = "26"

# Keep the scratch package out of any enclosing workspace.
[workspace]
"#;

const LIB_RS: &str = "#![no_std]\npub mod alpha;\npub mod beta;\n";

// Warn instead of the default deny so the build succeeds and every finding
// is reported for both modules.
const BUDGET_TOML: &str = "[lints]\nsoroban_storage_in_loop = \"warn\"\n";

fn module_source(name: &str) -> String {
    format!(
        "use soroban_sdk::Env;\n\
         \n\
         pub fn {name}_storage_in_loop(env: Env) {{\n    \
             for i in 0..10u32 {{\n        \
                 env.storage().instance().set(&1u32, &i);\n    \
             }}\n\
         }}\n"
    )
}

fn workspace_dir() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop();
    path
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

/// One target directory shared by every scratch repo, so the Soroban SDK is
/// compiled once rather than once per test.
fn fixture_target_dir() -> PathBuf {
    workspace_dir()
        .join("target")
        .join("diff-only-fixture-target")
}

fn git(dir: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args([
            "-c",
            "user.name=cost-lint-test",
            "-c",
            "user.email=cost-lint-test@example.invalid",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .current_dir(dir)
        .output()
        .expect("failed to run git");
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Scratch repository whose `origin/main` is the initial commit and whose
/// `HEAD` adds one commit touching only `changed` (a repo-relative path).
fn scratch_repo(changed: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("failed to create temp dir");
    let root = dir.path();
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(root.join("Cargo.toml"), CARGO_TOML).unwrap();
    fs::write(root.join("budget.toml"), BUDGET_TOML).unwrap();
    fs::write(root.join("README.md"), "scratch fixture\n").unwrap();
    fs::write(root.join("src/lib.rs"), LIB_RS).unwrap();
    fs::write(root.join("src/alpha.rs"), module_source("alpha")).unwrap();
    fs::write(root.join("src/beta.rs"), module_source("beta")).unwrap();

    git(root, &["init", "-q"]);
    git(root, &["add", "-A"]);
    git(root, &["commit", "-q", "-m", "baseline"]);
    git(root, &["update-ref", "refs/remotes/origin/main", "HEAD"]);

    let mut contents = fs::read_to_string(root.join(changed)).unwrap();
    contents.push_str("// changed on the feature branch\n");
    fs::write(root.join(changed), contents).unwrap();
    git(root, &["add", changed]);
    git(root, &["commit", "-q", "-m", "change"]);
    dir
}

/// Applies the environment every run in this file shares.
fn fixture_env(cmd: &mut Command, lib_dir: &Path) {
    cmd.env("DYLINT_LIBRARY_PATH", lib_dir)
        .env("CARGO_TARGET_DIR", fixture_target_dir())
        .env_remove("DYLINT_RUSTFLAGS")
        .env_remove("NO_COLOR")
        .env_remove("CARGO_TERM_COLOR");
}

fn cost_lint(dir: &Path, lib_dir: &Path, args: &[&str]) -> Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_cargo-cost-lint"));
    cmd.args(args).current_dir(dir);
    fixture_env(&mut cmd, lib_dir);
    cmd.output().expect("failed to execute cargo-cost-lint")
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).expect("output is not valid UTF-8")
}

#[test]
fn diff_only_text_mode_reports_only_the_changed_file() {
    let _guard = FIXTURE_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let lib_dir = build_lint_library();
    let repo = scratch_repo("src/alpha.rs");

    let output = cost_lint(repo.path(), &lib_dir, &["--diff-only"]);
    let stderr = text(&output.stderr);

    assert!(
        stderr.contains("--diff-only: linting 1 changed file(s)"),
        "stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("alpha.rs"),
        "the changed file's findings are missing, stderr:\n{stderr}"
    );
    assert!(
        !stderr.contains("beta.rs"),
        "--diff-only reported an unchanged file in text mode, stderr:\n{stderr}"
    );
}

#[test]
fn diff_only_text_mode_with_unchanged_only_diff_reports_nothing() {
    let _guard = FIXTURE_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let lib_dir = build_lint_library();
    let repo = scratch_repo("README.md");

    let output = cost_lint(repo.path(), &lib_dir, &["--diff-only"]);
    let stdout = text(&output.stdout);
    let stderr = text(&output.stderr);

    assert!(
        stderr.contains("--diff-only: linting 1 changed file(s)"),
        "stderr:\n{stderr}"
    );
    for file in ["alpha.rs", "beta.rs"] {
        assert!(
            !stderr.contains(file) && !stdout.contains(file),
            "no linted file changed, but {file} was reported:\n{stdout}\n{stderr}"
        );
    }
}

#[test]
fn diff_only_machine_formats_filter_identically() {
    let _guard = FIXTURE_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let lib_dir = build_lint_library();
    let repo = scratch_repo("src/alpha.rs");

    for format in ["json", "github", "sarif"] {
        let args = ["--diff-only", "--format", format];
        let output = cost_lint(repo.path(), &lib_dir, &args);
        let stdout = text(&output.stdout);
        assert!(
            stdout.contains("alpha.rs"),
            "--format {format}: the changed file's findings are missing, stdout:\n{stdout}"
        );
        assert!(
            !stdout.contains("beta.rs"),
            "--format {format}: an unchanged file was reported, stdout:\n{stdout}"
        );
    }
}

/// Drops the parts of cargo's stderr that legitimately differ between two
/// identical runs: the elapsed time on the `Finished` line, and lock-wait
/// notices caused by tests sharing a target directory.
fn normalize_cargo_stderr(stderr: &str) -> Vec<String> {
    stderr
        .lines()
        .filter(|l| !l.contains("Blocking waiting for file lock"))
        .map(|l| {
            if l.trim_start().starts_with("Finished ") {
                l.split_once(" in ").map_or(l, |(head, _)| head).to_string()
            } else {
                l.to_string()
            }
        })
        .collect()
}

#[test]
fn plain_run_without_flags_keeps_raw_echo_output() {
    let _guard = FIXTURE_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let lib_dir = build_lint_library();
    let repo = scratch_repo("src/alpha.rs");

    // What `cargo-cost-lint` spawns for a plain text run with this
    // budget.toml, invoked directly.
    let direct = || {
        let mut cmd = Command::new("cargo");
        cmd.args(["dylint", "--lib", "soroban_cost_lints"])
            .current_dir(repo.path());
        fixture_env(&mut cmd, &lib_dir);
        cmd.env("DYLINT_RUSTFLAGS", "-W soroban_storage_in_loop");
        cmd.output().expect("failed to execute cargo dylint")
    };

    // Warm the build so both compared runs replay the same cached output.
    let warmup = direct();
    assert!(
        warmup.status.success(),
        "direct cargo dylint run failed, stderr:\n{}",
        text(&warmup.stderr)
    );

    let wrapped = cost_lint(repo.path(), &lib_dir, &[]);
    let reference = direct();

    let wrapped_stderr = text(&wrapped.stderr);
    let reference_stderr = text(&reference.stderr);

    assert_eq!(
        wrapped.stdout, reference.stdout,
        "stdout of a plain run must be cargo's stdout, byte for byte"
    );

    let mut wrapped_lines = wrapped_stderr.lines();
    let first = wrapped_lines.next().unwrap_or_default();
    assert!(
        first.starts_with("Using config: ") && first.ends_with("budget.toml"),
        "expected the config notice first, stderr:\n{wrapped_stderr}"
    );
    let echoed: Vec<&str> = wrapped_lines.collect();
    assert_eq!(
        normalize_cargo_stderr(&echoed.join("\n")),
        normalize_cargo_stderr(&reference_stderr),
        "a plain run must echo cargo's stderr unchanged"
    );

    // The raw echo is unfiltered: both modules' findings are there, which is
    // also what makes the `--diff-only` absence checks above meaningful.
    assert!(wrapped_stderr.contains("alpha.rs"), "{wrapped_stderr}");
    assert!(wrapped_stderr.contains("beta.rs"), "{wrapped_stderr}");
}
