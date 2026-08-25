use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const DEFAULT_ITERATIONS: usize = 3;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("cargo-cost-lint must be located in the workspace")
        .to_path_buf()
}

fn corpus_contracts() -> Vec<PathBuf> {
    let contracts_dir = workspace_root().join("tests/corpus/contracts");
    let mut contracts: Vec<PathBuf> = std::fs::read_dir(&contracts_dir)
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

    eprintln!(
        "Benchmarking {} corpus contracts for {} iteration(s)",
        corpus_contracts().len(),
        iterations
    );

    for contract in corpus_contracts() {
        let mut samples: Vec<Duration> = (0..iterations)
            .map(|_| run_linter(&linter, &contract, &target_dir))
            .collect();
        samples.sort_unstable();

        let total: Duration = samples.iter().copied().sum();
        let average = total / samples.len() as u32;
        let median = percentile(&samples, 50);
        let p95 = percentile(&samples, 95);
        let name = contract
            .file_name()
            .map(|name| name.to_string_lossy())
            .unwrap_or_default();

        eprintln!(
            "linter_performance/{name}: min={:?} median={:?} avg={:?} p95={:?} max={:?}",
            samples[0],
            median,
            average,
            p95,
            samples[samples.len() - 1]
        );
    }
}
