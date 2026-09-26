//! Linter performance benchmark.
//!
//! Measures end-to-end wall-clock time for running `cargo-cost-lint` against
//! every contract in `tests/corpus/contracts/`, compares the results against a
//! recorded JSON baseline, and exits non-zero when a regression is detected.
//!
//! # Running
//!
//! ```text
//! cargo bench --bench linter_performance --package cargo-cost-lint
//! ```
//!
//! # Blessing a new baseline
//!
//! When a performance change is intentional, regenerate the baseline with:
//!
//! ```text
//! BLESS_BENCH=1 cargo bench --bench linter_performance --package cargo-cost-lint
//! ```
//!
//! This overwrites `cargo-cost-lint/benches/benchmark_baseline.json`.
//!
//! # Environment knobs
//!
//! | Variable                  | Default | Description                                              |
//! |---------------------------|---------|----------------------------------------------------------|
//! | `LINTER_BENCH_ITERATIONS` | `3`     | Number of timed runs per contract; median is recorded.   |
//! | `BENCHMARK_THRESHOLD`     | `25.0`  | Regression threshold as a percentage over baseline.      |
//! | `BLESS_BENCH`             | unset   | Set to any value to write a new baseline instead of comparing. |
//! | `BLESS_BENCHMARK`         | unset   | Alias for `BLESS_BENCH`.                                 |
//! | `BLESS`                   | unset   | Alias for `BLESS_BENCH`.                                 |
//! | `DYLINT_LIBRARY_PATH`     | auto    | Override the path where Dylint looks for lint libraries. |

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{exit, Command, Stdio};
use std::time::{Duration, Instant};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Default number of timed iterations per contract when
/// `LINTER_BENCH_ITERATIONS` is not set.
const DEFAULT_ITERATIONS: usize = 3;

/// Default regression threshold (percentage above baseline) when
/// `BENCHMARK_THRESHOLD` is not set and no baseline file is present.
const DEFAULT_THRESHOLD_PERCENT: f64 = 25.0;

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

/// Persisted baseline file written/read by this benchmark.
///
/// The file is stored at `cargo-cost-lint/benches/benchmark_baseline.json`.
/// Increment `version` when the schema changes in a backward-incompatible way.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkBaseline {
    /// Schema version; currently always `1`.
    pub version: u32,
    /// The percentage threshold that was active when the baseline was recorded.
    /// Used as the default threshold on subsequent runs unless overridden by
    /// `BENCHMARK_THRESHOLD`.
    pub threshold_percent: f64,
    /// Human-readable description of how to update this file.
    pub description: String,
    /// Per-contract median durations in milliseconds, keyed by contract
    /// directory name (e.g. `"symbol_key_enum_storage"`).
    pub contracts: BTreeMap<String, f64>,
    /// Sum of all per-contract median durations (ms).  Used for a single
    /// aggregate regression check in addition to the per-contract checks.
    pub total_median_ms: f64,
}

// ---------------------------------------------------------------------------
// Path helpers
// ---------------------------------------------------------------------------

/// Returns the workspace root (the directory that contains the top-level
/// `Cargo.toml`).
///
/// Derived from `CARGO_MANIFEST_DIR` (the `cargo-cost-lint` package directory)
/// by ascending one level.  Panics if the package is not inside a workspace.
fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("cargo-cost-lint must be located in the workspace")
        .to_path_buf()
}

/// Returns the path to the JSON baseline file.
fn baseline_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("benches")
        .join("benchmark_baseline.json")
}

// ---------------------------------------------------------------------------
// Contract discovery
// ---------------------------------------------------------------------------

/// Collects and sorts all corpus contract paths from
/// `tests/corpus/contracts/`.
///
/// A directory is considered a contract if it contains a `Cargo.toml` at its
/// top level.  The list is sorted so benchmark output is deterministic across
/// runs and platforms.
///
/// Panics if the contracts directory cannot be read, or if no contracts are
/// found (which would indicate a broken workspace checkout).
fn corpus_contracts() -> Vec<PathBuf> {
    let contracts_dir = workspace_root().join("tests/corpus/contracts");
    let mut contracts: Vec<PathBuf> = fs::read_dir(&contracts_dir)
        .unwrap_or_else(|error| panic!("failed to read {:?}: {error}", contracts_dir))
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.join("Cargo.toml").is_file())
        .collect();

    contracts.sort();
    assert!(
        !contracts.is_empty(),
        "no benchmark contracts found in {:?}",
        contracts_dir
    );
    contracts
}

// ---------------------------------------------------------------------------
// Binary / library location helpers
// ---------------------------------------------------------------------------

/// Returns the Cargo target directory that contains the benchmark executable.
///
/// Derived by ascending two levels from the current executable path
/// (`target/<profile>/deps/<bench>` → `target/<profile>` → `target`).
/// Panics if the executable is not located below a standard Cargo target tree.
fn profile_target_dir() -> PathBuf {
    let executable = env::current_exe().expect("failed to locate benchmark executable");
    executable
        .parent()
        .and_then(Path::parent)
        .expect("benchmark executable is not located below a Cargo target directory")
        .to_path_buf()
}

/// Returns the expected path of the `cargo-cost-lint` binary inside
/// `target_dir`.
fn linter_path(target_dir: &Path) -> PathBuf {
    let executable_name = if cfg!(windows) {
        "cargo-cost-lint.exe"
    } else {
        "cargo-cost-lint"
    };
    target_dir.join(executable_name)
}

/// Returns the path to the built `cargo-cost-lint` binary, building it first
/// if it does not already exist in `target_dir`.
///
/// The `--release` flag is forwarded when `target_dir` ends with `"release"`,
/// so the benchmark always measures the same optimization level as the binary
/// under test.
fn ensure_linter_built(target_dir: &Path) -> PathBuf {
    let linter = linter_path(target_dir);
    if linter.is_file() {
        return linter;
    }

    let mut command = Command::new(env::var_os("CARGO").unwrap_or_else(|| "cargo".into()));
    command
        .arg("build")
        .arg("--bin")
        .arg("cargo-cost-lint")
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .stdout(Stdio::null())
        .stderr(Stdio::inherit());

    if target_dir.ends_with("release") {
        command.arg("--release");
    }

    let status = command
        .status()
        .expect("failed to build cargo-cost-lint before benchmarking");
    assert!(
        status.success(),
        "failed to build cargo-cost-lint before benchmarking"
    );
    assert!(
        linter.is_file(),
        "cargo-cost-lint was not produced at {:?}",
        linter
    );
    linter
}

/// Ensures the `soroban_cost_lints` Dylint library is built so the linter can
/// load it during benchmarking.
///
/// Skipped when `DYLINT_LIBRARY_PATH` is already set in the environment,
/// because the caller has explicitly provided a pre-built library path and we
/// must not overwrite it.
fn ensure_lint_library(target_dir: &Path) {
    if env::var_os("DYLINT_LIBRARY_PATH").is_some() {
        return;
    }

    let lint_dir = workspace_root().join("soroban_cost_lints");
    let mut command = Command::new(env::var_os("CARGO").unwrap_or_else(|| "cargo".into()));
    command
        .arg("build")
        .current_dir(lint_dir)
        .stdout(Stdio::null())
        .stderr(Stdio::inherit());

    if target_dir.ends_with("release") {
        command.arg("--release");
    }

    let status = command
        .status()
        .expect("failed to build soroban_cost_lints before benchmarking");
    assert!(
        status.success(),
        "failed to build soroban_cost_lints before benchmarking"
    );
}

// ---------------------------------------------------------------------------
// Environment-variable configuration helpers
// ---------------------------------------------------------------------------

/// Returns the number of timed iterations to run per contract.
///
/// Reads `LINTER_BENCH_ITERATIONS`; falls back to [`DEFAULT_ITERATIONS`].
/// Values that parse as zero are treated as absent (i.e. the default is used)
/// because zero iterations would produce no samples to take the median of.
fn iteration_count() -> usize {
    env::var("LINTER_BENCH_ITERATIONS")
        .ok()
        .and_then(|value| value.parse().ok())
        .filter(|iterations| *iterations > 0)
        .unwrap_or(DEFAULT_ITERATIONS)
}

/// Returns the regression threshold as a percentage over the baseline.
///
/// Reads `BENCHMARK_THRESHOLD`; falls back to [`DEFAULT_THRESHOLD_PERCENT`].
/// Note: if a baseline file is loaded, its embedded threshold takes precedence
/// over this default unless `BENCHMARK_THRESHOLD` is explicitly set.
fn threshold_percent() -> f64 {
    env::var("BENCHMARK_THRESHOLD")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(DEFAULT_THRESHOLD_PERCENT)
}

// ---------------------------------------------------------------------------
// Timing helpers
// ---------------------------------------------------------------------------

/// Runs the linter against a single `contract` directory and returns the
/// wall-clock duration of that invocation.
///
/// `DYLINT_LIBRARY_PATH` is forwarded to the child process so it can locate
/// the compiled lint library.  If the variable is already set in the parent
/// environment that value is used; otherwise `target_dir` is passed so the
/// freshly built library is found automatically.
///
/// Panics when the linter process cannot be spawned or exits with a non-zero
/// status, which would indicate a broken environment rather than a performance
/// regression.
fn run_linter(linter: &Path, contract: &Path, target_dir: &Path) -> Duration {
    let started = Instant::now();
    let output = Command::new(linter)
        .arg("--format")
        .arg("json")
        .current_dir(contract)
        .env(
            "DYLINT_LIBRARY_PATH",
            env::var_os("DYLINT_LIBRARY_PATH")
                .unwrap_or_else(|| target_dir.as_os_str().to_os_string()),
        )
        .output()
        .unwrap_or_else(|error| panic!("failed to execute {:?}: {error}", linter));

    assert!(
        output.status.success(),
        "linter failed for {:?}:\n{}",
        contract.file_name().unwrap_or_default(),
        String::from_utf8_lossy(&output.stderr)
    );
    started.elapsed()
}

/// Computes the value at the given `percentile` (0–100) from a **sorted**
/// slice of `Duration` samples.
///
/// Uses ceiling-division for the index so that e.g. the 50th percentile of a
/// 3-element slice returns the middle element (index 1) rather than the lower
/// one (index 0).
fn percentile(sorted_samples: &[Duration], percentile: usize) -> Duration {
    let index = ((sorted_samples.len() - 1) * percentile).div_ceil(100);
    sorted_samples[index]
}

// ---------------------------------------------------------------------------
// Baseline I/O helpers
// ---------------------------------------------------------------------------

/// Writes `baseline` to the JSON file at [`baseline_path()`].
///
/// Called only when `BLESS_BENCH` (or its aliases) is set.
fn write_baseline(baseline: &BenchmarkBaseline) {
    let json =
        serde_json::to_string_pretty(baseline).expect("Failed to serialize benchmark baseline");
    fs::write(baseline_path(), json).expect("Failed to write benchmark_baseline.json");
    eprintln!(
        "\nRecorded new benchmark baseline ({:.2} ms total) to {:?}",
        baseline.total_median_ms,
        baseline_path()
    );
}

/// Loads and deserialises the baseline from [`baseline_path()`].
///
/// Returns `None` when the file does not exist — callers should handle the
/// missing-baseline case gracefully (typically by printing a warning and
/// skipping the regression check).
fn load_baseline() -> Option<BenchmarkBaseline> {
    let path = baseline_path();
    if !path.exists() {
        return None;
    }
    let content = fs::read_to_string(&path).expect("Failed to read benchmark_baseline.json");
    let baseline: BenchmarkBaseline =
        serde_json::from_str(&content).expect("Failed to parse benchmark_baseline.json");
    Some(baseline)
}

// ---------------------------------------------------------------------------
// Benchmark measurement
// ---------------------------------------------------------------------------

/// Runs the linter `iterations` times against every contract in `contracts`,
/// and returns a map of contract name → median duration in milliseconds plus
/// the sum of all medians.
fn measure_contracts(
    contracts: &[PathBuf],
    linter: &Path,
    target_dir: &Path,
    iterations: usize,
) -> (BTreeMap<String, f64>, f64) {
    let mut results: BTreeMap<String, f64> = BTreeMap::new();
    let mut total_median_ms = 0.0_f64;

    for contract in contracts {
        let mut samples: Vec<Duration> = (0..iterations)
            .map(|_| run_linter(linter, contract, target_dir))
            .collect();
        samples.sort_unstable();

        let median_ms = percentile(&samples, 50).as_secs_f64() * 1000.0;
        let name = contract
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();

        results.insert(name, median_ms);
        total_median_ms += median_ms;
    }

    (results, total_median_ms)
}

// ---------------------------------------------------------------------------
// Regression reporting
// ---------------------------------------------------------------------------

/// Compares `current` results against a `baseline` and prints a formatted
/// table to stderr.
///
/// Returns `true` when at least one contract or the aggregate total has
/// regressed beyond `threshold_pct`.
fn report_regression(
    current: &BTreeMap<String, f64>,
    total_current_ms: f64,
    baseline: &BenchmarkBaseline,
    threshold_pct: f64,
) -> bool {
    eprintln!("\n=== Linter Performance Benchmark vs Baseline ===");
    eprintln!(
        "{:<35} | {:>12} | {:>12} | {:>14}",
        "Contract", "Baseline(ms)", "Current(ms)", "Delta"
    );
    eprintln!("{}", "-".repeat(78));

    let mut has_regression = false;

    for (name, current_ms) in current {
        let baseline_ms = baseline.contracts.get(name).copied().unwrap_or(0.0);
        let delta_ms = current_ms - baseline_ms;
        let delta_pct = if baseline_ms > 0.0 {
            (delta_ms / baseline_ms) * 100.0
        } else {
            0.0
        };

        let status_str = if delta_pct > threshold_pct {
            has_regression = true;
            "[REGRESSED]"
        } else {
            "[OK]"
        };

        eprintln!(
            "{:<35} | {:>12.2} | {:>12.2} | {:>+8.2} ms ({:>+6.2}%) {}",
            name, baseline_ms, current_ms, delta_ms, delta_pct, status_str
        );
    }

    let total_delta_ms = total_current_ms - baseline.total_median_ms;
    let total_delta_pct = if baseline.total_median_ms > 0.0 {
        (total_delta_ms / baseline.total_median_ms) * 100.0
    } else {
        0.0
    };

    eprintln!("{}", "=".repeat(78));
    eprintln!(
        "TOTAL BENCHMARK: Baseline={:.2} ms | Current={:.2} ms | Delta={:+.2} ms ({:+.2}%)",
        baseline.total_median_ms, total_current_ms, total_delta_ms, total_delta_pct
    );
    eprintln!("REGRESSION THRESHOLD: {:.2}%", threshold_pct);

    // The aggregate total is also checked independently of per-contract
    // results so a small-but-widespread slowdown (below per-contract threshold)
    // is still caught when it accumulates into a larger total regression.
    if total_delta_pct > threshold_pct {
        has_regression = true;
    }

    has_regression
}

/// Prints the failure message and exits the process with code 1.
fn fail_with_regression(
    threshold_pct: f64,
    baseline_ms: f64,
    current_ms: f64,
    delta_ms: f64,
    delta_pct: f64,
) -> ! {
    eprintln!(
        "\nFAIL: Linter performance regressed beyond the {:.2}% threshold!\n\
         Before: {:.2} ms, After: {:.2} ms, Delta: {:+.2} ms ({:+.2}%).\n\
         If this slowdown is expected and accepted, update the baseline using:\n\
           BLESS_BENCH=1 cargo bench --bench linter_performance --package cargo-cost-lint\n\
         and commit the updated cargo-cost-lint/benches/benchmark_baseline.json.",
        threshold_pct, baseline_ms, current_ms, delta_ms, delta_pct
    );
    exit(1);
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

fn main() {
    linter_performance();
}

/// Top-level benchmark driver.
///
/// Orchestrates the full benchmark lifecycle:
/// 1. Locate/build the linter binary and lint library.
/// 2. Discover corpus contracts.
/// 3. Time the linter against each contract for `iterations` runs.
/// 4. Either write a new baseline (bless mode) or compare against the existing
///    one and exit non-zero on regression.
fn linter_performance() {
    let target_dir = profile_target_dir();
    let linter = ensure_linter_built(&target_dir);
    ensure_lint_library(&target_dir);

    let iterations = iteration_count();
    let threshold = threshold_percent();

    // Bless mode is activated by any of three environment variable aliases so
    // CI scripts can use whichever name is most readable in context.
    let is_bless = env::var("BLESS_BENCH").is_ok()
        || env::var("BLESS_BENCHMARK").is_ok()
        || env::var("BLESS").is_ok();

    let contracts = corpus_contracts();
    eprintln!(
        "Benchmarking {} corpus contracts for {} iteration(s)...",
        contracts.len(),
        iterations
    );

    let (current_results, total_median_ms) =
        measure_contracts(&contracts, &linter, &target_dir, iterations);

    if is_bless {
        let new_baseline = BenchmarkBaseline {
            version: 1,
            threshold_percent: threshold,
            description: "Linter performance baseline (median durations in milliseconds).\n\
                 Update deliberately by running: BLESS_BENCH=1 cargo bench --bench linter_performance --package cargo-cost-lint".to_string(),
            contracts: current_results,
            total_median_ms,
        };
        write_baseline(&new_baseline);
        return;
    }

    // Without a baseline file we cannot compare; warn and exit successfully so
    // the absence of a baseline does not fail CI on a fresh checkout.
    let baseline = match load_baseline() {
        Some(b) => b,
        None => {
            eprintln!(
                "\nWarning: Benchmark baseline file not found at {:?}.\n\
                 Generating baseline data. Run with BLESS_BENCH=1 to record it.",
                baseline_path()
            );
            return;
        }
    };

    // Prefer the threshold embedded in the baseline unless the caller has
    // explicitly overridden it via BENCHMARK_THRESHOLD.
    let effective_threshold = if env::var("BENCHMARK_THRESHOLD").is_ok() {
        threshold
    } else {
        baseline.threshold_percent
    };

    let has_regression = report_regression(
        &current_results,
        total_median_ms,
        &baseline,
        effective_threshold,
    );

    if has_regression {
        let total_delta_ms = total_median_ms - baseline.total_median_ms;
        let total_delta_pct = if baseline.total_median_ms > 0.0 {
            (total_delta_ms / baseline.total_median_ms) * 100.0
        } else {
            0.0
        };
        fail_with_regression(
            effective_threshold,
            baseline.total_median_ms,
            total_median_ms,
            total_delta_ms,
            total_delta_pct,
        );
    } else {
        eprintln!("\nSUCCESS: Linter performance is within the acceptable threshold.");
    }
}
