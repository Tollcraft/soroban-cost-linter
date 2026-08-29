use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio, exit};
use std::time::{Duration, Instant};

const DEFAULT_ITERATIONS: usize = 3;
const DEFAULT_THRESHOLD_PERCENT: f64 = 25.0;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkBaseline {
    pub version: u32,
    pub threshold_percent: f64,
    pub description: String,
    pub contracts: BTreeMap<String, f64>,
    pub total_median_ms: f64,
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("cargo-cost-lint must be located in the workspace")
        .to_path_buf()
}

fn baseline_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("benches")
        .join("benchmark_baseline.json")
}

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

fn profile_target_dir() -> PathBuf {
    let executable = env::current_exe().expect("failed to locate benchmark executable");
    executable
        .parent()
        .and_then(Path::parent)
        .expect("benchmark executable is not located below a Cargo target directory")
        .to_path_buf()
}

fn linter_path(target_dir: &Path) -> PathBuf {
    let executable_name = if cfg!(windows) {
        "cargo-cost-lint.exe"
    } else {
        "cargo-cost-lint"
    };
    target_dir.join(executable_name)
}

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

fn iteration_count() -> usize {
    env::var("LINTER_BENCH_ITERATIONS")
        .ok()
        .and_then(|value| value.parse().ok())
        .filter(|iterations| *iterations > 0)
        .unwrap_or(DEFAULT_ITERATIONS)
}

fn threshold_percent() -> f64 {
    env::var("BENCHMARK_THRESHOLD")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(DEFAULT_THRESHOLD_PERCENT)
}

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

fn percentile(sorted_samples: &[Duration], percentile: usize) -> Duration {
    let index = ((sorted_samples.len() - 1) * percentile).div_ceil(100);
    sorted_samples[index]
}

fn main() {
    linter_performance();
}

fn linter_performance() {
    let target_dir = profile_target_dir();
    let linter = ensure_linter_built(&target_dir);
    ensure_lint_library(&target_dir);
    let iterations = iteration_count();
    let is_bless = env::var("BLESS_BENCH").is_ok()
        || env::var("BLESS_BENCHMARK").is_ok()
        || env::var("BLESS").is_ok();
    let threshold = threshold_percent();

    let contracts = corpus_contracts();
    eprintln!(
        "Benchmarking {} corpus contracts for {} iteration(s)...",
        contracts.len(),
        iterations
    );

    let mut current_results: BTreeMap<String, f64> = BTreeMap::new();
    let mut total_median_ms = 0.0;

    for contract in &contracts {
        let mut samples: Vec<Duration> = (0..iterations)
            .map(|_| run_linter(&linter, contract, &target_dir))
            .collect();
        samples.sort_unstable();

        let median = percentile(&samples, 50);
        let median_ms = median.as_secs_f64() * 1000.0;
        let name = contract
            .file_name()
            .map(|name| name.to_string_lossy())
            .unwrap_or_default()
            .to_string();

        current_results.insert(name.clone(), median_ms);
        total_median_ms += median_ms;
    }

    if is_bless {
        let new_baseline = BenchmarkBaseline {
            version: 1,
            threshold_percent: threshold,
            description: "Linter performance baseline (median durations in milliseconds).\n\
                 Update deliberately by running: BLESS_BENCH=1 cargo bench --bench linter_performance --package cargo-cost-lint".to_string(),
            contracts: current_results,
            total_median_ms,
        };

        let json = serde_json::to_string_pretty(&new_baseline)
            .expect("Failed to serialize benchmark baseline");
        fs::write(baseline_path(), json).expect("Failed to write benchmark_baseline.json");
        eprintln!(
            "\nRecorded new benchmark baseline ({:.2} ms total) to {:?}",
            total_median_ms,
            baseline_path()
        );
        return;
    }

    // Compare against recorded baseline if available
    let b_path = baseline_path();
    if !b_path.exists() {
        eprintln!(
            "\nWarning: Benchmark baseline file not found at {:?}.\n\
             Generating baseline data. Run with BLESS_BENCH=1 to record it.",
            b_path
        );
        return;
    }

    let baseline_content =
        fs::read_to_string(&b_path).expect("Failed to read benchmark_baseline.json");
    let baseline: BenchmarkBaseline =
        serde_json::from_str(&baseline_content).expect("Failed to parse benchmark_baseline.json");

    let effective_threshold = if env::var("BENCHMARK_THRESHOLD").is_ok() {
        threshold
    } else {
        baseline.threshold_percent
    };

    eprintln!("\n=== Linter Performance Benchmark vs Baseline ===");
    eprintln!(
        "{:<35} | {:>12} | {:>12} | {:>14}",
        "Contract", "Baseline(ms)", "Current(ms)", "Delta"
    );
    eprintln!("{}", "-".repeat(78));

    let mut has_regression = false;

    for (name, current_ms) in &current_results {
        let baseline_ms = baseline.contracts.get(name).copied().unwrap_or(0.0);
        let delta_ms = current_ms - baseline_ms;
        let delta_pct = if baseline_ms > 0.0 {
            (delta_ms / baseline_ms) * 100.0
        } else {
            0.0
        };

        let status_str = if delta_pct > effective_threshold {
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

    let total_delta_ms = total_median_ms - baseline.total_median_ms;
    let total_delta_pct = if baseline.total_median_ms > 0.0 {
        (total_delta_ms / baseline.total_median_ms) * 100.0
    } else {
        0.0
    };

    eprintln!("{}", "=".repeat(78));
    eprintln!(
        "TOTAL BENCHMARK: Baseline={:.2} ms | Current={:.2} ms | Delta={:+.2} ms ({:+.2}%)",
        baseline.total_median_ms, total_median_ms, total_delta_ms, total_delta_pct
    );
    eprintln!("REGRESSION THRESHOLD: {:.2}%", effective_threshold);

    if total_delta_pct > effective_threshold || has_regression {
        eprintln!(
            "\nFAIL: Linter performance regressed beyond the {:.2}% threshold!\n\
             Before: {:.2} ms, After: {:.2} ms, Delta: {:+.2} ms ({:+.2}%).\n\
             If this slowdown is expected and accepted, update the baseline using:\n\
               BLESS_BENCH=1 cargo bench --bench linter_performance --package cargo-cost-lint\n\
             and commit the updated cargo-cost-lint/benches/benchmark_baseline.json.",
            effective_threshold,
            baseline.total_median_ms,
            total_median_ms,
            total_delta_ms,
            total_delta_pct
        );
        exit(1);
    } else {
        eprintln!("\nSUCCESS: Linter performance is within the acceptable threshold.");
    }
}
