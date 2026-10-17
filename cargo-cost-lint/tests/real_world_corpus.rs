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

/// Full baseline file format.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Baseline {
    pub version: u32,
    pub description: String,
    pub contracts: BTreeMap<String, BaselineEntry>,
}

/// Lint names that should always be considered true-positive when they fire.
/// These are lints with no known false-positive patterns.
const ALWAYS_TP: &[&str] = &[
    "redundant_env_clone",
    "symbol_new_for_short_literal",
    "inefficient_bytes_concat",
    "map_insert_in_loop",
    "unnecessary_host_function_call",
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
        .output()
        .expect("Failed to execute cargo-cost-lint");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!(
            "cargo-cost-lint failed on {:?}:\n{}",
            contract_dir.file_name().unwrap(),
            stderr
        );
    }

    let stdout_str = String::from_utf8(output.stdout).expect("Stdout is not valid UTF-8");
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
            contracts: BTreeMap::new(),
        }
    }
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

    let baseline = load_baseline();

    if bless {
        let mut new_baseline = Baseline {
            version: 1,
            description: format!(
                "Baseline lint findings for real-world corpus. Generated on {} with {} findings across {} contracts.\nRegenerate by running: BLESS=1 cargo test --test real_world_corpus --workspace",
                now_stamp(),
                grand_total,
                entries.len()
            ),
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
