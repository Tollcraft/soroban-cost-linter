//! Pins the config-resolution notices `cargo-cost-lint` writes to stderr.
//!
//! Matrix: (default, `--quiet`, `--verbose`) x (`budget.toml` present, absent)
//! x (no overrides, `--allow <lint>`), run against a copy of the
//! `test_fixtures/smoke_test` package. Only stderr is asserted on.
//!
//! The notices are all printed before `cargo dylint` is spawned, so each run
//! uses an empty `PATH`: the spawn fails fast and the tests need neither
//! `cargo-dylint` nor a build of the fixture's dependencies. `--no-cache`
//! keeps a cached result from short-circuiting the run before the
//! `[verbose] config:` line.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const ALLOWED_LINT: &str = "redundant_env_clone";
const MISSING_WARNING: &str = "Warning: budget.toml not found, using default lint levels.";
const VERBOSE_NO_CONFIG: &str = "[verbose] config: (none — using default lint levels)";

#[derive(Clone, Copy, Debug)]
enum Verbosity {
    Default,
    Quiet,
    Verbose,
}

fn smoke_test_fixture() -> PathBuf {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    dir.pop();
    dir.join("test_fixtures").join("smoke_test")
}

/// Copies the smoke_test package into a fresh directory, with or without its
/// `budget.toml`. The fixture's own `[workspace]` table stops config
/// discovery from walking above the copy.
fn fixture_copy(with_budget: bool) -> tempfile::TempDir {
    let src = smoke_test_fixture();
    let dir = tempfile::tempdir().expect("failed to create temp dir");
    fs::create_dir_all(dir.path().join("src")).unwrap();
    fs::copy(src.join("Cargo.toml"), dir.path().join("Cargo.toml")).unwrap();
    fs::copy(src.join("src/lib.rs"), dir.path().join("src/lib.rs")).unwrap();
    if with_budget {
        fs::copy(src.join("budget.toml"), dir.path().join("budget.toml")).unwrap();
    }
    dir
}

fn run_stderr(cwd: &Path, verbosity: Verbosity, allow: bool) -> String {
    let empty_path = tempfile::tempdir().expect("failed to create temp dir");
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_cargo-cost-lint"));
    cmd.arg("--no-cache")
        .current_dir(cwd)
        .env("PATH", empty_path.path())
        .env_remove("DYLINT_RUSTFLAGS");
    match verbosity {
        Verbosity::Default => {}
        Verbosity::Quiet => {
            cmd.arg("--quiet");
        }
        Verbosity::Verbose => {
            cmd.arg("--verbose");
        }
    }
    if allow {
        cmd.arg("--allow").arg(ALLOWED_LINT);
    }
    let output = cmd.output().expect("failed to execute cargo-cost-lint");
    String::from_utf8(output.stderr).expect("stderr is not valid UTF-8")
}

/// Every stderr line that reports how the config was resolved, including the
/// retired `Using config file:` and `No budget.toml found,` forms.
fn config_notices(stderr: &str) -> Vec<String> {
    stderr
        .lines()
        .filter(|l| {
            l.starts_with("Using config")
                || l.starts_with("[verbose] config:")
                || l.contains("budget.toml not found")
                || l.starts_with("No budget.toml found")
        })
        .map(str::to_string)
        .collect()
}

fn notices_for(verbosity: Verbosity, with_budget: bool, allow: bool) -> (String, Vec<String>) {
    let dir = fixture_copy(with_budget);
    let stderr = run_stderr(dir.path(), verbosity, allow);
    let notices = config_notices(&stderr);
    (stderr, notices)
}

fn is_resolved_path_line(line: &str, prefix: &str) -> bool {
    line.starts_with(prefix) && line.trim_end().ends_with("budget.toml")
}

#[test]
fn config_notice_matrix() {
    for verbosity in [Verbosity::Default, Verbosity::Quiet, Verbosity::Verbose] {
        for with_budget in [true, false] {
            for allow in [false, true] {
                let (stderr, notices) = notices_for(verbosity, with_budget, allow);
                let case = format!(
                    "verbosity={verbosity:?} budget.toml={with_budget} --allow={allow}\n\
                     stderr:\n{stderr}"
                );
                match (verbosity, with_budget) {
                    (Verbosity::Quiet, _) => {
                        assert!(notices.is_empty(), "expected no notice for {case}");
                    }
                    (Verbosity::Default, true) => {
                        assert_eq!(notices.len(), 1, "{case}");
                        assert!(
                            is_resolved_path_line(&notices[0], "Using config: "),
                            "{case}"
                        );
                    }
                    (Verbosity::Default, false) if allow => {
                        assert!(notices.is_empty(), "expected no notice for {case}");
                    }
                    (Verbosity::Default, false) => {
                        assert_eq!(notices, [MISSING_WARNING], "{case}");
                    }
                    (Verbosity::Verbose, true) => {
                        assert_eq!(notices.len(), 1, "{case}");
                        assert!(
                            is_resolved_path_line(&notices[0], "[verbose] config: "),
                            "{case}"
                        );
                    }
                    (Verbosity::Verbose, false) => {
                        assert_eq!(notices, [VERBOSE_NO_CONFIG], "{case}");
                    }
                }
            }
        }
    }
}

#[test]
fn verbose_without_budget_toml_prints_exactly_one_no_config_line() {
    let (stderr, notices) = notices_for(Verbosity::Verbose, false, false);
    assert_eq!(notices, [VERBOSE_NO_CONFIG], "stderr:\n{stderr}");
    assert!(
        !stderr.contains("No budget.toml found"),
        "the duplicate no-config line is back:\n{stderr}"
    );
}

#[test]
fn verbose_with_budget_toml_prints_exactly_one_resolved_path_line() {
    let (stderr, notices) = notices_for(Verbosity::Verbose, true, false);
    assert_eq!(notices.len(), 1, "stderr:\n{stderr}");
    assert!(
        is_resolved_path_line(&notices[0], "[verbose] config: "),
        "stderr:\n{stderr}"
    );
    assert!(
        !stderr.contains("Using config file:"),
        "the duplicate resolved-path line is back:\n{stderr}"
    );
}

#[test]
fn default_verbosity_with_budget_toml_prints_using_config() {
    let (stderr, notices) = notices_for(Verbosity::Default, true, false);
    assert_eq!(notices.len(), 1, "stderr:\n{stderr}");
    assert!(
        is_resolved_path_line(&notices[0], "Using config: "),
        "stderr:\n{stderr}"
    );
}

#[test]
fn default_verbosity_without_budget_toml_prints_missing_config_warning() {
    let (stderr, notices) = notices_for(Verbosity::Default, false, false);
    assert_eq!(notices, [MISSING_WARNING], "stderr:\n{stderr}");
}

#[test]
fn quiet_prints_no_config_notice_with_or_without_budget_toml() {
    for with_budget in [true, false] {
        let (stderr, notices) = notices_for(Verbosity::Quiet, with_budget, false);
        assert!(
            notices.is_empty(),
            "budget.toml={with_budget}, stderr:\n{stderr}"
        );
    }
}

#[test]
fn allow_override_without_budget_toml_suppresses_missing_config_warning() {
    let (stderr, notices) = notices_for(Verbosity::Default, false, true);
    assert!(
        notices.is_empty(),
        "--allow should suppress the missing-config warning, stderr:\n{stderr}"
    );
}
