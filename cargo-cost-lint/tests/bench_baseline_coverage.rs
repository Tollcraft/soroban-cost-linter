//! Verification of benchmark baseline coverage across corpus contracts.
//!
//! Asserts that every contract in `tests/corpus/contracts/` has an entry
//! in `cargo-cost-lint/benches/benchmark_baseline.json` and that the stored
//! `total_median_ms` equals the sum of individual contract medians.

use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct BenchmarkBaseline {
    version: u32,
    threshold_percent: f64,
    description: String,
    contracts: BTreeMap<String, f64>,
    total_median_ms: f64,
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("cargo-cost-lint has a parent directory")
        .to_path_buf()
}

/// Discovers every directory under `tests/corpus/contracts/` that has a `Cargo.toml`.
fn discover_corpus_contracts() -> BTreeSet<String> {
    let corpus_dir = workspace_root().join("tests/corpus/contracts");
    let mut contracts = BTreeSet::new();

    let entries = fs::read_dir(&corpus_dir)
        .unwrap_or_else(|err| panic!("failed to read corpus directory {:?}: {err}", corpus_dir));

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir()
            && path.join("Cargo.toml").is_file()
            && let Some(name) = path.file_name().and_then(|n| n.to_str())
        {
            contracts.insert(name.to_string());
        }
    }

    contracts
}

/// Reads and deserializes `cargo-cost-lint/benches/benchmark_baseline.json`.
fn load_benchmark_baseline() -> BenchmarkBaseline {
    let baseline_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("benches/benchmark_baseline.json");
    let content = fs::read_to_string(&baseline_path)
        .unwrap_or_else(|err| panic!("failed to read {:?}: {err}", baseline_path));
    serde_json::from_str(&content)
        .unwrap_or_else(|err| panic!("failed to parse JSON from {:?}: {err}", baseline_path))
}

#[test]
fn test_bench_baseline_coverage_and_sum() {
    let corpus_contracts = discover_corpus_contracts();
    let baseline = load_benchmark_baseline();

    // 1. Assert that the stored total_median_ms equals the recomputed sum of individual entries.
    let recomputed_sum: f64 = baseline.contracts.values().sum();
    let total_diff = (baseline.total_median_ms - recomputed_sum).abs();
    assert!(
        total_diff < 1e-4,
        "Stored total_median_ms ({}) does not equal recomputed sum ({:.2}) of contracts entries (diff = {})",
        baseline.total_median_ms,
        recomputed_sum,
        total_diff
    );

    // 2. Assert that the baseline contracts and corpus contracts key sets match.
    let baseline_keys: BTreeSet<String> = baseline.contracts.keys().cloned().collect();
    let missing_from_baseline: Vec<_> = corpus_contracts.difference(&baseline_keys).collect();
    let extra_in_baseline: Vec<_> = baseline_keys.difference(&corpus_contracts).collect();

    assert!(
        missing_from_baseline.is_empty() && extra_in_baseline.is_empty(),
        "Benchmark baseline contract set does not match corpus contracts.\nMissing from baseline ({}/{}): {:?}\nExtra in baseline: {:?}",
        missing_from_baseline.len(),
        corpus_contracts.len(),
        missing_from_baseline,
        extra_in_baseline
    );
}
