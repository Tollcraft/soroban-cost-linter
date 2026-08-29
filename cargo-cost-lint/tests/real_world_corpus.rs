use std::collections::BTreeMap;
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

/// A single lint finding from the corpus.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct Finding {
    pub lint_name: String,
    pub file: String,
    pub line: usize,
    pub message: String,
}

/// Per-contract baseline entry.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BaselineEntry {
    pub total: usize,
    pub true_positives: Vec<Finding>,
    pub false_positives: Vec<Finding>,
}

/// Summary statistics for corpus baseline.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct BaselineSummary {
    pub total_findings: usize,
    pub total_true_positives: usize,
    pub total_false_positives: usize,
    pub false_positive_rate_percent: f64,
}

/// Full baseline file format.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Baseline {
    pub version: u32,
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<BaselineSummary>,
    pub contracts: BTreeMap<String, BaselineEntry>,
}

/// Lint names that should always be considered true-positive when they fire.
/// These are lints with no known false-positive patterns.
const ALWAYS_TP: &[&str] = &[
    "redundant_env_clone",
    "symbol_new_for_short_literal",
    "inefficient_bytes_concat",
    "soroban_inefficient_bytes_concat",
    "vec_index_in_loop",
    "map_insert_in_loop",
    "unnecessary_host_function_call",
    "unwrap_on_storage_get",
    "persistent_read_without_ttl_extension",
];

fn workspace_root() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p
}

fn corpus_dir() -> PathBuf {
    workspace_root().join("tests").join("corpus")
}

fn contracts_dir() -> PathBuf {
    corpus_dir().join("contracts")
}

fn baseline_path() -> PathBuf {
    corpus_dir().join("baseline.json")
}

/// Every corpus contract is an independent cargo workspace depending on the
/// same `soroban-sdk`, so left alone each one compiles that dependency tree
/// into its own `target/` — nine near-identical builds, none of them shared and
/// none of them covered by the CI cache, which only holds the root `target/`.
/// Pointing every contract build and its dylint pass at one directory compiles
/// the SDK once and leaves a single directory to cache.
fn shared_target_dir() -> PathBuf {
    corpus_dir().join(".shared-target")
}

/// Run `cargo-cost-lint --format json` in the given contract directory.
fn run_lints_on_contract(contract_dir: &Path) -> Vec<Finding> {
    let bin_path = env!("CARGO_BIN_EXE_cargo-cost-lint");
    let mut target_dir = PathBuf::from(bin_path);
    target_dir.pop();

    let output = Command::new(bin_path)
        .arg("--format")
        .arg("json")
        .current_dir(contract_dir)
        .env("DYLINT_LIBRARY_PATH", &target_dir)
        .env("CARGO_TARGET_DIR", shared_target_dir())
        .output()
        .expect("Failed to execute cargo-cost-lint");

    let stdout_str = String::from_utf8(output.stdout).expect("Stdout is not valid UTF-8");

    // A non-zero exit can mean cargo-cost-lint genuinely failed to run, or
    // that a `deny`-level lint fired and correctly failed the underlying
    // `cargo check` — findings were already streamed to stdout before the
    // process exited in that case. Only treat this as a hard failure when
    // there's nothing to show for it.
    if !output.status.success() && stdout_str.trim().is_empty() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!(
            "cargo-cost-lint failed on {:?}:\n{}",
            contract_dir.file_name().unwrap(),
            stderr
        );
    }

    let findings: Vec<Finding> = stdout_str
        .lines()
        .filter(|l| !l.is_empty())
        .map(|line| {
            let json: serde_json::Value =
                serde_json::from_str(line).expect("Output line is not valid JSON");
            Finding {
                lint_name: json["name"].as_str().unwrap_or("unknown").to_string(),
                file: json["file"].as_str().unwrap_or("unknown").to_string(),
                line: json["span"]["line_start"].as_u64().unwrap_or(0) as usize,
                message: json["message"].as_str().unwrap_or("").to_string(),
            }
        })
        .collect();

    findings
}

fn triage_findings(findings: &[Finding]) -> (Vec<Finding>, Vec<Finding>) {
    let mut tps = Vec::new();
    let mut fps = Vec::new();

    for f in findings {
        if ALWAYS_TP.contains(&f.lint_name.as_str()) {
            tps.push(f.clone());
        } else {
            fps.push(f.clone());
        }
    }

    (tps, fps)
}

fn build_soroban_cost_lints() {
    let binding = env::var("CARGO");
    let cargo = binding.as_deref().unwrap_or("cargo");

    let mut lint_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    lint_dir.pop();
    lint_dir.push("soroban_cost_lints");

    let status = Command::new(cargo)
        .arg("build")
        .current_dir(&lint_dir)
        .status()
        .expect("Failed to build soroban_cost_lints");

    assert!(status.success(), "Failed to build soroban_cost_lints");
}

fn collect_and_report(contract_dir: &Path) -> (String, Vec<Finding>) {
    let name = contract_dir
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string();
    let binding = env::var("CARGO");
    let cargo = binding.as_deref().unwrap_or("cargo");

    let build_status = Command::new(cargo)
        .arg("build")
        .current_dir(contract_dir)
        .env("CARGO_TARGET_DIR", shared_target_dir())
        .status()
        .expect("Failed to build contract");

    assert!(build_status.success(), "Failed to build contract {name}");

    let findings = run_lints_on_contract(contract_dir);

    (name, findings)
}

fn load_baseline() -> Baseline {
    let path = baseline_path();
    if path.exists() {
        let content = std::fs::read_to_string(&path).expect("Failed to read baseline.json");
        serde_json::from_str(&content).expect("Failed to parse baseline.json")
    } else {
        Baseline {
            version: 1,
            description: String::new(),
            summary: None,
            contracts: BTreeMap::new(),
        }
    }
}

fn compute_summary(all_findings: &BTreeMap<String, Vec<Finding>>) -> BaselineSummary {
    let mut total_tp = 0usize;
    let mut total_fp = 0usize;

    for findings in all_findings.values() {
        let (tps, fps) = triage_findings(findings);
        total_tp += tps.len();
        total_fp += fps.len();
    }

    let total = total_tp + total_fp;
    let fp_rate = if total > 0 {
        ((total_fp as f64 / total as f64) * 10000.0).round() / 100.0
    } else {
        0.0
    };

    BaselineSummary {
        total_findings: total,
        total_true_positives: total_tp,
        total_false_positives: total_fp,
        false_positive_rate_percent: fp_rate,
    }
}

fn print_corpus_summary(all_findings: &BTreeMap<String, Vec<Finding>>) {
    let mut total_tp = 0usize;
    let mut total_fp = 0usize;
    let mut per_lint_stats: BTreeMap<String, (usize, usize)> = BTreeMap::new();

    for findings in all_findings.values() {
        let (tps, fps) = triage_findings(findings);
        total_tp += tps.len();
        total_fp += fps.len();
        for tp in &tps {
            let entry = per_lint_stats.entry(tp.lint_name.clone()).or_insert((0, 0));
            entry.0 += 1;
        }
        for fp in &fps {
            let entry = per_lint_stats.entry(fp.lint_name.clone()).or_insert((0, 0));
            entry.1 += 1;
        }
    }

    let total = total_tp + total_fp;
    let fp_pct = if total > 0 {
        (total_fp as f64 / total as f64) * 100.0
    } else {
        0.0
    };
    let tp_pct = if total > 0 {
        (total_tp as f64 / total as f64) * 100.0
    } else {
        0.0
    };

    eprintln!("\n================================================================================");
    eprintln!("Corpus False-Positive vs True-Positive Summary:");
    eprintln!("  Total Findings:       {}", total);
    eprintln!("  True Positives (TP):  {} ({:.2}%)", total_tp, tp_pct);
    eprintln!("  False Positives (FP): {} ({:.2}%)", total_fp, fp_pct);
    eprintln!("--------------------------------------------------------------------------------");
    eprintln!(
        "{:<40} {:>6} {:>6} {:>8} {:>8}",
        "Lint Name", "TP", "FP", "Total", "% FP"
    );
    eprintln!(
        "{:<40} {:>6} {:>6} {:>8} {:>8}",
        "---------", "--", "--", "-----", "----"
    );
    for (lint_name, (tp, fp)) in &per_lint_stats {
        let lint_total = tp + fp;
        let lint_fp_pct = if lint_total > 0 {
            (*fp as f64 / lint_total as f64) * 100.0
        } else {
            0.0
        };
        eprintln!(
            "{:<40} {:>6} {:>6} {:>8} {:>7.1}%",
            lint_name, tp, fp, lint_total, lint_fp_pct
        );
    }
    eprintln!("================================================================================\n");
}

fn save_baseline(baseline: &Baseline) {
    let path = baseline_path();
    let content = serde_json::to_string_pretty(baseline).expect("Failed to serialize baseline");
    std::fs::write(&path, content).expect("Failed to write baseline.json");
}

#[test]
fn real_world_corpus_triage() {
    build_soroban_cost_lints();
    let bless = env::var("BLESS").is_ok();
    let contracts = contracts_dir();
    assert!(
        contracts.exists(),
        "Corpus contracts directory not found: {:?}",
        contracts
    );

    let mut all_findings: BTreeMap<String, Vec<Finding>> = BTreeMap::new();
    let mut grand_total = 0usize;

    let mut entries: Vec<_> = std::fs::read_dir(&contracts)
        .expect("Failed to read contracts directory")
        .filter_map(|e| e.ok())
        .filter(|e| e.path().join("Cargo.toml").exists())
        .collect();
    entries.sort_by_key(|e| e.file_name());

    for entry in &entries {
        let (name, findings) = collect_and_report(&entry.path());
        eprintln!("\n=== {}: {} findings ===", name, findings.len());
        for f in &findings {
            eprintln!("  {}:{} — {} — {}", f.file, f.line, f.lint_name, f.message);
        }
        all_findings.insert(name.clone(), findings);
        grand_total += all_findings.get(&name).map_or(0, |v| v.len());
    }

    eprintln!(
        "\n=== Grand total: {grand_total} findings across {} contracts ===",
        entries.len()
    );

    print_corpus_summary(&all_findings);

    let baseline = load_baseline();

    if bless {
        let summary = compute_summary(&all_findings);
        let mut new_baseline = Baseline {
            version: 1,
            description: format!(
                "Baseline lint findings for real-world corpus. Generated on {} with {} findings ({} TP, {} FP, {:.2}% FP rate) across {} contracts.\nRegenerate by running: BLESS=1 cargo test --test real_world_corpus --workspace",
                now_stamp(),
                grand_total,
                summary.total_true_positives,
                summary.total_false_positives,
                summary.false_positive_rate_percent,
                entries.len()
            ),
            summary: Some(summary),
            contracts: BTreeMap::new(),
        };

        for (name, findings) in &all_findings {
            let (tps, fps) = triage_findings(findings);
            new_baseline.contracts.insert(
                name.clone(),
                BaselineEntry {
                    total: findings.len(),
                    true_positives: tps,
                    false_positives: fps,
                },
            );
        }

        save_baseline(&new_baseline);
        eprintln!("Baseline written to {:?}", baseline_path());
    } else {
        let mut total_fp_increase = 0usize;
        let mut any_failure = false;

        for (name, findings) in &all_findings {
            let baseline_entry = baseline.contracts.get(name);
            let current_fps = triage_findings(findings).1.len();

            let baseline_fps = baseline_entry.map(|e| e.false_positives.len()).unwrap_or(0);

            if current_fps > baseline_fps {
                let increase = current_fps - baseline_fps;
                total_fp_increase += increase;
                any_failure = true;
                eprintln!(
                    "FAIL: {name} now has {current_fps} FPs (baseline: {baseline_fps}, +{increase})"
                );
            } else {
                eprintln!("OK:   {name} has {current_fps} FPs (baseline: {baseline_fps})");
            }
        }

        if any_failure {
            panic!(
                "\n\nFalse-positive count increased by {total_fp_increase} across corpus.\n\
                 If these are legitimate new findings, re-bless by running:\n\
                    BLESS=1 cargo test --test real_world_corpus --workspace\n\
                 Then commit the updated baseline.json"
            );
        }
    }
}

fn now_stamp() -> String {
    let d = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap();
    format!("t={}", d.as_secs())
}
