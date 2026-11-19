//! End-to-end proof for issue #810: the per-lint findings summary that
//! `README.md` documents is actually emitted by the real `cargo-cost-lint`
//! binary. Issue #401 shipped `print_findings_summary` and #433 audited the
//! `#![allow(dead_code)]` that hid it, but the function had no caller in the
//! run path — this binary is that missing call site.
//!
//! The summary is wired into the *filtered* text path (`--diff-only` or a
//! present `.lintignore`), which is the text branch that already reconstructs
//! each diagnostic from cargo's JSON message stream and can therefore append a
//! roll-up. A plain text run takes the raw-echo branch and is pinned, byte for
//! byte, to cargo's own output by `tests/cli_diff_only_text.rs` (an invariant
//! owned by #938), so it is deliberately not annotated here — see that issue's
//! test and the PR body. `.lintignore` is discovered from the run's cwd, so we
//! trigger the path with a throwaway copy of the `smoke_test` crate carrying a
//! `.lintignore` that matches nothing (every finding survives; the flag only
//! flips the run into the streaming mode).
//!
//! `--no-cache` is mandatory: the cache key is the *linted crate's* source
//! hash plus toolchain/version/format, never this crate's formatter code, so a
//! warm entry would replay stale output instead of exercising the wiring.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

/// Source fixture crate (deny `soroban_storage_in_loop`, warn others).
const FIXTURE: &str = "test_fixtures/smoke_test";
/// The lint `smoke_test` pins to `deny`; it must appear in the summary.
const DENY_LINT: &str = "soroban_storage_in_loop";

fn workspace_root() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop(); // …/cargo-cost-lint → workspace root
    p
}

/// Build the `soroban_cost_lints` cdylib and return the dir for
/// `DYLINT_LIBRARY_PATH`, mirroring `format_contracts.rs` / `integration.rs`.
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

/// Copy the fixture crate (source, manifest, budget) into a fresh temp dir and
/// drop in a `.lintignore` whose single pattern matches none of its files.
///
/// A copy (not the tracked dir) keeps this run isolated from the concurrent
/// `format_contracts.rs` tests, which lint the tracked `smoke_test` without a
/// `.lintignore`; writing a `.lintignore` into the tracked dir would flip those
/// runs into the streaming branch and change their output.
fn staged_fixture_with_lintignore() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("failed to create temp dir");
    let src = workspace_root().join(FIXTURE);
    let dst = dir.path();

    let copy = |rel: &str| {
        std::fs::copy(src.join(rel), dst.join(rel))
            .unwrap_or_else(|e| panic!("cannot copy {rel} into staging dir: {e}"));
    };
    std::fs::create_dir(dst.join("src")).expect("create staged src dir");
    copy("Cargo.toml");
    copy("budget.toml");
    copy("src/lib.rs");
    // A pattern that matches no file present in the crate: discovery succeeds
    // (so the run is a *filtered* text run) but every finding survives.
    std::fs::write(
        dst.join(".lintignore"),
        "this_pattern_matches_no_source_file.rs\n",
    )
    .expect("failed to write .lintignore");
    dir
}

/// One `target` dir shared by every run in this test binary: the workspace's
/// own target, which already holds a warm Soroban SDK build. Pointing each
/// throwaway staging crate at it makes the SDK link instant instead of
/// recompiling the SDK once per temp dir; cargo's target lock serialises the
/// concurrent fixture builds.
fn shared_target_dir() -> &'static Path {
    static TARGET: OnceLock<PathBuf> = OnceLock::new();
    TARGET.get_or_init(|| workspace_root().join("target"))
}

/// Run the real binary in `staged` and return (stdout, stderr). Non-zero exit
/// is expected (the fixture denies a lint), so status is not asserted.
fn run(staged: &Path, lib_dir: &Path, args: &[&str]) -> (String, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_cargo-cost-lint"))
        .args(args)
        .arg("--no-cache")
        .current_dir(staged)
        .env("DYLINT_LIBRARY_PATH", lib_dir)
        .env("CARGO_TARGET_DIR", shared_target_dir())
        .output()
        .expect("Failed to execute cargo-cost-lint");
    (
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    )
}

/// The headline proof: a filtered text run closes with the documented roll-up,
/// naming the denied lint — the exact behaviour #810 says "the CLI never
/// prints". This fails if the `print_findings_summary` call site is removed
/// from `main.rs` (mutation-proven in the PR body).
#[test]
fn filtered_text_run_ends_with_findings_summary() {
    let lib_dir = ensure_lint_library_built();
    let staged = staged_fixture_with_lintignore();

    let (stdout, stderr) = run(staged.path(), &lib_dir, &["--format", "text"]);

    // Diagnostics render on stderr in the text path; the summary follows.
    assert!(
        stderr.contains("Found") && stderr.contains("cost lint finding"),
        "no findings-summary header in stderr.\n--- stderr ---\n{stderr}\n--- stdout ---\n{stdout}"
    );
    assert!(
        stderr.contains("By lint name:"),
        "summary is missing the by-lint breakdown.\n--- stderr ---\n{stderr}"
    );
    assert!(
        stderr.contains(DENY_LINT),
        "summary does not name the denied lint {DENY_LINT:?}.\n--- stderr ---\n{stderr}"
    );
    assert!(
        stderr.contains("By severity:"),
        "summary is missing the by-severity breakdown.\n--- stderr ---\n{stderr}"
    );
}

/// `--quiet` suppresses the summary entirely (README/#810 done-criterion).
#[test]
fn quiet_text_run_prints_no_summary() {
    let lib_dir = ensure_lint_library_built();
    let staged = staged_fixture_with_lintignore();

    let (_stdout, stderr) = run(staged.path(), &lib_dir, &["--format", "text", "--quiet"]);

    assert!(
        !stderr.contains("By lint name:"),
        "--quiet must not print the findings summary.\n--- stderr ---\n{stderr}"
    );
}

/// Machine formats stay pure: `--format json` stdout is line-delimited JSON
/// with no summary prose.
#[test]
fn json_stdout_stays_machine_only() {
    let lib_dir = ensure_lint_library_built();
    let staged = staged_fixture_with_lintignore();

    let (stdout, _stderr) = run(staged.path(), &lib_dir, &["--format", "json"]);

    assert!(
        !stdout.contains("By lint name:") && !stdout.contains("cost lint finding"),
        "json stdout leaked a summary.\n--- stdout ---\n{stdout}"
    );
    for line in stdout.lines().filter(|l| !l.trim().is_empty()) {
        serde_json::from_str::<serde_json::Value>(line)
            .unwrap_or_else(|e| panic!("json stdout line is not JSON: {e}\n{line}"));
    }
}
